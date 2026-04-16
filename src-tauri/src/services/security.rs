use std::net::IpAddr;

use crate::domain::state::AppState;

pub fn is_private_or_loopback(ip: &str) -> bool {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        Ok(IpAddr::V6(v6)) => {
            v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local()
        }
        Err(_) => false,
    }
}

pub fn should_accept_incoming(state: &mut AppState, sender_id: &str, timestamp_ms: u64) -> bool {
    if timestamp_ms > state.last_applied_timestamp_ms {
        state.last_applied_timestamp_ms = timestamp_ms;
        state.last_applied_sender = sender_id.to_string();
        return true;
    }

    if timestamp_ms == state.last_applied_timestamp_ms
        && sender_id > state.last_applied_sender.as_str()
    {
        state.last_applied_sender = sender_id.to_string();
        return true;
    }

    false
}
