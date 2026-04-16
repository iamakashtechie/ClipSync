use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::config::CLIPSYNC_WS_PORT;
use crate::domain::models::{IncomingImage, TransportMessage};
use crate::domain::state::SharedState;
use crate::network::set_transport_status;
use crate::services::hashing::remember_hash;
use crate::services::logging::{
    format_backend_event, log_backend, log_backend_event, log_backend_event_with_state,
    push_diagnostic,
};
use crate::services::security::should_accept_incoming;

pub async fn handle_incoming_transport_connection(
    stream: tokio::net::TcpStream,
    state: SharedState,
) {
    let Ok(peer_addr) = stream.peer_addr() else {
        return;
    };

    let Ok(mut ws_stream) = accept_async(stream).await else {
        log_backend_event(
            "FAILED",
            "INBOUND_TRANSPORT_ACCEPT",
            &format!("peer={} accept websocket failed", peer_addr),
        );
        return;
    };

    let next_msg = tokio::time::timeout(Duration::from_secs(4), ws_stream.next()).await;
    let incoming = match next_msg {
        Ok(Some(Ok(msg))) => msg,
        _ => {
            log_backend_event(
                "FAILED",
                "INBOUND_HELLO",
                &format!("peer={} timeout or invalid first frame", peer_addr),
            );
            let _ = ws_stream.close(None).await;
            return;
        }
    };

    let Message::Text(payload) = incoming else {
        log_backend_event(
            "FAILED",
            "INBOUND_HELLO",
            &format!("peer={} first frame not text", peer_addr),
        );
        let _ = ws_stream.close(None).await;
        return;
    };

    let hello = serde_json::from_str::<TransportMessage>(&payload);
    let (remote_name, remote_code, local_name, local_code) = match hello {
        Ok(TransportMessage::Hello {
            device_name,
            pairing_code,
        }) => {
            let Ok(s) = state.lock() else {
                return;
            };
            (
                device_name,
                pairing_code,
                s.device_name.clone(),
                s.settings.pairing_code.clone(),
            )
        }
        _ => {
            log_backend_event(
                "FAILED",
                "INBOUND_HELLO_PARSE",
                &format!("peer={} invalid hello payload", peer_addr),
            );
            let _ = ws_stream.close(None).await;
            return;
        }
    };

    let accepted = !local_code.is_empty() && local_code == remote_code;
    let reason = if accepted {
        "authenticated".to_string()
    } else {
        "pairing_code_mismatch".to_string()
    };

    let ack = TransportMessage::HelloAck {
        device_name: local_name,
        accepted,
        reason: reason.clone(),
    };
    if let Ok(ack_text) = serde_json::to_string(&ack) {
        let _ = ws_stream.send(Message::Text(ack_text.into())).await;
    }

    let peer_label = format!("{} ({})", remote_name, peer_addr.ip());
    if accepted {
        log_backend_event_with_state(
            &state,
            "SUCCESS",
            "PAIRING_AUTH_INBOUND",
            &format!("peer={} authenticated", peer_label),
        );
        set_transport_status(&state, peer_label, "authenticated (inbound)".to_string());

        let maybe_sync = tokio::time::timeout(Duration::from_secs(2), ws_stream.next()).await;
        if let Ok(Some(Ok(Message::Text(payload)))) = maybe_sync {
            let parsed = serde_json::from_str::<TransportMessage>(&payload);
            match parsed {
                Ok(TransportMessage::SyncText {
                    sender_id,
                    timestamp_ms,
                    message_hash,
                    text,
                }) => {
                    if let Ok(mut s) = state.lock() {
                        if s.recent_hashes.contains(&message_hash) {
                            s.sync_dropped_count += 1;
                            let event = format_backend_event(
                                "FAILED",
                                "TEXT_RECEIVED",
                                &format!("duplicate hash={} sender={}", message_hash, sender_id),
                            );
                            log_backend(&event);
                            push_diagnostic(&mut s, event);
                        } else if !should_accept_incoming(&mut s, &sender_id, timestamp_ms) {
                            s.sync_rejected_stale_count += 1;
                            let event = format_backend_event(
                                "FAILED",
                                "TEXT_RECEIVED",
                                &format!("stale sender={} timestamp={}", sender_id, timestamp_ms),
                            );
                            log_backend(&event);
                            push_diagnostic(&mut s, event);
                        } else {
                            remember_hash(&mut s, message_hash);
                            s.pending_remote_text = Some(text);
                            s.sync_received_count += 1;
                            let event = format_backend_event(
                                "SUCCESS",
                                "TEXT_RECEIVED",
                                &format!("sender={} timestamp={}", sender_id, timestamp_ms),
                            );
                            log_backend(&event);
                            push_diagnostic(&mut s, event);
                        }
                        s.transport_status.insert(
                            format!("{} ({})", sender_id, peer_addr.ip()),
                            "authenticated (inbound) + synced text".to_string(),
                        );
                    }
                }
                Ok(TransportMessage::SyncImage {
                    sender_id,
                    timestamp_ms,
                    message_hash,
                    mime_type,
                    image_base64,
                }) => {
                    if let Ok(mut s) = state.lock() {
                        if s.recent_hashes.contains(&message_hash) {
                            s.sync_dropped_count += 1;
                            let event = format_backend_event(
                                "FAILED",
                                "IMAGE_RECEIVED",
                                &format!("duplicate hash={} sender={}", message_hash, sender_id),
                            );
                            log_backend(&event);
                            push_diagnostic(&mut s, event);
                        } else if !should_accept_incoming(&mut s, &sender_id, timestamp_ms) {
                            s.sync_rejected_stale_count += 1;
                            let event = format_backend_event(
                                "FAILED",
                                "IMAGE_RECEIVED",
                                &format!("stale sender={} timestamp={}", sender_id, timestamp_ms),
                            );
                            log_backend(&event);
                            push_diagnostic(&mut s, event);
                        } else {
                            remember_hash(&mut s, message_hash);
                            s.pending_remote_image = Some(IncomingImage {
                                mime_type,
                                image_base64,
                            });
                            s.sync_received_count += 1;
                            let event = format_backend_event(
                                "SUCCESS",
                                "IMAGE_RECEIVED",
                                &format!("sender={} timestamp={}", sender_id, timestamp_ms),
                            );
                            log_backend(&event);
                            push_diagnostic(&mut s, event);
                        }
                        s.transport_status.insert(
                            format!("{} ({})", sender_id, peer_addr.ip()),
                            "authenticated (inbound) + synced image".to_string(),
                        );
                    }
                }
                _ => {
                    log_backend_event_with_state(
                        &state,
                        "FAILED",
                        "INBOUND_SYNC_PARSE",
                        &format!("peer={} payload parse failed", peer_addr),
                    );
                }
            }
        }
    } else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "PAIRING_AUTH_INBOUND",
            &format!("peer={} pairing mismatch", peer_label),
        );
        set_transport_status(&state, peer_label, "rejected: pairing mismatch".to_string());
    }

    let _ = ws_stream.close(None).await;
}

pub fn start_transport_server(state: SharedState) {
    tauri::async_runtime::spawn(async move {
        let Ok(listener) = TcpListener::bind(("0.0.0.0", CLIPSYNC_WS_PORT)).await else {
            eprintln!("Transport server bind failed on port {}", CLIPSYNC_WS_PORT);
            return;
        };

        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };

            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                handle_incoming_transport_connection(stream, state_clone).await;
            });
        }
    });
}
