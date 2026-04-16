use crate::config::CLIPSYNC_WS_PORT;
use crate::domain::state::SharedState;
use crate::network::discovery::{start_mdns_discovery, start_udp_fallback_discovery};
use crate::network::transport::{start_transport_handshake_loop, start_transport_server};
use crate::services::logging::{format_backend_event, log_backend, log_backend_event, now_ms, push_diagnostic};
use crate::services::settings::{effective_device_name, load_settings};
use tauri::Manager;

pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    let settings = load_settings(app);
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

    let state_clone: SharedState = app.state::<SharedState>().inner().clone();
    start_mdns_discovery(state_clone.clone(), device_name.clone());
    start_udp_fallback_discovery(state_clone, device_name);

    let transport_state: SharedState = app.state::<SharedState>().inner().clone();
    start_transport_server(transport_state.clone());
    start_transport_handshake_loop(transport_state);

    log_backend_event("SUCCESS", "APP_STARTUP", "ClipSync v0.1 started");
    Ok(())
}
