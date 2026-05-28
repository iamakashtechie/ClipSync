use tauri::State;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use tauri_plugin_autostart::ManagerExt;

use crate::domain::models::AppSettings;
use crate::domain::state::SharedState;
use crate::services::logging::{format_backend_event, log_backend, push_diagnostic};
use crate::services::settings::{effective_device_name, save_settings_to_disk};

#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::services::tray::refresh_tray_state;

use crate::network::transport::{send_pairing_request, send_pairing_response};

#[cfg(target_os = "windows")]
const AUTOSTART_EVENT: &str = "WINDOWS_AUTOSTART_APPLY";

#[cfg(target_os = "linux")]
const AUTOSTART_EVENT: &str = "LINUX_AUTOSTART_APPLY";

#[tauri::command]
pub fn toggle_sync(
    enabled: bool,
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.sync_enabled = enabled;
        let event = format_backend_event(
            "SUCCESS",
            "SYNC_TOGGLE",
            if enabled { "enabled" } else { "disabled" },
        );
        log_backend(&event);
        push_diagnostic(&mut s, event);
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    refresh_tray_state(&app, state.inner());
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, SharedState>) -> Result<AppSettings, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
pub fn save_settings(
    max_image_size_kb: u32,
    device_name_override: String,
    background_mode_enabled: bool,
    windows_start_on_login: bool,
    dev_mode_enabled: bool,
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {

    let trusted_peers = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.settings.trusted_peers.clone()
    };

    let settings = AppSettings {
        max_image_size_kb,
        trusted_peers,
        device_name_override,
        background_mode_enabled,
        windows_start_on_login,
        dev_mode_enabled,
    };

    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.settings = settings.clone();
        let host_name = whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
        s.device_name = effective_device_name(&s.settings, &host_name);

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            let apply_result = if s.settings.windows_start_on_login {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            };

            if let Err(err) = apply_result {
                let event = format_backend_event(
                    "FAILED",
                    AUTOSTART_EVENT,
                    &format!("failed to apply startup setting: {err}"),
                );
                log_backend(&event);
                push_diagnostic(&mut s, event);
            }
        }
    }

    let result = save_settings_to_disk(&app, &settings);

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    refresh_tray_state(&app, state.inner());
    result
}

#[tauri::command]
pub async fn request_connection(
    peer_name: String,
    state: State<'_, SharedState>,
    _app: tauri::AppHandle,
) -> Result<(), String> {
    let token = uuid::Uuid::new_v4().to_string();
    let addr = {
        let mut s = state.inner().lock().map_err(|e| e.to_string())?;
        let addr = s.discovered.get(&peer_name).cloned();
        if addr.is_some() {
            s.outgoing_pairing_requests.insert(peer_name.clone(), token.clone());
        }
        addr
    };

    if let Some(addr) = addr {
        send_pairing_request(peer_name, addr, token, state.inner().clone()).await;
        Ok(())
    } else {
        Err("Device not found on network".to_string())
    }
}

#[tauri::command]
pub fn approve_connection(
    peer_name: String,
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let token = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.pending_pairing_requests.remove(&peer_name)
    };

    if let Some(t) = token {
        let (settings, addr) = {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.settings.trusted_peers.insert(peer_name.clone(), t);
            let addr = s.discovered.get(&peer_name).cloned();
            (s.settings.clone(), addr)
        };
        let _ = save_settings_to_disk(&app, &settings);
        
        if let Some(addr) = addr {
            let state_clone = state.inner().clone();
            tauri::async_runtime::spawn(async move {
                send_pairing_response(peer_name, addr, true, state_clone).await;
            });
        }
        
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        refresh_tray_state(&app, state.inner());
        
        Ok(())
    } else {
        Err("No pending request for this device".to_string())
    }
}

#[tauri::command]
pub fn reject_connection(
    peer_name: String,
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let token = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.pending_pairing_requests.remove(&peer_name)
    };

    if token.is_some() {
        let addr = {
            let s = state.lock().map_err(|e| e.to_string())?;
            s.discovered.get(&peer_name).cloned()
        };
        if let Some(addr) = addr {
            let state_clone = state.inner().clone();
            tauri::async_runtime::spawn(async move {
                send_pairing_response(peer_name, addr, false, state_clone).await;
            });
        }
    }
    
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    refresh_tray_state(&app, state.inner());
    
    Ok(())
}
