use std::time::Duration;

use crate::domain::state::SharedState;
use crate::services::logging::{format_backend_event, log_backend, now_ms, push_diagnostic};

pub fn start_transport_handshake_loop(state: SharedState) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(mut s) = state.lock() {
                let now = now_ms();
                let ttl_ms = 30_000_u64;
                let stale_names: Vec<String> = s
                    .discovered_last_seen_ms
                    .iter()
                    .filter_map(|(name, seen)| {
                        if now.saturating_sub(*seen) > ttl_ms {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                for name in stale_names {
                    s.discovered.remove(&name);
                    s.discovered_last_seen_ms.remove(&name);
                    s.transport_status.remove(&name);
                    s.stale_peers_pruned += 1;
                    let event =
                        format_backend_event("INFO", "PEER_PRUNED", &format!("peer={}", name));
                    log_backend(&event);
                    push_diagnostic(&mut s, event);
                }
            }

            tokio::time::sleep(Duration::from_secs(4)).await;
        }
    });
}
