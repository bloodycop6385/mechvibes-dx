use crate::components::ui::{ Collapse, PageHeader, Toggler };
use crate::libs::theme::{ use_theme, BuiltInTheme, Theme };
use crate::libs::tray_service::request_tray_update;
use crate::service::ServiceManager;
use crate::utils::config::use_config;
use crate::utils::constants::{ APP_NAME_DISPLAY, APP_NAME };
use dioxus::prelude::*;
use lucide_dioxus::Settings;

#[component]
#[allow(non_snake_case)]
pub fn SettingsPage() -> Element {
    // Use shared config hook
    let (config, update_config) = use_config();

    // Use computed signals that always reflect current config state
    let enable_sound = use_memo(move || config().enable_sound);
    let auto_start = use_memo(move || config().auto_start);
    let start_minimized = use_memo(move || config().start_minimized);
    let admin_mode_enabled = use_memo(move || config().admin_mode_enabled);
    let show_notifications = use_memo(move || config().show_notifications);
    let show_debug_console = use_memo(move || config().show_debug_console);
    // Service manager state (not needed as variable, just for status tracking)
    let service_status = use_signal(|| "Checking service status...".to_string()); // Initialize service status
    use_effect(move || {
        let mut service_status = service_status.clone();
        spawn(async move {
            let mut manager = ServiceManager::new();
            let status = manager.get_status_description().await;
            service_status.set(status);
        });
    });

    // Theme state - use theme context (initialized in Layout component)
    let mut theme = use_theme();
    rsx! {
      div { class: "p-12 pb-32", // Page header
        PageHeader {
          title: "Settings".to_string(),
          subtitle: format!("Config your {} experience.", APP_NAME_DISPLAY),
          icon: Some(rsx! {
            Settings { class: "w-8 h-8 mx-auto" }
          }),
        }

        // Settings sections
        div { class: "space-y-4",
          // General Settings Section
          Collapse {
            title: "General".to_string(),
            group_name: "setting-accordion".to_string(),
            default_open: true,
            content_class: "collapse-content text-sm",
            children: rsx! {
              div { class: "space-y-6", // Volume Control
                Toggler {
                  title: "Enable all sounds".to_string(),
                  description: Some("You can also use Ctrl+Alt+M to toggle sound on/off".to_string()),
                  checked: enable_sound(),
                  on_change: {
                      let update_config = update_config.clone();
                      move |new_value: bool| {
                          update_config(
                              Box::new(move |config| {
                                  config.enable_sound = new_value;
                              }),
                          );
                          request_tray_update();
                      }
                  },
                }
                // Auto Start
                Toggler {
                  title: "Start with Windows".to_string(),
                  description: Some(format!("Automatically start {} when Windows boots", APP_NAME)),
                  checked: auto_start(),
                  on_change: {
                      let update_config = update_config.clone();
                      move |new_value: bool| {
                          update_config(
                              Box::new(move |config| {
                                  config.auto_start = new_value;
                              }),
                          );
                          spawn(async move {
                              match crate::utils::auto_startup::set_auto_startup(new_value) {
                                  Ok(_) => {
                                      let status = if new_value { "enabled" } else { "disabled" };
                                      println!("✅ Auto startup {}", status);
                                  }
                                  Err(e) => {
                                      eprintln!("❌ Failed to set auto startup: {}", e);
                                  }
                              }
                          });
                      }
                  },
                }
                // Start Minimized (only show when auto start is enabled)
                if auto_start() {
                  Toggler {
                    title: "Start minimized to tray".to_string(),
                    description: Some("When starting with Windows, open minimized to system tray".to_string()),
                    checked: start_minimized(),
                    on_change: {
                        let update_config = update_config.clone();
                        move |new_value: bool| {
                            update_config(
                                Box::new(move |config| {
                                    config.start_minimized = new_value;
                                }),
                            );
                            spawn(async move {
                                if crate::state::config::AppConfig::load().auto_start {
                                    match crate::utils::auto_startup::set_auto_startup(true) {
                                        Ok(_) => {
                                            let status = if new_value {
                                                "with minimized flag"
                                            } else {
                                                "without minimized flag"
                                            };
                                            println!("✅ Auto startup updated {}", status);
                                        }
                                        Err(e) => {
                                            eprintln!("❌ Failed to update auto startup: {}", e);
                                        }
                                    }
                                }
                            });
                        }
                    },
                  }
                }
                // Notifications
                Toggler {
                  title: "Show Notifications".to_string(),
                  description: Some("Display system notifications for important events".to_string()),
                  checked: show_notifications(),
                  on_change: {
                      let update_config = update_config.clone();
                      move |new_value: bool| {
                          update_config(
                              Box::new(move |config| {
                                  config.show_notifications = new_value;
                              }),
                          );
                      }
                  },
                } // Debug Console
                Toggler {
                  title: "Show Debug Console".to_string(),
                  description: Some("Show terminal window for debugging (requires restart)".to_string()),
                  checked: show_debug_console(),
                  on_change: {
                      let update_config = update_config.clone();
                      move |new_value: bool| {
                          update_config(
                              Box::new(move |config| {
                                  config.show_debug_console = new_value;
                              }),
                          );
                      }
                  },
                } // Administrator Mode (Service) - Windows only
                if cfg!(target_os = "windows") {
                  div { class: "space-y-4",
                    h4 { class: "font-medium text-base-content", "Administrator Mode" }
                    // Service status display
                    div { class: "alert alert-info",
                      div { class: "flex items-center gap-3",
                        span { class: "text-info text-xl", "🛡️" }
                        div {
                          div { class: "font-medium", "Service Status" }
                          div { class: "text-sm opacity-70 mt-1", "{service_status()}" }
                        }
                      }
                    } // Admin mode toggle
                    Toggler {
                      title: "Enable Administrator Mode".to_string(),
                      description: Some(
                          "Install privileged service for full input capture (no UAC prompts after setup)"
                              .to_string(),
                      ),
                      checked: admin_mode_enabled(),
                      on_change: {
                          let update_config = update_config.clone();
                          let service_status = service_status.clone();
                          move |new_value: bool| {
                              {
                                  let update_config = update_config.clone();
                                  update_config(
                                      Box::new(move |config| {
                                          config.admin_mode_enabled = new_value;
                                      }),
                                  );
                              }
                              {
                                  let mut service_status = service_status.clone();
                                  let update_config = update_config.clone();
                                  spawn(async move {
                                      let manager = ServiceManager::new();
                                      if new_value {
                                          service_status.set("Installing service...".to_string());
                                          match manager.install_service().await {
                                              Ok(_) => {
                                                  match manager.start_service().await {
                                                      Ok(_) => {
                                                          service_status
                                                              .set("Service installed and running".to_string());
                                                      }
                                                      Err(e) => {
                                                          service_status
                                                              .set(
                                                                  format!("Service installed but failed to start: {}", e),
                                                              );
                                                      }
                                                  }
                                              }
                                              Err(e) => {
                                                  service_status
                                                      .set(format!("Failed to install service: {}", e));
                                                  update_config(
                                                      Box::new(move |config| {
                                                          config.admin_mode_enabled = false;
                                                      }),
                                                  );
                                              }
                                          }
                                      } else {
                                          service_status.set("Uninstalling service...".to_string());
                                          match manager.uninstall_service().await {
                                              Ok(_) => {
                                                  service_status.set("Service uninstalled".to_string());
                                              }
                                              Err(e) => {
                                                  service_status
                                                      .set(format!("Failed to uninstall service: {}", e));
                                              }
                                          }
                                      }
                                  });
                              }
                          }
                      },
                    }
                    // Information about admin mode
                    div { class: "text-sm text-base-content/70 bg-base-100 p-3 rounded-lg border border-base-300",
                      p { class: "mb-2",
                        "Administrator Mode uses a Windows Service to provide elevated privileges without UAC prompts."
                      }
                      p { class: "mb-2", "Benefits of Administrator Mode:" }
                      ul { class: "list-disc list-inside space-y-1 ml-2",
                        li { "Capture input from elevated processes (Task Manager, UAC dialogs, etc.)" }
                        li { "No UAC prompts after initial service installation" }
                        li { "Reliable system-level input event access" }
                        li { "Better compatibility with security software" }
                      }
                      if !admin_mode_enabled() {
                        p { class: "mt-3 text-warning font-medium",
                          "⚠️ Without Administrator Mode, input capture may be blocked by elevated processes."
                        }
                      }
                    }
                  }
                }
              }
            },
          }
          // App info Section
          Collapse {
            title: "App info".to_string(),
            group_name: "setting-accordion".to_string(),
            content_class: "collapse-content text-sm",
            children: rsx! {
              crate::components::app_info::AppInfoDisplay {}
            },
          }
          // Danger Zone Section
          Collapse {
            title: "Danger zone".to_string(),
            group_name: "setting-accordion".to_string(),
            title_class: "collapse-title font-semibold text-error",
            variant: "border border-base-300 bg-base-200",
            content_class: "collapse-content text-sm",
            children: rsx! {
              p { class: "mb-4 text-base-content/70",
                "Reset all settings to their default values. This action cannot be undone."
              }
              div { class: " justify-start",
                button {
                  class: "btn btn-error btn-soft btn-sm",
                  onclick: {
                      let update_config = update_config.clone();
                      move |_| {
                          theme.set(Theme::BuiltIn(BuiltInTheme::System));
                          update_config(
                              Box::new(|config| {
                                  config.volume = 1.0;
                                  config.enable_sound = true;
                                  config.auto_start = false;
                                  config.show_notifications = true;
                                  config.theme = Theme::BuiltIn(BuiltInTheme::System);
                              }),
                          );
                      }
                  },
                  "Reset to Defaults"
                }
              }
            },
          }
        }
      }
    }
}
