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

#[cfg(any(target_os = "windows", target_os = "linux"))]
use tauri_plugin_autostart::ManagerExt;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use crate::services::tray::{refresh_tray_state, setup_desktop_tray};

#[cfg(target_os = "windows")]
const AUTOSTART_EVENT: &str = "WINDOWS_AUTOSTART_APPLY";

#[cfg(target_os = "linux")]
const AUTOSTART_EVENT: &str = "LINUX_AUTOSTART_APPLY";

pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let settings = load_settings(app);
    let host_name = whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
    let device_name = effective_device_name(&settings, &host_name);

    let state: tauri::State<'_, SharedState> = app.state();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.settings = settings.clone();
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

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        let desktop_start_on_login = settings.windows_start_on_login;
        setup_desktop_tray(app, app.state::<SharedState>().inner().clone())?;

        let autostart_result = if desktop_start_on_login {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };

        match autostart_result {
            Ok(_) => {
                log_backend_event(
                    "SUCCESS",
                    AUTOSTART_EVENT,
                    if desktop_start_on_login {
                        "enabled"
                    } else {
                        "disabled"
                    },
                );
            }
            Err(err) => {
                log_backend_event(
                    "FAILED",
                    AUTOSTART_EVENT,
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
    let app_for_transport = app.clone();

    #[cfg(target_os = "android")]
    let local_command_state = discovery_state.clone();

    thread::spawn(move || {
        start_mdns_discovery(discovery_state.clone(), discovery_name.clone());
        start_udp_fallback_discovery(discovery_state.clone(), discovery_name);
        start_transport_server(discovery_state.clone(), app_for_transport);
        start_transport_handshake_loop(discovery_state);
    });

    #[cfg(target_os = "android")]
    crate::network::transport::server::start_local_command_server(local_command_state);

    log_backend_event("SUCCESS", "APP_STARTUP", "ClipSync v0.1 started");
    Ok(())
}
