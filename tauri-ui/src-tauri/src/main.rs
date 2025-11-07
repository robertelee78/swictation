// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod database;
mod models;
mod socket;
mod utils;

use commands::AppState;
use database::Database;
use socket::SocketConnection;
use std::sync::{Arc, Mutex};
use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, WindowEvent,
};

fn main() {
    // Initialize logger
    env_logger::init();

    // Create system tray menu
    let show_metrics = CustomMenuItem::new("show_metrics".to_string(), "Show Metrics");
    let toggle_recording = CustomMenuItem::new("toggle_recording".to_string(), "Toggle Recording");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show_metrics)
        .add_item(toggle_recording)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(handle_system_tray_event)
        .setup(|app| {
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
            let socket_path = utils::get_default_socket_path();
            let socket = Arc::new(SocketConnection::new(
                socket_path.clone(),
                app.handle(),
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
        .on_window_event(|event| {
            if let WindowEvent::CloseRequested { api, .. } = event.event() {
                // Prevent window close, hide instead
                event.window().hide().unwrap();
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

/// Handle system tray events
fn handle_system_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "show_metrics" => {
                // Show main window
                if let Some(window) = app.get_window("main") {
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
            }
            "toggle_recording" => {
                // Emit toggle event to frontend
                app.emit_all("toggle-recording-requested", ()).ok();
            }
            "quit" => {
                // Quit application
                std::process::exit(0);
            }
            _ => {}
        },
        SystemTrayEvent::LeftClick { .. } => {
            // Show window on left click
            if let Some(window) = app.get_window("main") {
                window.show().unwrap();
                window.set_focus().unwrap();
            }
        }
        _ => {}
    }
}
