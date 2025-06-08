/// Input Service Manager - Handles input capture via service when admin mode is enabled
/// Falls back to local input capture when service is not available

use crate::service::ServiceManager;
use crate::service::ServiceMessage;
use crate::libs::input_listener::start_unified_input_listener;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{ AtomicBool, Ordering };

pub struct InputService {
    service_manager: ServiceManager,
    is_using_service: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
}

impl InputService {
    /// Create a new input service manager
    pub fn new() -> Self {
        Self {
            service_manager: ServiceManager::new(),
            is_using_service: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start input capture (service or local based on admin mode)
    pub async fn start_input_capture(
        &mut self,
        keyboard_tx: mpsc::Sender<String>,
        mouse_tx: mpsc::Sender<String>,
        hotkey_tx: mpsc::Sender<String>,
        use_service: bool
    ) -> Result<(), Box<dyn std::error::Error>> {
        if use_service {
            // Try to use service-based input capture
            match self.try_service_input_capture(&keyboard_tx, &mouse_tx, &hotkey_tx).await {
                Ok(()) => {
                    println!("✅ Using service-based input capture with elevated privileges");
                    self.is_using_service.store(true, Ordering::Relaxed);
                    self.is_running.store(true, Ordering::Relaxed);
                    return Ok(());
                }
                Err(e) => {
                    println!("⚠️ Service input capture failed: {}, falling back to local capture", e);
                }
            }
        }

        // Fall back to local input capture
        println!("🎮 Using local input capture");
        self.start_local_input_capture(keyboard_tx, mouse_tx, hotkey_tx);
        self.is_using_service.store(false, Ordering::Relaxed);
        self.is_running.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// Try to start service-based input capture
    async fn try_service_input_capture(
        &mut self,
        _keyboard_tx: &mpsc::Sender<String>,
        _mouse_tx: &mpsc::Sender<String>,
        _hotkey_tx: &mpsc::Sender<String>
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if service is running
        if !self.service_manager.is_service_running().await {
            return Err("Service is not running".into());
        }

        // Send start input capture message to service
        self.service_manager.send_message(ServiceMessage::StartInputCapture).await?;

        // TODO: Set up IPC communication to receive input events from service
        // For now, we'll implement a placeholder that forwards events

        println!("📡 Service input capture started");
        Ok(())
    }

    /// Start local input capture (fallback)
    fn start_local_input_capture(
        &self,
        keyboard_tx: mpsc::Sender<String>,
        mouse_tx: mpsc::Sender<String>,
        hotkey_tx: mpsc::Sender<String>
    ) {
        start_unified_input_listener(keyboard_tx, mouse_tx, hotkey_tx);
    }

    /// Stop input capture
    pub async fn stop_input_capture(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        if self.is_using_service.load(Ordering::Relaxed) {
            // Stop service input capture
            self.service_manager.send_message(ServiceMessage::StopInputCapture).await?;
            println!("📡 Service input capture stopped");
        } else {
            // Local input capture stops automatically when channels are dropped
            println!("🎮 Local input capture stopped");
        }

        self.is_running.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// Check if using service-based input capture
    pub fn is_using_service(&self) -> bool {
        self.is_using_service.load(Ordering::Relaxed)
    }

    /// Check if input capture is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Get status description
    pub fn get_status(&self) -> String {
        if !self.is_running() {
            "Input capture stopped".to_string()
        } else if self.is_using_service() {
            "Using service-based input capture (elevated privileges)".to_string()
        } else {
            "Using local input capture (standard privileges)".to_string()
        }
    }
}
