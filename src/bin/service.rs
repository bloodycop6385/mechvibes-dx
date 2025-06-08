// Service binary - separate executable for the privileged service daemon
// This allows the main app to run with standard privileges while the service
// runs with elevated privileges when needed

use mechvibes_dx::service::daemon;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for service
    env_logger::init();

    println!("🚀 MechVibes DX Service Daemon starting...");

    // Run the service daemon
    daemon::run_service().await
}
