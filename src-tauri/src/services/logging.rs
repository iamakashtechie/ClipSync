use std::time::SystemTime;

use crate::domain::state::{AppState, SharedState};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn log_backend(message: &str) {
    println!("[ClipSync/Backend][{}] {}", now_ms(), message);
}

pub fn format_backend_event(level: &str, event: &str, details: &str) -> String {
    format!("[{}] {} :: {}", level, event, details)
}

pub fn log_backend_event(level: &str, event: &str, details: &str) {
    log_backend(&format_backend_event(level, event, details));
}

pub fn push_diagnostic(state: &mut AppState, event: String) {
    const MAX_EVENTS: usize = 120;
    state.diagnostic_events.push_back(event);
    if state.diagnostic_events.len() > MAX_EVENTS {
        state.diagnostic_events.pop_front();
    }
}

pub fn log_backend_event_with_state(state: &SharedState, level: &str, event: &str, details: &str) {
    let message = format_backend_event(level, event, details);
    log_backend(&message);
    if let Ok(mut s) = state.lock() {
        push_diagnostic(&mut s, message);
    }
}
