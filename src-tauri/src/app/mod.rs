use crate::config::CLIPSYNC_WS_PORT;
use crate::domain::state::SharedState;
use crate::network::discovery::{start_mdns_discovery, start_udp_fallback_discovery};
use crate::network::transport::{start_transport_handshake_loop, start_transport_server};
use crate::services::logging::{
    format_backend_event, log_backend, log_backend_event, now_ms, push_diagnostic,
};
use crate::services::settings::{effective_device_name, load_settings};
use tauri::Manager;
use std::thread;

#[cfg(target_os = "windows")]
use tauri_plugin_autostart::ManagerExt;

#[cfg(target_os = "windows")]
use crate::services::tray::{refresh_tray_state, setup_windows_tray};

pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let settings = load_settings(app);
    let windows_start_on_login = settings.windows_start_on_login;
    let host_name = whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
    let device_name = effective_device_name(&settings, &host_name);

    let state: tauri::State<'_, SharedState> = app.state();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.settings = settings;
    s.device_name = device_name.clone();
    s.last_visibility_report_ms = now_ms();
    let startup_event = format_backend_event(
        "INFO",
        "APP_STARTUP",
        &format!("device_name={} ws_port={}", s.device_name, CLIPSYNC_WS_PORT),
    );
    log_backend(&startup_event);
    push_diagnostic(&mut s, startup_event);
    drop(s);

    #[cfg(target_os = "windows")]
    {
        setup_windows_tray(app, app.state::<SharedState>().inner().clone())?;

        let autostart_result = if windows_start_on_login {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };

        match autostart_result {
            Ok(_) => {
                log_backend_event(
                    "SUCCESS",
                    "WINDOWS_AUTOSTART_APPLY",
                    if windows_start_on_login {
                        "enabled"
                    } else {
                        "disabled"
                    },
                );
            }
            Err(err) => {
                log_backend_event(
                    "FAILED",
                    "WINDOWS_AUTOSTART_APPLY",
                    &format!("failed to apply startup setting: {err}"),
                );
            }
        }

        let app_handle = app.clone();
        let tray_state = app.state::<SharedState>().inner().clone();
        tauri::async_runtime::spawn(async move {
            loop {
                refresh_tray_state(&app_handle, &tray_state);
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        });
    }

    let discovery_state: SharedState = app.state::<SharedState>().inner().clone();
    let discovery_name = device_name.clone();
    thread::spawn(move || {
        start_mdns_discovery(discovery_state.clone(), discovery_name.clone());
        start_udp_fallback_discovery(discovery_state.clone(), discovery_name);
        start_transport_server(discovery_state.clone());
        start_transport_handshake_loop(discovery_state);
    });

    log_backend_event("SUCCESS", "APP_STARTUP", "ClipSync v0.1 started");
    Ok(())
}
