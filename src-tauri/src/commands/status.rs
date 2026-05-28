use tauri::State;

use crate::domain::state::SharedState;
use crate::services::logging::{format_backend_event, log_backend, now_ms, push_diagnostic};

#[tauri::command]
pub fn get_status(state: State<'_, SharedState>) -> Result<serde_json::Value, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let devices: Vec<String> = state.discovered.keys().cloned().collect();
    let peer_transport = state.transport_status.clone();
    let visibility_age_ms = now_ms().saturating_sub(state.last_visibility_report_ms);
    let auth_age_ms = if state.last_auth_success_ms == 0 {
        u64::MAX
    } else {
        now_ms().saturating_sub(state.last_auth_success_ms)
    };
    let authenticated_peer_count = state
        .transport_status
        .values()
        .filter(|status| status.contains("authenticated"))
        .count();
    Ok(serde_json::json!({
        "status": if !devices.is_empty() { "connected" } else { "searching" },
        "sync_enabled": state.sync_enabled,
        "paired": !state.settings.trusted_peers.is_empty(),
        "devices": devices,
        "peer_transport": peer_transport,
        "runtime": {
            "is_app_foreground": state.is_app_foreground,
            "visibility_report_age_ms": visibility_age_ms,
            "background_mode_enabled": state.settings.background_mode_enabled,
            "last_auth_age_ms": auth_age_ms,
            "stale_peers_pruned": state.stale_peers_pruned,
            "authenticated_peer_count": authenticated_peer_count
        },
        "sync_stats": {
            "sent": state.sync_sent_count,
            "received": state.sync_received_count,
            "dropped": state.sync_dropped_count,
            "stale_rejected": state.sync_rejected_stale_count
        },
        "pending_requests": state.pending_pairing_requests.keys().cloned().collect::<Vec<String>>(),
        "outgoing_requests": state.outgoing_pairing_requests.keys().cloned().collect::<Vec<String>>(),
        "trusted_peers": state.settings.trusted_peers.keys().cloned().collect::<Vec<String>>()
    }))
}

#[tauri::command]
pub fn report_app_visibility(
    is_foreground: bool,
    state: State<'_, SharedState>,
) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.is_app_foreground = is_foreground;
    s.last_visibility_report_ms = now_ms();
    let event = format_backend_event(
        "INFO",
        "APP_VISIBILITY",
        if is_foreground {
            "foreground"
        } else {
            "background"
        },
    );
    log_backend(&event);
    push_diagnostic(&mut s, event);
    Ok(())
}

#[tauri::command]
pub fn get_diagnostics(state: State<'_, SharedState>) -> Result<Vec<String>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.diagnostic_events.iter().cloned().collect())
}
