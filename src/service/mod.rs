/// Service module for handling privileged operations via IPC
/// This architecture separates the UI from privileged operations,
/// making it easier to support different platforms and security models

pub mod daemon;
pub mod ipc;
pub mod manager;

// Re-export key types for easier access
pub use manager::ServiceManager;

use serde::{ Deserialize, Serialize };

/// Service state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub is_running: bool,
    pub is_installed: bool,
    pub has_privileges: bool,
    pub version: String,
    pub platform: String,
}

/// Messages that can be sent between UI and service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceMessage {
    // Status queries
    GetStatus,
    StatusResponse(ServiceStatus),

    // Input capture control
    StartInputCapture,
    StopInputCapture,
    InputCaptureStarted,
    InputCaptureStopped,

    // Input events
    KeyPressed {
        key: String,
        timestamp: u64,
    },
    KeyReleased {
        key: String,
        timestamp: u64,
    },
    MousePressed {
        button: String,
        timestamp: u64,
    },
    MouseReleased {
        button: String,
        timestamp: u64,
    },

    // Service control
    ShutdownService,
    ServiceShutdown,

    // Error handling
    Error(String),
}

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_name: String,
    pub display_name: String,
    pub description: String,
    pub executable_path: String,
    pub auto_start: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            service_name: "MechVibesDXService".to_string(),
            display_name: "MechVibes DX Service".to_string(),
            description: "MechVibes DX privileged input capture service".to_string(),
            executable_path: String::new(),
            auto_start: true,
        }
    }
}
