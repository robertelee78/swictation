// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod database;
mod models;
mod socket;
mod utils;

use commands::AppState;
use database::Database;
use image::GenericImageView;
use socket::SocketConnection;
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

fn main() {
    // Initialize logger
    env_logger::init();

    tauri::Builder::default()
        .setup(|app| {
            // Only create tray icon if not disabled (e.g., when launched from QT tray on Sway)
            if std::env::var("SWICTATION_NO_TRAY").is_err() {
                // Create menu items
                let show_metrics = MenuItemBuilder::with_id("show_metrics", "Show Metrics").build(app)?;
                let toggle_recording = MenuItemBuilder::with_id("toggle_recording", "Toggle Recording").build(app)?;
                let separator = PredefinedMenuItem::separator(app)?;
                let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

                // Build menu
                let menu = Menu::with_items(app, &[&show_metrics, &toggle_recording, &separator, &quit])?;

                // Load tray icon from embedded bytes (for SNI compatibility)
                let icon_bytes = include_bytes!("../icons/tray-48.png");
                let img = image::load_from_memory(icon_bytes)?;
                let rgba = img.to_rgba8();
                let (width, height) = img.dimensions();
                let tray_icon = Image::new_owned(rgba.into_raw(), width, height);

                // Build and configure tray icon with template mode for better SNI compatibility
                let _tray = TrayIconBuilder::new()
                    .icon(tray_icon)
                    .icon_as_template(true)
                    .menu(&menu)
                    .show_menu_on_left_click(false)
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
                        // Emit toggle event to frontend
                        let _ = app.emit("toggle-recording-requested", ());
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
                        // Left click: Toggle recording (same as Qt tray and hotkey)
                        let app = tray.app_handle();
                        let _ = app.emit("toggle-recording-requested", ());
                    }
                    TrayIconEvent::Click {
                        button: MouseButton::Middle,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        // Middle click: Toggle window visibility (same as Qt tray)
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

            // Create socket connection
            let socket_path = crate::socket::get_metrics_socket_path()
                .to_string_lossy()
                .to_string();
            let socket = Arc::new(SocketConnection::new(
                socket_path.clone(),
                app.handle().clone(),
            ));

            // Create app state
            let state = AppState {
                db: Mutex::new(db.unwrap_or_else(|| {
                    // Fallback: try to create database if it doesn't exist
                    Database::new(&db_path).expect("Failed to create database")
                })),
                socket: socket.clone(),
            };

            app.manage(state);

            // Start socket listener AFTER app is managed (within Tauri's async context)
            tauri::async_runtime::spawn(async move {
                socket.start_listener().await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Prevent window close, hide instead
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_recent_sessions,
            commands::get_session_details,
            commands::search_transcriptions,
            commands::get_lifetime_stats,
            commands::toggle_recording,
            commands::get_connection_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
