/// IPC (Inter-Process Communication) module
/// Handles communication between the UI process and service daemon
/// Uses platform-appropriate IPC mechanisms:
/// - Windows: Named Pipes
/// - Unix-like: Unix Domain Sockets

use crate::service::{ ServiceMessage, ServiceStatus };
use serde_json;
use tokio::io::{ AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader };

#[cfg(target_os = "windows")]
use tokio::net::windows::named_pipe::{ NamedPipeClient, NamedPipeServer };

#[cfg(unix)]
use tokio::net::{ UnixListener, UnixStream };

/// IPC client for UI to communicate with service
pub struct IpcClient {
    #[cfg(target_os = "windows")]
    stream: Option<NamedPipeClient>,

    #[cfg(unix)]
    stream: Option<UnixStream>,
}

/// IPC server for service to accept connections from UI
pub struct IpcServer {
    #[cfg(target_os = "windows")]
    server: NamedPipeServer,

    #[cfg(unix)]
    listener: UnixListener,
}

impl IpcClient {
    /// Create a new IPC client
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        {
            use tokio::net::windows::named_pipe::ClientOptions;
            let stream = ClientOptions::new().open(r"\\.\pipe\mechvibes-dx-service")?;
            Ok(Self { stream: Some(stream) })
        }

        #[cfg(unix)]
        {
            let socket_path = Self::get_socket_path();
            let stream = UnixStream::connect(&socket_path).await?;
            Ok(Self { stream: Some(stream) })
        }
    }

    /// Send a message to the service
    pub async fn send_message(
        &mut self,
        message: &ServiceMessage
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json_data = serde_json::to_string(message)?;
        let data = format!("{}\n", json_data);

        #[cfg(target_os = "windows")]
        if let Some(ref mut stream) = self.stream {
            stream.write_all(data.as_bytes()).await?;
        }

        #[cfg(unix)]
        if let Some(ref mut stream) = self.stream {
            stream.write_all(data.as_bytes()).await?;
        }

        Ok(())
    }

    /// Receive a message from the service
    pub async fn receive_message(&mut self) -> Result<ServiceMessage, Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        if let Some(ref mut stream) = self.stream {
            let mut reader = AsyncBufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            let message: ServiceMessage = serde_json::from_str(&line.trim())?;
            return Ok(message);
        }

        #[cfg(unix)]
        if let Some(ref mut stream) = self.stream {
            let mut reader = AsyncBufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            let message: ServiceMessage = serde_json::from_str(&line.trim())?;
            return Ok(message);
        }

        Err("No active stream".into())
    }

    /// Get service status
    pub async fn get_status(&mut self) -> Result<ServiceStatus, Box<dyn std::error::Error>> {
        self.send_message(&ServiceMessage::GetStatus).await?;

        match self.receive_message().await? {
            ServiceMessage::StatusResponse(status) => Ok(status),
            ServiceMessage::Error(err) => Err(err.into()),
            _ => Err("Unexpected response".into()),
        }
    }

    #[cfg(unix)]
    fn get_socket_path() -> String {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            format!("{}/mechvibes-dx-service.sock", runtime_dir)
        } else {
            "/tmp/mechvibes-dx-service.sock".to_string()
        }
    }
}

impl IpcServer {
    /// Create a new IPC server
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        {
            use tokio::net::windows::named_pipe::ServerOptions;
            let server = ServerOptions::new()
                .first_pipe_instance(true)
                .create(r"\\.\pipe\mechvibes-dx-service")?;
            Ok(Self { server })
        }

        #[cfg(unix)]
        {
            let socket_path = Self::get_socket_path();

            // Remove existing socket file if it exists
            let _ = std::fs::remove_file(&socket_path);

            let listener = UnixListener::bind(&socket_path)?;

            // Set appropriate permissions for socket file
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&socket_path)?;
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o600); // Read/write for owner only
                std::fs::set_permissions(&socket_path, permissions)?;
            }

            Ok(Self { listener })
        }
    }

    /// Accept incoming connections and handle them
    pub async fn run<F>(&mut self, message_handler: F) -> Result<(), Box<dyn std::error::Error>>
        where F: Fn(ServiceMessage) -> ServiceMessage + Send + Sync + 'static + Clone
    {
        println!("🚀 IPC Server listening for connections...");
        #[cfg(target_os = "windows")]
        {
            loop {
                self.server.connect().await?;

                let handler = message_handler.clone();

                // Create a new server for the next connection
                use tokio::net::windows::named_pipe::ServerOptions;
                let new_server = ServerOptions::new().create(r"\\.\pipe\mechvibes-dx-service")?;
                let stream = std::mem::replace(&mut self.server, new_server);

                tokio::spawn(async move {
                    Self::handle_client_windows(stream, handler).await;
                });
            }
        }

        #[cfg(unix)]
        {
            loop {
                let (stream, _addr) = self.listener.accept().await?;
                let handler = message_handler.clone();

                tokio::spawn(async move {
                    Self::handle_client_unix(stream, handler).await;
                });
            }
        }
    }
    #[cfg(target_os = "windows")]
    async fn handle_client_windows<F>(stream: NamedPipeServer, handler: F)
        where F: Fn(ServiceMessage) -> ServiceMessage
    {
        use tokio::io::{ split, AsyncBufReadExt, AsyncWriteExt };

        let (read_half, mut write_half) = split(stream);
        let mut reader = AsyncBufReader::new(read_half);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    break;
                } // Connection closed
                Ok(_) => {
                    if let Ok(message) = serde_json::from_str::<ServiceMessage>(&line.trim()) {
                        let response = handler(message);
                        let response_json = serde_json::to_string(&response).unwrap_or_default();
                        let response_data = format!("{}\n", response_json);

                        if let Err(_) = write_half.write_all(response_data.as_bytes()).await {
                            break;
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
    #[cfg(unix)]
    async fn handle_client_unix<F>(stream: UnixStream, handler: F)
        where F: Fn(ServiceMessage) -> ServiceMessage
    {
        use tokio::io::{ split, AsyncBufReadExt, AsyncWriteExt };

        let (read_half, mut write_half) = split(stream);
        let mut reader = AsyncBufReader::new(read_half);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    break;
                } // Connection closed
                Ok(_) => {
                    if let Ok(message) = serde_json::from_str::<ServiceMessage>(&line.trim()) {
                        let response = handler(message);
                        let response_json = serde_json::to_string(&response).unwrap_or_default();
                        let response_data = format!("{}\n", response_json);

                        if let Err(_) = write_half.write_all(response_data.as_bytes()).await {
                            break;
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    #[cfg(unix)]
    fn get_socket_path() -> String {
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            format!("{}/mechvibes-dx-service.sock", runtime_dir)
        } else {
            "/tmp/mechvibes-dx-service.sock".to_string()
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let socket_path = Self::get_socket_path();
            let _ = std::fs::remove_file(socket_path);
        }
    }
}
