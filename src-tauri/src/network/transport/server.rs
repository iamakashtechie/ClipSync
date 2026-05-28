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
    app: tauri::AppHandle,
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
                "INBOUND_PAYLOAD",
                &format!("peer={} timeout or invalid first frame", peer_addr),
            );
            let _ = ws_stream.close(None).await;
            return;
        }
    };

    let Message::Text(payload) = incoming else {
        log_backend_event(
            "FAILED",
            "INBOUND_PAYLOAD",
            &format!("peer={} first frame not text", peer_addr),
        );
        let _ = ws_stream.close(None).await;
        return;
    };

    let msg = serde_json::from_str::<TransportMessage>(&payload);
    let msg = match msg {
        Ok(m) => m,
        _ => {
            log_backend_event(
                "FAILED",
                "INBOUND_PARSE",
                &format!("peer={} invalid payload", peer_addr),
            );
            let _ = ws_stream.close(None).await;
            return;
        }
    };

    match msg {
        TransportMessage::PairingRequest { device_name, token } => {
            let details = format!("peer={} requested pairing", device_name);
            let event = format_backend_event("INFO", "PAIRING_REQUEST_RCVD", &details);
            log_backend(&event);
            if let Ok(mut s) = state.lock() {
                s.pending_pairing_requests.insert(device_name.clone(), token);
                push_diagnostic(&mut s, event);
            }
        }
        TransportMessage::PairingResponse { device_name, accepted } => {
            let details = format!("peer={} pairing accepted={}", device_name, accepted);
            let event = format_backend_event("INFO", "PAIRING_RESPONSE_RCVD", &details);
            log_backend(&event);
            if let Ok(mut s) = state.lock() {
                push_diagnostic(&mut s, event);
                if accepted {
                    if let Some(token) = s.outgoing_pairing_requests.remove(&device_name) {
                        s.settings.trusted_peers.insert(device_name.clone(), token);
                        let settings = s.settings.clone();
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = crate::services::settings::save_settings_to_disk(&app_clone, &settings);
                        });
                    }
                } else {
                    s.outgoing_pairing_requests.remove(&device_name);
                }
            }
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            crate::services::tray::refresh_tray_state(&app, &state);
        }
        TransportMessage::Hello { device_name, token } => {
            let (local_name, accepted) = {
                let Ok(s) = state.lock() else { return; };
                let auth = s.settings.trusted_peers.get(&device_name).map(|t| t == &token).unwrap_or(false);
                (s.device_name.clone(), auth)
            };

            let reason = if accepted {
                "authenticated".to_string()
            } else {
                "token_mismatch".to_string()
            };

            let ack = TransportMessage::HelloAck {
                device_name: local_name,
                accepted,
                reason: reason.clone(),
            };
            if let Ok(ack_text) = serde_json::to_string(&ack) {
                let _ = ws_stream.send(Message::Text(ack_text.into())).await;
            }

            let peer_label = format!("{} ({})", device_name, peer_addr.ip());
            if !accepted {
                log_backend_event_with_state(
                    &state,
                    "FAILED",
                    "PAIRING_AUTH_INBOUND",
                    &format!("peer={} auth mismatch", peer_label),
                );
                set_transport_status(&state, peer_label, "rejected: token mismatch".to_string());
                let _ = ws_stream.close(None).await;
                return;
            }

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
                    Ok(TransportMessage::SyncText { sender_id, timestamp_ms, message_hash, text }) => {
                        if let Ok(mut s) = state.lock() {
                            if s.recent_hashes.contains(&message_hash) {
                                s.sync_dropped_count += 1;
                            } else if !should_accept_incoming(&mut s, &sender_id, timestamp_ms) {
                                s.sync_rejected_stale_count += 1;
                            } else {
                                remember_hash(&mut s, message_hash);
                                s.pending_remote_text = Some(text);
                                s.sync_received_count += 1;
                            }
                            s.transport_status.insert(
                                format!("{} ({})", sender_id, peer_addr.ip()),
                                "authenticated (inbound) + synced text".to_string(),
                            );
                        }
                    }
                    Ok(TransportMessage::SyncImage { sender_id, timestamp_ms, message_hash, mime_type, image_base64 }) => {
                        if let Ok(mut s) = state.lock() {
                            if s.recent_hashes.contains(&message_hash) {
                                s.sync_dropped_count += 1;
                            } else if !should_accept_incoming(&mut s, &sender_id, timestamp_ms) {
                                s.sync_rejected_stale_count += 1;
                            } else {
                                remember_hash(&mut s, message_hash);
                                s.pending_remote_image = Some(IncomingImage {
                                    mime_type,
                                    image_base64,
                                });
                                s.sync_received_count += 1;
                            }
                            s.transport_status.insert(
                                format!("{} ({})", sender_id, peer_addr.ip()),
                                "authenticated (inbound) + synced image".to_string(),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    let _ = ws_stream.close(None).await;
}

pub fn start_transport_server(state: SharedState, app: tauri::AppHandle) {
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
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                handle_incoming_transport_connection(stream, state_clone, app_clone).await;
            });
        }
    });
}

#[cfg(target_os = "android")]
#[derive(serde::Deserialize)]
struct LocalCommandMessage {
    #[serde(rename = "type")]
    msg_type: String,
    text: Option<String>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    #[serde(rename = "imageBase64")]
    image_base64: Option<String>,
}

#[cfg(target_os = "android")]
pub fn start_local_command_server(state: SharedState) {
    tauri::async_runtime::spawn(async move {
        let Ok(listener) = TcpListener::bind(("127.0.0.1", 10191)).await else {
            eprintln!("Local command server bind failed on port 10191");
            return;
        };

        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                continue;
            };

            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buf = Vec::new();
                let mut chunk = [0u8; 8192];
                loop {
                    match socket.read(&mut chunk).await {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&chunk[..n]);
                            if buf.len() > 10_000_000 {
                                return;
                            }
                        }
                        Err(_) => return,
                    }
                }

                if let Ok(msg_str) = String::from_utf8(buf) {
                    if let Ok(msg) = serde_json::from_str::<LocalCommandMessage>(&msg_str) {
                        match msg.msg_type.as_str() {
                            "text" => {
                                if let Some(text) = msg.text {
                                    let _ = crate::commands::clipboard::push_local_text_clipboard_impl(text, state_clone);
                                }
                            }
                            "image" => {
                                if let (Some(mime), Some(base64)) = (msg.mime_type, msg.image_base64) {
                                    let _ = crate::commands::clipboard::push_local_image_payload_impl(mime, base64, state_clone);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            });
        }
    });
}
