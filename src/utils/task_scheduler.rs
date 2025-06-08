use std::process::Command;

/// Task name for the scheduled admin task
const TASK_NAME: &str = "MechVibesDX-AdminMode";

/// Task Scheduler utility for managing elevated privileges without UAC prompts
/// This creates a Windows Task Scheduler task that runs with elevated privileges
/// but doesn't show UAC prompts after initial setup
pub struct TaskScheduler;

impl TaskScheduler {
    /// Check if the admin mode task exists in Task Scheduler
    pub fn is_admin_task_installed() -> bool {
        let output = Command::new("schtasks").args(&["/query", "/tn", TASK_NAME]).output();

        match output {
            Ok(result) => result.status.success(),
            Err(_) => false,
        }
    }

    /// Install the admin mode task in Task Scheduler
    /// This requires UAC prompt only during setup, not during runtime
    pub fn install_admin_task(exe_path: &str) -> Result<(), String> {
        // First check if task already exists
        if Self::is_admin_task_installed() {
            return Ok(()); // Task already installed
        }

        // Create the scheduled task with highest privileges
        let output = Command::new("schtasks")
            .args(
                &[
                    "/create",
                    "/tn",
                    TASK_NAME,
                    "/tr",
                    exe_path,
                    "/sc",
                    "onlogon",
                    "/rl",
                    "highest", // Run with highest privileges
                    "/f", // Force create (overwrite if exists)
                ]
            )
            .output()
            .map_err(|e| format!("Failed to execute schtasks: {}", e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create scheduled task: {}", error));
        }

        println!("✅ Admin mode task installed successfully");
        Ok(())
    }

    /// Remove the admin mode task from Task Scheduler
    pub fn uninstall_admin_task() -> Result<(), String> {
        if !Self::is_admin_task_installed() {
            return Ok(()); // Task doesn't exist, nothing to do
        }

        let output = Command::new("schtasks")
            .args(&["/delete", "/tn", TASK_NAME, "/f"])
            .output()
            .map_err(|e| format!("Failed to execute schtasks: {}", e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to delete scheduled task: {}", error));
        }

        println!("✅ Admin mode task removed successfully");
        Ok(())
    }

    /// Start the application via the scheduled task (with elevated privileges)
    /// This won't show UAC prompt because the task is pre-authorized
    pub fn start_via_admin_task(_args: &[&str]) -> Result<(), String> {
        if !Self::is_admin_task_installed() {
            return Err("Admin mode task is not installed".to_string());
        }

        // Build the command with arguments
        let cmd_args = vec!["/run", "/tn", TASK_NAME];

        // Note: schtasks doesn't directly support arguments, so we'll need to
        // modify the task if we need to pass arguments. For now, we'll start basic task.
        let output = Command::new("schtasks")
            .args(&cmd_args)
            .output()
            .map_err(|e| format!("Failed to execute schtasks: {}", e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to run scheduled task: {}", error));
        }

        Ok(())
    }

    /// Check if current process is running with elevated privileges
    pub fn is_running_elevated() -> bool {
        crate::utils::admin::is_running_as_admin()
    }

    /// Get the full path of the current executable
    pub fn get_current_exe_path() -> Result<String, String> {
        std::env
            ::current_exe()
            .map_err(|e| format!("Failed to get current executable path: {}", e))?
            .to_str()
            .ok_or_else(|| "Invalid executable path".to_string())
            .map(|s| s.to_string())
    }

    /// Get user-friendly status of admin mode
    pub fn get_admin_mode_status() -> String {
        if Self::is_running_elevated() {
            "Running with Administrator privileges".to_string()
        } else if Self::is_admin_task_installed() {
            "Admin mode available (no UAC required)".to_string()
        } else {
            "Standard user mode".to_string()
        }
    }

    /// Check if we can enable admin mode (install task)
    pub fn can_enable_admin_mode() -> bool {
        // We can enable admin mode if:
        // 1. Not already running elevated
        // 2. Not already installed
        !Self::is_running_elevated() && !Self::is_admin_task_installed()
    }

    /// Check if we can disable admin mode (remove task)
    pub fn can_disable_admin_mode() -> bool {
        // We can disable admin mode if:
        // 1. Not currently running elevated (avoid removing while running)
        // 2. Task is installed
        !Self::is_running_elevated() && Self::is_admin_task_installed()
    }
}
