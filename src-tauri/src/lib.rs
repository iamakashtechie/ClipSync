#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod config;
mod domain;
mod network;
mod services;

use std::sync::{Arc, Mutex};

use commands::{
    approve_connection, consume_remote_image, consume_remote_text, get_diagnostics, get_settings,
    get_status, push_local_image_payload, push_local_text_clipboard, reject_connection,
    report_app_visibility, request_connection, save_settings, toggle_sync,
    read_clipboard_text, write_clipboard_text,
};
use domain::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_state = Arc::new(Mutex::new(AppState::default()));

    let mut builder = tauri::Builder::default()
        .manage(initial_state)
        .plugin(tauri_plugin_opener::init());

    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        builder = builder.plugin(tauri_plugin_clipboard_manager::init());
    }

    builder = builder.invoke_handler(tauri::generate_handler![
            get_status,
            report_app_visibility,
            get_diagnostics,
            consume_remote_text,
            consume_remote_image,
            push_local_text_clipboard,
            push_local_image_payload,
            toggle_sync,
            get_settings,
            save_settings,
            request_connection,
            approve_connection,
            reject_connection,
            read_clipboard_text,
            write_clipboard_text
        ]);

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));
    }

    builder
        .on_window_event(|window, event| {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| app::initialize(app.handle()).map_err(|e| e.into()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
