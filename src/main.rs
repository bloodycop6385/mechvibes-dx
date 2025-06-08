// #![windows_subsystem = "windows"]
#![allow(non_snake_case)]

mod components;
mod libs;
mod service;
mod state;
mod utils;

use dioxus::desktop::{ Config, LogicalPosition, LogicalSize, WindowBuilder };
use dioxus::prelude::*;
use libs::protocol;
use utils::constants::{ APP_NAME, APP_PROTOCOL_URL };
use libs::ui;
use libs::window_manager::{ WindowAction, WINDOW_MANAGER };
use libs::input_service::InputService;
use libs::input_manager::init_input_channels;
use std::sync::mpsc;

// Function to conditionally set windows subsystem based on config
fn should_show_console() -> bool {
    // Try to load config to check debug console setting
    match std::panic::catch_unwind(|| { state::config::AppConfig::load().show_debug_console }) {
        Ok(show_debug) => show_debug,
        Err(_) => false, // Default to false if config loading fails
    }
}

// Use .ico format for better Windows compatibility
const EMBEDDED_ICON: &[u8] = include_bytes!("../assets/icon.ico");

fn load_icon() -> Option<dioxus::desktop::tao::window::Icon> {
    // Try to create icon from embedded ICO data
    match image::load_from_memory_with_format(EMBEDDED_ICON, image::ImageFormat::Ico) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            match dioxus::desktop::tao::window::Icon::from_rgba(rgba.into_raw(), width, height) {
                Ok(icon) => {
                    debug_print!("✅ Loaded embedded ICO icon ({}x{})", width, height);
                    Some(icon)
                }
                Err(e) => {
                    always_eprint!("❌ Failed to create icon from embedded ICO data: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            debug_eprint!("❌ Failed to load embedded ICO data: {}", e);
            None
        }
    }
}

fn main() {
    // Initialize debug logging first
    utils::logger::init_debug_logging();

    // Check for command line arguments first
    let args: Vec<String> = std::env::args().collect();

    // Check if we should run as service daemon
    if args.contains(&"--service".to_string()) {
        // Run as service daemon
        tokio::runtime::Runtime
            ::new()
            .unwrap()
            .block_on(async {
                if let Err(e) = service::daemon::run_service().await {
                    eprintln!("❌ Service error: {}", e);
                    std::process::exit(1);
                }
            });
        return;
    }

    // Hide console window if debug console is disabled in config
    if !should_show_console() {
        #[cfg(windows)]
        {
            unsafe {
                let console = winapi::um::wincon::GetConsoleWindow();
                if !console.is_null() {
                    winapi::um::winuser::ShowWindow(console, winapi::um::winuser::SW_HIDE);
                }
            }
        }
    }
    env_logger::init();

    debug_print!("🚀 Initializing {}...", APP_NAME);

    // Check and log current privilege level
    #[cfg(target_os = "windows")]
    {
        let privilege_level = utils::admin::get_privilege_level_description();
        if utils::admin::is_running_as_admin() {
            debug_print!("🔐 Running with {} privileges", privilege_level);
            debug_print!("✅ Input capture from elevated processes is enabled");
        } else {
            debug_print!("👤 Running with {} privileges", privilege_level);
            debug_print!("⚠️ Administrator privileges not available");
            debug_print!(
                "💡 This may be because UAC was declined or app was built without admin manifest"
            );
            debug_print!(
                "ℹ️ Input capture may be limited for elevated processes like Task Manager"
            );
        }
    }

    // Initialize app manifest first
    let _manifest = state::manifest::AppManifest::load(); // Check for command line arguments (protocol handling and startup options)
    debug_print!("🔍 Command line args: {:?}", args); // Check if we should start minimized (from auto-startup)
    let should_start_minimized =
        args.contains(&"--minimized".to_string()) ||
        (state::config::AppConfig::load().auto_start &&
            state::config::AppConfig::load().start_minimized);

    if should_start_minimized {
        debug_print!("🔽 Starting minimized to system tray");
    }

    if args.len() > 1 {
        // Handle protocol URL if passed as argument
        for arg in &args[1..] {
            if arg == "--minimized" {
                debug_print!("🔽 Starting minimized to tray");
                continue;
            }

            if arg.starts_with(APP_PROTOCOL_URL) {
                debug_print!("✅ Detected protocol URL: {}", arg);
                if let Err(e) = protocol::handle_protocol_url(arg) {
                    always_eprint!("❌ Failed to handle protocol URL {}: {}", arg, e);
                } else {
                    debug_print!("✅ Protocol URL handled successfully");
                }
                return; // Exit after handling protocol
            }
        }
    } else {
        debug_print!("ℹ️ No command line arguments provided");
    }

    // Register protocol on first run
    // if let Err(e) = protocol::register_protocol() {
    //     eprintln!("Warning: Failed to register mechvibes:// protocol: {}", e);
    // }    // Initialize global app state before rendering
    state::app::init_app_state();

    // Load config to check admin mode setting
    let config = state::config::AppConfig::load();
    let use_service_input = config.admin_mode_enabled;

    // Create input event channels for communication between input listener and UI
    let (keyboard_tx, keyboard_rx) = mpsc::channel::<String>();
    let (mouse_tx, mouse_rx) = mpsc::channel::<String>();
    let (hotkey_tx, hotkey_rx) = mpsc::channel::<String>();

    // Initialize global input channels for UI to access
    init_input_channels(keyboard_rx, mouse_rx, hotkey_rx);

    // Start input capture (service-based or local)
    debug_print!("🎮 Starting input capture system...");
    let mut input_service = InputService::new();

    // Start input capture in a separate task since it's async
    let keyboard_tx_clone = keyboard_tx.clone();
    let mouse_tx_clone = mouse_tx.clone();
    let hotkey_tx_clone = hotkey_tx.clone();

    tokio::runtime::Runtime
        ::new()
        .unwrap()
        .spawn(async move {
            if
                let Err(e) = input_service.start_input_capture(
                    keyboard_tx_clone,
                    mouse_tx_clone,
                    hotkey_tx_clone,
                    use_service_input
                ).await
            {
                eprintln!("❌ Failed to start input capture: {}", e);
            }
        });

    // Create window action channel
    let (window_tx, _window_rx) = mpsc::channel::<WindowAction>();
    WINDOW_MANAGER.set_action_sender(window_tx); // Create a WindowBuilder with custom appearance
    let window_builder = WindowBuilder::default()
        .with_title(APP_NAME)
        .with_transparent(false) // Disable transparency for better performance
        .with_always_on_top(false) // Allow normal window behavior for taskbar
        .with_position(LogicalPosition::new(1700.0, 300.0))
        .with_inner_size(LogicalSize::new(500.0, 850.0))
        .with_fullscreen(None)
        .with_decorations(false) // Use custom title bar
        .with_resizable(false) // Disable window resizing
        .with_visible(!should_start_minimized) // Hide window if starting minimized
        .with_window_icon(load_icon()); // Set window icon for taskbar

    // Create config with our window settings
    let config = Config::new().with_window(window_builder).with_menu(None); // Launch the app with our config
    dioxus::LaunchBuilder::desktop().with_cfg(config).launch(app_with_stylesheets)
}

fn app_with_stylesheets() -> Element {
    rsx! {
        ui::app {}
    }
}
