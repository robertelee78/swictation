// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod database;
mod models;
mod socket;
mod utils;

use commands::{AppState, ConfigState, CorrectionsState};
use database::Database;
use image::GenericImageView;
use socket::daemon_ipc::{self, DaemonState};
use socket::MetricsSocket;
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

/// Load a tray icon from embedded PNG bytes.
fn load_tray_icon(png_bytes: &[u8]) -> Image<'static> {
    let img = image::load_from_memory(png_bytes).expect("Failed to load tray icon");
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();
    Image::new_owned(rgba.into_raw(), width, height)
}

/// Create a red-tinted recording icon from the template source.
///
/// macOS template icons (isTemplate=true) discard all RGB data and use only
/// the alpha channel as a mask. To show a colored icon, we must:
/// 1. Set icon_as_template(false) so macOS renders actual pixel colors
/// 2. Clean the semi-transparent background haze to fully transparent
/// 3. Tint the icon shape pixels solid red
///
/// This matches how professional macOS menu bar apps show recording state.
fn create_recording_icon(png_bytes: &[u8]) -> Image<'static> {
    let img = image::load_from_memory(png_bytes).expect("Failed to load tray icon");
    let mut rgba = img.to_rgba8();
    let (width, height) = img.dimensions();

    for pixel in rgba.pixels_mut() {
        if pixel[3] > 20 {
            // Visible pixel: tint red, preserve alpha for shape/anti-aliasing
            pixel[0] = 220; // R
            pixel[1] = 40;  // G
            pixel[2] = 40;  // B
        } else {
            // Background haze: force fully transparent so non-template
            // rendering doesn't show a gray blob behind the icon
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 0;
        }
    }

    Image::new_owned(rgba.into_raw(), width, height)
}

/// Create a grayed version of an icon for the disconnected state.
fn create_disconnected_icon(png_bytes: &[u8]) -> Image<'static> {
    let img = image::load_from_memory(png_bytes).expect("Failed to load tray icon");
    let mut rgba = img.to_rgba8();
    let (width, height) = img.dimensions();

    for pixel in rgba.pixels_mut() {
        if pixel[3] > 0 {
            let gray = ((pixel[0] as f32 * 0.299)
                + (pixel[1] as f32 * 0.587)
                + (pixel[2] as f32 * 0.114)) as u8;
            pixel[0] = gray;
            pixel[1] = gray;
            pixel[2] = gray;
            pixel[3] = pixel[3] / 2; // 50% opacity
        }
    }

    Image::new_owned(rgba.into_raw(), width, height)
}

fn main() {
    // Initialize tracing subscriber (compatible with both log and tracing crates)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // macOS: Set activation policy to Accessory to hide from dock
            // This makes the app a pure menu bar app - only the tray icon shows
            // The dock icon won't appear and clicking dock won't reactivate hidden windows
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            // Only create tray icon if not disabled (e.g., when launched from QT tray on Sway)
            if std::env::var("SWICTATION_NO_TRAY").is_err() {
                // Create menu items
                let show_metrics = MenuItemBuilder::with_id("show_metrics", "Show Metrics").build(app)?;
                let toggle_recording = MenuItemBuilder::with_id("toggle_recording", "Toggle Recording").build(app)?;
                let separator = PredefinedMenuItem::separator(app)?;
                let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

                // Build menu
                let menu = Menu::with_items(app, &[&show_metrics, &toggle_recording, &separator, &quit])?;

                // Pre-render icon variants for each state
                let icon_bytes = include_bytes!("../icons/tray-48.png");
                let idle_icon = load_tray_icon(icon_bytes);

                // Build and configure tray icon
                // On macOS: show_menu_on_left_click(true) for native menu bar behavior
                // On Linux: show_menu_on_left_click(false) to allow left-click toggle
                let show_menu_on_left = cfg!(target_os = "macos");

                let tray = TrayIconBuilder::new()
                    .icon(idle_icon)
                    .icon_as_template(true)
                    .menu(&menu)
                    .show_menu_on_left_click(show_menu_on_left)
                    .tooltip("Swictation - Idle")
                    .on_menu_event(|app, event| match event.id.as_ref() {
                    "show_metrics" => {
                        // Show main window
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "toggle_recording" => {
                        // Send toggle command directly to daemon via IPC socket
                        // (matching the Linux Python tray approach)
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            match daemon_ipc::toggle_recording().await {
                                Ok(new_state) => {
                                    log::info!("Toggle recording: {}", new_state.as_str());
                                    let _ = app_handle.emit("daemon-state-changed", new_state.as_str());
                                }
                                Err(e) => {
                                    log::error!("Toggle failed: {}", e);
                                }
                            }
                        });
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        // On Linux: Left click toggles recording (same as Python tray)
                        // On macOS: Left click shows menu (handled by show_menu_on_left_click)
                        #[cfg(not(target_os = "macos"))]
                        {
                            let app = tray.app_handle().clone();
                            tauri::async_runtime::spawn(async move {
                                match daemon_ipc::toggle_recording().await {
                                    Ok(new_state) => {
                                        log::info!("Toggle recording: {}", new_state.as_str());
                                        let _ = app.emit("daemon-state-changed", new_state.as_str());
                                    }
                                    Err(e) => {
                                        log::error!("Toggle failed: {}", e);
                                    }
                                }
                            });
                        }
                        #[cfg(target_os = "macos")]
                        {
                            let _ = tray; // suppress unused warning
                        }
                    }
                    TrayIconEvent::Click {
                        button: MouseButton::Middle,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        // Middle click: Toggle window visibility (same as Python tray)
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    _ => {}
                })
                .build(app)?;

                // Start daemon state polling task (1-second interval, matching Python tray)
                let app_handle = app.handle().clone();
                let tray_id = tray.id().clone();
                tauri::async_runtime::spawn(async move {
                    let icon_bytes = include_bytes!("../icons/tray-48.png");
                    let idle_icon = load_tray_icon(icon_bytes);
                    let recording_icon = create_recording_icon(icon_bytes);
                    let disconnected_icon = create_disconnected_icon(icon_bytes);

                    let mut current_state = DaemonState::Disconnected;

                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                        let new_state = daemon_ipc::query_daemon_state().await;

                        if new_state != current_state {
                            let old_state = current_state.clone();
                            current_state = new_state.clone();

                            // Update tray icon
                            if let Some(tray) = app_handle.tray_by_id(&tray_id) {
                                let (icon, as_template) = match &current_state {
                                    DaemonState::Idle => (&idle_icon, true),
                                    // Recording: disable template mode so red color is visible
                                    // macOS template icons are forced monochrome by the system
                                    DaemonState::Recording => (&recording_icon, false),
                                    DaemonState::Disconnected => (&disconnected_icon, true),
                                };
                                let _ = tray.set_icon(Some(icon.clone()));
                                let _ = tray.set_icon_as_template(as_template);
                                let tooltip = match &current_state {
                                    DaemonState::Idle => "Swictation - Idle",
                                    DaemonState::Recording => "Swictation - Recording",
                                    DaemonState::Disconnected => "Swictation - Disconnected",
                                };
                                let _ = tray.set_tooltip(Some(tooltip));
                            }

                            // Send notifications on recording state transitions
                            // (matching Python tray's showMessage behavior)
                            match (&old_state, &current_state) {
                                (_, DaemonState::Recording) if old_state != DaemonState::Recording => {
                                    let _ = app_handle.emit("recording-notification", "Recording started");
                                    log::info!("Recording started");
                                }
                                (DaemonState::Recording, DaemonState::Idle) => {
                                    let _ = app_handle.emit("recording-notification", "Recording stopped");
                                    log::info!("Recording stopped");
                                }
                                _ => {}
                            }

                            // Emit state change to frontend
                            let _ = app_handle.emit("daemon-state-changed", current_state.as_str());
                        }
                    }
                });
            } // End of tray icon creation (disabled when SWICTATION_NO_TRAY is set)

            // Get database path
            let db_path = utils::get_default_db_path();
            log::info!("Opening database at: {:?}", db_path);

            // Open database (or create if it doesn't exist yet)
            let db = Database::new(&db_path)
                .map_err(|e| {
                    log::warn!("Database not found, will retry on first query: {}", e);
                    e
                })
                .ok();

            // Create app state
            let state = AppState {
                db: Mutex::new(db.unwrap_or_else(|| {
                    // Fallback: try to create database if it doesn't exist
                    Database::new(&db_path).expect("Failed to create database")
                })),
            };

            app.manage(state);

            // Initialize corrections state for learned patterns
            let corrections_state = Mutex::new(CorrectionsState::new());
            app.manage(corrections_state);

            // Initialize config state
            let config_path = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("swictation")
                .join("config.toml");
            let config_state = ConfigState {
                config_path: Mutex::new(config_path),
            };
            app.manage(config_state);

            // Start metrics socket listener using correct async implementation
            let mut metrics_socket = MetricsSocket::new();
            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                if let Err(e) = metrics_socket.listen(app_handle).await {
                    log::error!("Metrics socket error: {}", e);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide window instead of closing to keep app running in tray
                // This is standard tray app behavior on all platforms
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_recent_sessions,
            commands::get_session_count,
            commands::get_session_details,
            commands::search_transcriptions,
            commands::get_lifetime_stats,
            commands::toggle_recording,
            commands::get_connection_status,
            commands::reset_database,
            // Corrections commands
            commands::corrections::learn_correction,
            commands::corrections::get_corrections,
            commands::corrections::delete_correction,
            commands::corrections::update_correction,
            commands::corrections::extract_corrections_diff,
            // Config commands
            commands::config::get_daemon_config,
            commands::config::update_daemon_config,
            commands::config::update_phonetic_threshold,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
