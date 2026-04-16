pub mod discovery;
pub mod transport;

use crate::domain::state::SharedState;
use crate::services::logging::now_ms;

pub fn upsert_discovered_device(state: &SharedState, device_name: String, addr: String) {
    if let Ok(mut s) = state.lock() {
        s.discovered.insert(device_name.clone(), addr);
        s.discovered_last_seen_ms.insert(device_name, now_ms());
    }
}

pub fn set_transport_status(state: &SharedState, peer: String, status: String) {
    if let Ok(mut s) = state.lock() {
        s.transport_status.insert(peer, status);
    }
}
