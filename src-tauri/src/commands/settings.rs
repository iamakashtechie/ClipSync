use tauri::State;

#[cfg(target_os = "windows")]
use tauri_plugin_autostart::ManagerExt;

use crate::domain::models::AppSettings;
use crate::domain::state::SharedState;
use crate::services::logging::{format_backend_event, log_backend, push_diagnostic};
use crate::services::settings::{effective_device_name, save_settings_to_disk};

#[cfg(target_os = "windows")]
use crate::services::tray::refresh_tray_state;

#[tauri::command]
pub fn toggle_sync(
    enabled: bool,
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if enabled && !s.paired {
            let event = format_backend_event("FAILED", "SYNC_TOGGLE", "blocked: pairing required");
            log_backend(&event);
            push_diagnostic(&mut s, event);
            return Err("Pairing required before enabling sync".to_string());
        }
        s.sync_enabled = enabled;
        let event = format_backend_event(
            "SUCCESS",
            "SYNC_TOGGLE",
            if enabled { "enabled" } else { "disabled" },
        );
        log_backend(&event);
        push_diagnostic(&mut s, event);
    }
    #[cfg(target_os = "windows")]
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
    pairing_code: String,
    device_name_override: String,
    background_mode_enabled: bool,
    windows_start_on_login: bool,
    dev_mode_enabled: bool,
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if !pairing_code.chars().all(|c| c.is_ascii_digit()) || pairing_code.len() != 4 {
        return Err("Pairing code must be exactly 4 digits".to_string());
    }

    let settings = AppSettings {
        max_image_size_kb,
        pairing_code,
        device_name_override,
        background_mode_enabled,
        windows_start_on_login,
        dev_mode_enabled,
    };

    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.settings = settings.clone();
        let host_name =
            whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
        s.device_name = effective_device_name(&s.settings, &host_name);
        s.transport_status.clear();
        s.paired = false;
        s.sync_enabled = false;

        #[cfg(target_os = "windows")]
        {
            let apply_result = if s.settings.windows_start_on_login {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            };

            if let Err(err) = apply_result {
                let event = format_backend_event(
                    "FAILED",
                    "WINDOWS_AUTOSTART_APPLY",
                    &format!("failed to apply startup setting: {err}"),
                );
                log_backend(&event);
                push_diagnostic(&mut s, event);
            }
        }
    }

    let result = save_settings_to_disk(&app, &settings);

    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        match &result {
            Ok(_) => {
                let event = format_backend_event(
                    "SUCCESS",
                    "SAVE_SETTINGS",
                    &format!(
                        "max_image_size_kb={} device_name={} background_mode_enabled={} windows_start_on_login={}",
                        s.settings.max_image_size_kb,
                        s.device_name,
                        s.settings.background_mode_enabled,
                        s.settings.windows_start_on_login
                    ),
                );
                log_backend(&event);
                push_diagnostic(&mut s, event);
            }
            Err(err) => {
                let event = format_backend_event("FAILED", "SAVE_SETTINGS", err);
                log_backend(&event);
                push_diagnostic(&mut s, event);
            }
        }
    }

    #[cfg(target_os = "windows")]
    refresh_tray_state(&app, state.inner());
    result
}

#[tauri::command]
pub fn validate_pairing(
    code: String,
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let ok = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let ok = !s.settings.pairing_code.is_empty() && s.settings.pairing_code == code;
        s.paired = ok;
        if !ok {
            s.sync_enabled = false;
        }
        let event = if ok {
            format_backend_event("SUCCESS", "VALIDATE_PAIRING", "pairing verified")
        } else {
            format_backend_event("FAILED", "VALIDATE_PAIRING", "pairing mismatch")
        };
        log_backend(&event);
        push_diagnostic(&mut s, event);
        ok
    };
    #[cfg(target_os = "windows")]
    refresh_tray_state(&app, state.inner());
    Ok(ok)
}
