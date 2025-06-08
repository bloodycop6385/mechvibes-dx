/// Service Manager - High-level interface for managing the service
/// Handles installation, uninstallation, and communication with the service

use crate::service::{ ServiceConfig, ServiceMessage, ServiceStatus };
use crate::service::ipc::IpcClient;

pub struct ServiceManager {
    config: ServiceConfig,
    ipc_client: Option<IpcClient>,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new() -> Self {
        let mut config = ServiceConfig::default();

        // Set executable path to current binary
        if let Ok(exe_path) = std::env::current_exe() {
            config.executable_path = exe_path.to_string_lossy().to_string();
        }

        Self {
            config,
            ipc_client: None,
        }
    }

    /// Check if service is installed on the system
    pub fn is_service_installed(&self) -> bool {
        #[cfg(target_os = "windows")]
        return self.is_windows_service_installed();

        #[cfg(target_os = "linux")]
        return self.is_linux_service_installed();

        #[cfg(target_os = "macos")]
        return self.is_macos_service_installed();

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        false
    }

    /// Check if service is currently running
    pub async fn is_service_running(&mut self) -> bool {
        match self.get_service_status().await {
            Ok(status) => status.is_running,
            Err(_) => false,
        }
    }

    /// Install the service on the system
    pub async fn install_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_service_installed() {
            return Ok(()); // Already installed
        }

        #[cfg(target_os = "windows")]
        return self.install_windows_service().await;

        #[cfg(target_os = "linux")]
        return self.install_linux_service().await;

        #[cfg(target_os = "macos")]
        return self.install_macos_service().await;

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        Err("Service installation not supported on this platform".into())
    }

    /// Uninstall the service from the system
    pub async fn uninstall_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_service_installed() {
            return Ok(()); // Not installed
        }

        // Stop service first if running
        let _ = self.stop_service().await;

        #[cfg(target_os = "windows")]
        return self.uninstall_windows_service().await;

        #[cfg(target_os = "linux")]
        return self.uninstall_linux_service().await;

        #[cfg(target_os = "macos")]
        return self.uninstall_macos_service().await;

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        Err("Service uninstallation not supported on this platform".into())
    }

    /// Start the service
    pub async fn start_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        return self.start_windows_service().await;

        #[cfg(target_os = "linux")]
        return self.start_linux_service().await;

        #[cfg(target_os = "macos")]
        return self.start_macos_service().await;

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        Err("Service start not supported on this platform".into())
    }

    /// Stop the service
    pub async fn stop_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "windows")]
        return self.stop_windows_service().await;

        #[cfg(target_os = "linux")]
        return self.stop_linux_service().await;

        #[cfg(target_os = "macos")]
        return self.stop_macos_service().await;

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        Err("Service stop not supported on this platform".into())
    }

    /// Get service status via IPC
    pub async fn get_service_status(
        &mut self
    ) -> Result<ServiceStatus, Box<dyn std::error::Error>> {
        // Try to connect if not already connected
        if self.ipc_client.is_none() {
            match IpcClient::new().await {
                Ok(client) => {
                    self.ipc_client = Some(client);
                }
                Err(_) => {
                    // Service might not be running
                    return Ok(ServiceStatus {
                        is_running: false,
                        is_installed: self.is_service_installed(),
                        has_privileges: false,
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        platform: std::env::consts::OS.to_string(),
                    });
                }
            }
        }

        if let Some(ref mut client) = self.ipc_client {
            match client.get_status().await {
                Ok(status) => Ok(status),
                Err(_) => {
                    // Connection failed, reset client
                    self.ipc_client = None;
                    Ok(ServiceStatus {
                        is_running: false,
                        is_installed: self.is_service_installed(),
                        has_privileges: false,
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        platform: std::env::consts::OS.to_string(),
                    })
                }
            }
        } else {
            Ok(ServiceStatus {
                is_running: false,
                is_installed: self.is_service_installed(),
                has_privileges: false,
                version: env!("CARGO_PKG_VERSION").to_string(),
                platform: std::env::consts::OS.to_string(),
            })
        }
    }

    /// Send a message to the service
    pub async fn send_message(
        &mut self,
        message: ServiceMessage
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.ipc_client.is_none() {
            self.ipc_client = Some(IpcClient::new().await?);
        }

        if let Some(ref mut client) = self.ipc_client {
            client.send_message(&message).await?;
        }

        Ok(())
    }

    /// Get user-friendly status description
    pub async fn get_status_description(&mut self) -> String {
        match self.get_service_status().await {
            Ok(status) => {
                if status.is_running && status.has_privileges {
                    "Service running with elevated privileges".to_string()
                } else if status.is_running {
                    "Service running with standard privileges".to_string()
                } else if status.is_installed {
                    "Service installed but not running".to_string()
                } else {
                    "Service not installed".to_string()
                }
            }
            Err(_) => "Unable to determine service status".to_string(),
        }
    }
}

// Platform-specific implementations
#[cfg(target_os = "windows")]
impl ServiceManager {
    fn is_windows_service_installed(&self) -> bool {
        use std::process::Command;

        let output = Command::new("sc").args(&["query", &self.config.service_name]).output();

        match output {
            Ok(result) => result.status.success(),
            Err(_) => false,
        }
    }

    async fn install_windows_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;

        let service_path = format!("\"{}\" --service", self.config.executable_path);

        let output = Command::new("sc")
            .args(
                &[
                    "create",
                    &self.config.service_name,
                    "binPath=",
                    &service_path,
                    "DisplayName=",
                    &self.config.display_name,
                    "start=",
                    if self.config.auto_start { "auto" } else { "demand" },
                ]
            )
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to install Windows service: {}", error).into());
        }

        println!("✅ Windows service installed successfully");
        Ok(())
    }

    async fn uninstall_windows_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;

        let output = Command::new("sc").args(&["delete", &self.config.service_name]).output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to uninstall Windows service: {}", error).into());
        }

        println!("✅ Windows service uninstalled successfully");
        Ok(())
    }

    async fn start_windows_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;

        let output = Command::new("sc").args(&["start", &self.config.service_name]).output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to start Windows service: {}", error).into());
        }

        Ok(())
    }

    async fn stop_windows_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;

        let output = Command::new("sc").args(&["stop", &self.config.service_name]).output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to stop Windows service: {}", error).into());
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl ServiceManager {
    fn is_linux_service_installed(&self) -> bool {
        use std::path::Path;
        let service_file = format!(
            "/etc/systemd/system/{}.service",
            self.config.service_name.to_lowercase()
        );
        Path::new(&service_file).exists()
    }

    async fn install_linux_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let service_content = format!(
            r#"[Unit]
Description={}
After=graphical-session.target

[Service]
Type=simple
ExecStart={} --service
Restart=always
RestartSec=5
User=root

[Install]
WantedBy=multi-user.target
"#,
            self.config.description,
            self.config.executable_path
        );

        let service_file = format!(
            "/etc/systemd/system/{}.service",
            self.config.service_name.to_lowercase()
        );
        std::fs::write(&service_file, service_content)?;

        // Reload systemd
        std::process::Command::new("systemctl").args(&["daemon-reload"]).output()?;

        if self.config.auto_start {
            std::process::Command
                ::new("systemctl")
                .args(&["enable", &format!("{}.service", self.config.service_name.to_lowercase())])
                .output()?;
        }

        println!("✅ Linux service installed successfully");
        Ok(())
    }

    async fn uninstall_linux_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let service_name = format!("{}.service", self.config.service_name.to_lowercase());

        // Disable service
        std::process::Command::new("systemctl").args(&["disable", &service_name]).output()?;

        // Remove service file
        let service_file = format!("/etc/systemd/system/{}", service_name);
        std::fs::remove_file(&service_file)?;

        // Reload systemd
        std::process::Command::new("systemctl").args(&["daemon-reload"]).output()?;

        println!("✅ Linux service uninstalled successfully");
        Ok(())
    }

    async fn start_linux_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let service_name = format!("{}.service", self.config.service_name.to_lowercase());

        let output = std::process::Command
            ::new("systemctl")
            .args(&["start", &service_name])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to start Linux service: {}", error).into());
        }

        Ok(())
    }

    async fn stop_linux_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let service_name = format!("{}.service", self.config.service_name.to_lowercase());

        let output = std::process::Command
            ::new("systemctl")
            .args(&["stop", &service_name])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to stop Linux service: {}", error).into());
        }

        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl ServiceManager {
    fn is_macos_service_installed(&self) -> bool {
        use std::path::Path;
        let plist_path = format!(
            "/Library/LaunchDaemons/com.mechvibes.{}.plist",
            self.config.service_name.to_lowercase()
        );
        Path::new(&plist_path).exists()
    }

    async fn install_macos_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mechvibes.{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>--service</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
"#,
            self.config.service_name.to_lowercase(),
            self.config.executable_path
        );

        let plist_path = format!(
            "/Library/LaunchDaemons/com.mechvibes.{}.plist",
            self.config.service_name.to_lowercase()
        );
        std::fs::write(&plist_path, plist_content)?;

        if self.config.auto_start {
            std::process::Command::new("launchctl").args(&["load", &plist_path]).output()?;
        }

        println!("✅ macOS service installed successfully");
        Ok(())
    }

    async fn uninstall_macos_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let plist_path = format!(
            "/Library/LaunchDaemons/com.mechvibes.{}.plist",
            self.config.service_name.to_lowercase()
        );

        // Unload service
        std::process::Command::new("launchctl").args(&["unload", &plist_path]).output()?;

        // Remove plist file
        std::fs::remove_file(&plist_path)?;

        println!("✅ macOS service uninstalled successfully");
        Ok(())
    }

    async fn start_macos_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let plist_path = format!(
            "/Library/LaunchDaemons/com.mechvibes.{}.plist",
            self.config.service_name.to_lowercase()
        );

        let output = std::process::Command::new("launchctl").args(&["load", &plist_path]).output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to start macOS service: {}", error).into());
        }

        Ok(())
    }

    async fn stop_macos_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let plist_path = format!(
            "/Library/LaunchDaemons/com.mechvibes.{}.plist",
            self.config.service_name.to_lowercase()
        );

        let output = std::process::Command
            ::new("launchctl")
            .args(&["unload", &plist_path])
            .output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to stop macOS service: {}", error).into());
        }

        Ok(())
    }
}
