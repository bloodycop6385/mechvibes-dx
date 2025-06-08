/// Service Daemon - The main service process that runs with elevated privileges
/// This handles input capture and communicates with the UI via IPC

use crate::service::{ ServiceMessage, ServiceStatus };
use crate::service::ipc::IpcServer;
use crate::utils::admin;
use tokio::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{ AtomicBool, Ordering };
use rdev::{ listen, Button, Event, EventType, Key };
use std::collections::HashSet;
use std::sync::Mutex;
use std::thread;
use std::time::{ Duration, Instant };

pub struct ServiceDaemon {
    ipc_server: Option<IpcServer>,
    is_running: Arc<AtomicBool>,
    input_capture_active: Arc<AtomicBool>,
    event_sender: Option<mpsc::UnboundedSender<String>>,
}

// Maps a keyboard key to its standardized code
fn map_key_to_code(key: Key) -> &'static str {
    match key {
        // Number row
        Key::Num0 => "Digit0",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",

        // Letters
        Key::KeyA => "KeyA",
        Key::KeyB => "KeyB",
        Key::KeyC => "KeyC",
        Key::KeyD => "KeyD",
        Key::KeyE => "KeyE",
        Key::KeyF => "KeyF",
        Key::KeyG => "KeyG",
        Key::KeyH => "KeyH",
        Key::KeyI => "KeyI",
        Key::KeyJ => "KeyJ",
        Key::KeyK => "KeyK",
        Key::KeyL => "KeyL",
        Key::KeyM => "KeyM",
        Key::KeyN => "KeyN",
        Key::KeyO => "KeyO",
        Key::KeyP => "KeyP",
        Key::KeyQ => "KeyQ",
        Key::KeyR => "KeyR",
        Key::KeyS => "KeyS",
        Key::KeyT => "KeyT",
        Key::KeyU => "KeyU",
        Key::KeyV => "KeyV",
        Key::KeyW => "KeyW",
        Key::KeyX => "KeyX",
        Key::KeyY => "KeyY",
        Key::KeyZ => "KeyZ",

        // Special keys
        Key::Space => "Space",
        Key::Return => "Enter",
        Key::Tab => "Tab",
        Key::Escape => "Escape",
        Key::Backspace => "Backspace",
        Key::Delete => "Delete",
        Key::Insert => "Insert",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",

        // Arrow keys
        Key::UpArrow => "ArrowUp",
        Key::DownArrow => "ArrowDown",
        Key::LeftArrow => "ArrowLeft",
        Key::RightArrow => "ArrowRight",
        // Modifiers
        Key::ShiftLeft => "ShiftLeft",
        Key::ShiftRight => "ShiftRight",
        Key::ControlLeft => "ControlLeft",
        Key::ControlRight => "ControlRight",
        Key::Alt => "AltLeft",
        Key::AltGr => "AltRight",
        Key::MetaLeft => "MetaLeft",
        Key::MetaRight => "MetaRight",

        // Function keys
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        // Punctuation and symbols
        Key::Comma => "Comma",
        Key::Dot => "Period",
        Key::Slash => "Slash",
        Key::SemiColon => "Semicolon",
        Key::Quote => "Quote",
        Key::LeftBracket => "BracketLeft",
        Key::RightBracket => "BracketRight",
        Key::BackSlash => "Backslash",
        Key::Minus => "Minus",
        Key::Equal => "Equal",
        Key::BackQuote => "Backquote",

        // Numpad
        Key::KpMinus => "NumpadSubtract",
        Key::KpPlus => "NumpadAdd",
        Key::KpMultiply => "NumpadMultiply",
        Key::KpDivide => "NumpadDivide",
        Key::KpReturn => "NumpadEnter",
        Key::KpDelete => "NumpadDecimal",
        Key::Kp0 => "Numpad0",
        Key::Kp1 => "Numpad1",
        Key::Kp2 => "Numpad2",
        Key::Kp3 => "Numpad3",
        Key::Kp4 => "Numpad4",
        Key::Kp5 => "Numpad5",
        Key::Kp6 => "Numpad6",
        Key::Kp7 => "Numpad7",
        Key::Kp8 => "Numpad8",
        Key::Kp9 => "Numpad9",

        // Lock keys
        Key::CapsLock => "CapsLock",
        Key::NumLock => "NumLock",
        Key::ScrollLock => "ScrollLock",

        // Other keys
        Key::PrintScreen => "PrintScreen",
        Key::Pause => "Pause",

        _ => "", // Unknown key
    }
}

// Maps a mouse button to its standardized code
fn map_button_to_code(button: Button) -> &'static str {
    match button {
        Button::Left => "MouseLeft",
        Button::Right => "MouseRight",
        Button::Middle => "MouseMiddle",
        Button::Unknown(_) => "MouseUnknown",
    }
}

impl ServiceDaemon {
    /// Create a new service daemon
    pub fn new() -> Self {
        Self {
            ipc_server: None,
            is_running: Arc::new(AtomicBool::new(false)),
            input_capture_active: Arc::new(AtomicBool::new(false)),
            event_sender: None,
        }
    }
    /// Start the service daemon
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting MechVibes DX Service Daemon...");

        // Check if we have appropriate privileges
        let has_admin = admin::is_running_as_admin();
        println!("🔐 Running with {} privileges", if has_admin {
            "administrator"
        } else {
            "standard user"
        });

        // Initialize IPC server
        self.ipc_server = Some(IpcServer::new().await?);
        self.is_running.store(true, Ordering::Relaxed);

        // Create event channel for input capture
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<String>();
        self.event_sender = Some(event_tx);

        // Create shared references for the message handler
        let is_running = Arc::clone(&self.is_running);
        let input_capture_active = Arc::clone(&self.input_capture_active);
        let event_sender = self.event_sender.clone();

        let message_handler = move |message: ServiceMessage| -> ServiceMessage {
            match message {
                ServiceMessage::GetStatus => {
                    let status = ServiceStatus {
                        is_running: is_running.load(Ordering::Relaxed),
                        is_installed: true, // If we're running, we're installed
                        has_privileges: admin::is_running_as_admin(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        platform: std::env::consts::OS.to_string(),
                    };
                    ServiceMessage::StatusResponse(status)
                }

                ServiceMessage::StartInputCapture => {
                    println!("📝 Starting input capture...");
                    input_capture_active.store(true, Ordering::Relaxed);

                    // Start input capture in separate thread
                    if let Some(ref sender) = event_sender {
                        let sender_clone = sender.clone();
                        let active_flag = Arc::clone(&input_capture_active);

                        thread::spawn(move || {
                            start_input_capture(sender_clone, active_flag);
                        });
                    }

                    ServiceMessage::InputCaptureStarted
                }

                ServiceMessage::StopInputCapture => {
                    println!("⏸️ Stopping input capture...");
                    input_capture_active.store(false, Ordering::Relaxed);
                    ServiceMessage::InputCaptureStopped
                }

                ServiceMessage::ShutdownService => {
                    println!("🛑 Shutting down service...");
                    is_running.store(false, Ordering::Relaxed);
                    ServiceMessage::ServiceShutdown
                }

                _ => ServiceMessage::Error("Unsupported message".to_string()),
            }
        };

        // Handle input events in separate task
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                // TODO: Forward events to connected clients via IPC
                println!("🎮 Input event: {}", event);
            }
        });

        // Start IPC server
        if let Some(ref mut server) = self.ipc_server {
            println!("✅ Service daemon started successfully");
            server.run(message_handler).await?;
        }

        Ok(())
    }

    /// Stop the service daemon gracefully
    pub async fn stop(&mut self) {
        println!("🛑 Stopping service daemon...");
        self.is_running.store(false, Ordering::Relaxed);
        self.input_capture_active.store(false, Ordering::Relaxed);
    }

    /// Check if the daemon is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Check if input capture is active
    pub fn is_input_capture_active(&self) -> bool {
        self.input_capture_active.load(Ordering::Relaxed)
    }
}

/// Start input capture in a separate thread
fn start_input_capture(event_sender: mpsc::UnboundedSender<String>, is_active: Arc<AtomicBool>) {
    println!("🎮 Starting privileged input capture...");

    // Separate state tracking for keyboard and mouse
    let keyboard_last_press = Arc::new(Mutex::new(Instant::now()));
    let mouse_last_press = Arc::new(Mutex::new(Instant::now()));
    let pressed_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
    let pressed_buttons = Arc::new(Mutex::new(HashSet::<String>::new()));

    // Track pressed modifier keys for hotkey detection
    let mut ctrl_pressed = false;
    let mut alt_pressed = false;

    let result = listen(move |event: Event| {
        // Check if input capture is still active
        if !is_active.load(Ordering::Relaxed) {
            return;
        }

        match event.event_type {
            // ===== KEYBOARD EVENTS =====
            EventType::KeyPress(key) => {
                let key_code = map_key_to_code(key);
                if !key_code.is_empty() {
                    // Track modifier keys for hotkey detection
                    match key_code {
                        "ControlLeft" | "ControlRight" => {
                            ctrl_pressed = true;
                        }
                        "AltLeft" | "AltRight" => {
                            alt_pressed = true;
                        }
                        "KeyM" => {
                            // Check for Ctrl+Alt+M hotkey combination
                            if ctrl_pressed && alt_pressed {
                                println!("🔥 Hotkey detected: Ctrl+Alt+M - Toggling global sound");
                                let _ = event_sender.send("HOTKEY:TOGGLE_SOUND".to_string());
                                return; // Don't process this as a regular key event
                            }
                        }
                        _ => {}
                    }

                    // Check if key is already pressed
                    let mut pressed = pressed_keys.lock().unwrap();
                    if pressed.contains(&key_code.to_string()) {
                        return; // Key already pressed, ignore
                    }
                    pressed.insert(key_code.to_string());
                    drop(pressed);

                    // Apply debounce
                    let now = Instant::now();
                    let mut last = keyboard_last_press.lock().unwrap();
                    if now.duration_since(*last) > Duration::from_millis(1) {
                        *last = now;
                        let _ = event_sender.send(format!("KEYBOARD:{}", key_code));
                    }
                }
            }
            EventType::KeyRelease(key) => {
                let key_code = map_key_to_code(key);
                if !key_code.is_empty() {
                    // Track modifier key releases for hotkey detection
                    match key_code {
                        "ControlLeft" | "ControlRight" => {
                            ctrl_pressed = false;
                        }
                        "AltLeft" | "AltRight" => {
                            alt_pressed = false;
                        }
                        _ => {}
                    }

                    // Remove key from pressed set
                    let mut pressed = pressed_keys.lock().unwrap();
                    pressed.remove(&key_code.to_string());
                    drop(pressed);

                    let _ = event_sender.send(format!("KEYBOARD:UP:{}", key_code));
                }
            }

            // ===== MOUSE EVENTS =====
            EventType::ButtonPress(button) => {
                let button_code = map_button_to_code(button);
                if !button_code.is_empty() && button_code != "MouseUnknown" {
                    // Check if button is already pressed
                    let mut pressed = pressed_buttons.lock().unwrap();
                    if pressed.contains(&button_code.to_string()) {
                        return; // Button already pressed, ignore
                    }
                    pressed.insert(button_code.to_string());
                    drop(pressed);

                    // Apply debounce
                    let now = Instant::now();
                    let mut last = mouse_last_press.lock().unwrap();
                    if now.duration_since(*last) > Duration::from_millis(1) {
                        *last = now;
                        let _ = event_sender.send(format!("MOUSE:{}", button_code));
                    }
                }
            }
            EventType::ButtonRelease(button) => {
                let button_code = map_button_to_code(button);
                if !button_code.is_empty() && button_code != "MouseUnknown" {
                    // Remove button from pressed set
                    let mut pressed = pressed_buttons.lock().unwrap();
                    pressed.remove(&button_code.to_string());
                    drop(pressed);

                    let _ = event_sender.send(format!("MOUSE:UP:{}", button_code));
                }
            }
            EventType::Wheel { delta_x: _, delta_y } => {
                let wheel_event = if delta_y > 0 {
                    "MouseWheelUp"
                } else if delta_y < 0 {
                    "MouseWheelDown"
                } else {
                    return; // No vertical scroll, ignore
                };

                // Apply longer debounce for wheel events
                let now = Instant::now();
                let mut last = mouse_last_press.lock().unwrap();
                if now.duration_since(*last) > Duration::from_millis(50) {
                    *last = now;
                    let _ = event_sender.send(format!("MOUSE:{}", wheel_event));
                }
            }
            EventType::MouseMove { x: _, y: _ } => {
                // Mouse move events are too noisy, ignore them
            }
        }
    });

    if let Err(error) = result {
        eprintln!("❌ Input capture error: {:?}", error);
    }

    println!("🎮 Input capture stopped");
}

/// Service entry point for command line --service flag
pub async fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 MechVibes DX Service starting...");

    // Set up signal handling for graceful shutdown
    let mut daemon = ServiceDaemon::new();
    // Handle SIGINT/SIGTERM for graceful shutdown
    let is_running = Arc::new(AtomicBool::new(true));

    #[cfg(unix)]
    {
        use tokio::signal;

        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())?;
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())?;
        let is_running_clone = Arc::clone(&is_running);

        tokio::spawn(async move {
            tokio::select! {
                _ = sigint.recv() => {
                    println!("📡 Received SIGINT, shutting down...");
                    is_running_clone.store(false, Ordering::Relaxed);
                }
                _ = sigterm.recv() => {
                    println!("📡 Received SIGTERM, shutting down...");
                    is_running_clone.store(false, Ordering::Relaxed);
                }
            }
        });
    }
    #[cfg(target_os = "windows")]
    {
        use tokio::signal;

        let ctrl_c = signal::ctrl_c();
        let is_running_clone = Arc::clone(&is_running);

        tokio::spawn(async move {
            let _ = ctrl_c.await;
            println!("📡 Received Ctrl+C, shutting down...");
            is_running_clone.store(false, Ordering::Relaxed);
        });
    }

    // Start the daemon
    let daemon_task = tokio::spawn(async move {
        if let Err(e) = daemon.start().await {
            eprintln!("❌ Service daemon error: {}", e);
        }
    });

    // Wait for shutdown signal
    while is_running.load(Ordering::Relaxed) {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Graceful shutdown
    daemon_task.abort();

    println!("✅ MechVibes DX Service stopped gracefully");
    Ok(())
}
