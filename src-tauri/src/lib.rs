#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod config;
mod domain;
mod network;
mod services;

use std::sync::{Arc, Mutex};

use commands::{
    consume_remote_image, consume_remote_text, get_diagnostics, get_settings, get_status,
    push_local_image_payload, push_local_text_clipboard, report_app_visibility, save_settings,
    toggle_sync, validate_pairing,
};
use domain::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_state = Arc::new(Mutex::new(AppState::default()));

    tauri::Builder::default()
        .manage(initial_state)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
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
            validate_pairing
        ])
        .setup(|app| {
            app::initialize(app.handle()).map_err(|e| e.into())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
