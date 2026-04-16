use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::domain::models::TransportMessage;
use crate::domain::state::SharedState;
use crate::network::set_transport_status;
use crate::services::logging::{
    format_backend_event, log_backend, log_backend_event_with_state, now_ms, push_diagnostic,
};

pub async fn attempt_outbound_handshake(peer_name: String, addr: String, state: SharedState) {
    let (local_name, local_code) = {
        let Ok(s) = state.lock() else {
            return;
        };
        (s.device_name.clone(), s.settings.pairing_code.clone())
    };

    if local_name.is_empty() || local_code.is_empty() {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "HANDSHAKE_OUTBOUND",
            &format!("peer={} blocked: pairing config missing", peer_name),
        );
        set_transport_status(
            &state,
            peer_name,
            "blocked: configure 4-digit pairing code".to_string(),
        );
        return;
    }

    let ws_url = format!("ws://{addr}");
    let connect_result = tokio::time::timeout(Duration::from_secs(3), connect_async(ws_url)).await;
    let Ok(Ok((mut ws_stream, _))) = connect_result else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "HANDSHAKE_OUTBOUND",
            &format!("peer={} connect failed", peer_name),
        );
        set_transport_status(&state, peer_name, "connect failed".to_string());
        return;
    };

    let hello = TransportMessage::Hello {
        device_name: local_name,
        pairing_code: local_code,
    };
    let Ok(hello_text) = serde_json::to_string(&hello) else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "HANDSHAKE_OUTBOUND",
            &format!("peer={} local hello serialization failed", peer_name),
        );
        set_transport_status(&state, peer_name, "local serialization failed".to_string());
        let _ = ws_stream.close(None).await;
        return;
    };

    if ws_stream
        .send(Message::Text(hello_text.into()))
        .await
        .is_err()
    {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "HANDSHAKE_OUTBOUND",
            &format!("peer={} send hello failed", peer_name),
        );
        set_transport_status(&state, peer_name, "send hello failed".to_string());
        let _ = ws_stream.close(None).await;
        return;
    }

    let ack_msg = tokio::time::timeout(Duration::from_secs(4), ws_stream.next()).await;
    let Ok(Some(Ok(Message::Text(payload)))) = ack_msg else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "HANDSHAKE_OUTBOUND",
            &format!("peer={} ack timeout", peer_name),
        );
        set_transport_status(&state, peer_name, "ack timeout".to_string());
        let _ = ws_stream.close(None).await;
        return;
    };

    match serde_json::from_str::<TransportMessage>(&payload) {
        Ok(TransportMessage::HelloAck {
            device_name,
            accepted,
            reason,
        }) => {
            if accepted {
                log_backend_event_with_state(
                    &state,
                    "SUCCESS",
                    "HANDSHAKE_OUTBOUND",
                    &format!("peer={} authenticated with {}", peer_name, device_name),
                );
                set_transport_status(
                    &state,
                    peer_name,
                    format!("authenticated with {device_name}"),
                );
                if let Ok(mut s) = state.lock() {
                    s.last_auth_success_ms = now_ms();
                }
            } else {
                log_backend_event_with_state(
                    &state,
                    "FAILED",
                    "HANDSHAKE_OUTBOUND",
                    &format!("peer={} rejected: {}", peer_name, reason),
                );
                set_transport_status(&state, peer_name, format!("rejected: {reason}"));
            }
        }
        _ => {
            log_backend_event_with_state(
                &state,
                "FAILED",
                "HANDSHAKE_OUTBOUND",
                &format!("peer={} invalid ack payload", peer_name),
            );
            set_transport_status(&state, peer_name, "invalid ack".to_string());
        }
    }

    let _ = ws_stream.close(None).await;
}

pub async fn send_transport_payload_to_peer(
    peer_name: String,
    addr: String,
    sender_id: String,
    pairing_code: String,
    transport_payload: TransportMessage,
    state: SharedState,
) {
    let ws_url = format!("ws://{addr}");
    let connect_result = tokio::time::timeout(Duration::from_secs(3), connect_async(ws_url)).await;
    let Ok(Ok((mut ws_stream, _))) = connect_result else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "PAYLOAD_SEND",
            &format!("peer={} connect failed", peer_name),
        );
        set_transport_status(&state, peer_name, "connect failed".to_string());
        return;
    };

    let hello = TransportMessage::Hello {
        device_name: sender_id.clone(),
        pairing_code,
    };
    let Ok(hello_text) = serde_json::to_string(&hello) else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "PAYLOAD_SEND",
            &format!("peer={} hello serialization failed", peer_name),
        );
        let _ = ws_stream.close(None).await;
        return;
    };
    if ws_stream
        .send(Message::Text(hello_text.into()))
        .await
        .is_err()
    {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "PAYLOAD_SEND",
            &format!("peer={} send hello failed", peer_name),
        );
        set_transport_status(&state, peer_name, "send hello failed".to_string());
        let _ = ws_stream.close(None).await;
        return;
    }

    let ack_msg = tokio::time::timeout(Duration::from_secs(4), ws_stream.next()).await;
    let Ok(Some(Ok(Message::Text(ack_payload)))) = ack_msg else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "PAYLOAD_SEND",
            &format!("peer={} ack timeout", peer_name),
        );
        set_transport_status(&state, peer_name, "ack timeout".to_string());
        let _ = ws_stream.close(None).await;
        return;
    };

    let accepted = matches!(
        serde_json::from_str::<TransportMessage>(&ack_payload),
        Ok(TransportMessage::HelloAck { accepted: true, .. })
    );

    if !accepted {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "PAYLOAD_SEND",
            &format!("peer={} rejected: pairing mismatch", peer_name),
        );
        set_transport_status(&state, peer_name, "rejected: pairing mismatch".to_string());
        let _ = ws_stream.close(None).await;
        return;
    }

    let payload_label = match &transport_payload {
        TransportMessage::SyncImage { .. } => "image",
        TransportMessage::SyncText { .. } => "text",
        _ => "payload",
    };

    let Ok(sync_text) = serde_json::to_string(&transport_payload) else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "PAYLOAD_SEND",
            &format!(
                "peer={} {} payload serialization failed",
                peer_name, payload_label
            ),
        );
        let _ = ws_stream.close(None).await;
        return;
    };

    if ws_stream
        .send(Message::Text(sync_text.into()))
        .await
        .is_ok()
    {
        let peer_for_diag = peer_name.clone();
        set_transport_status(
            &state,
            peer_name,
            format!("authenticated + sent {payload_label}"),
        );
        if let Ok(mut s) = state.lock() {
            s.sync_sent_count += 1;
            s.last_auth_success_ms = now_ms();
            let event = format_backend_event(
                "SUCCESS",
                "PAYLOAD_SEND",
                &format!("peer={} payload={}", peer_for_diag, payload_label),
            );
            log_backend(&event);
            push_diagnostic(&mut s, event);
        }
    } else {
        log_backend_event_with_state(
            &state,
            "FAILED",
            "PAYLOAD_SEND",
            &format!(
                "peer={} authenticated but {} send failed",
                peer_name, payload_label
            ),
        );
        set_transport_status(
            &state,
            peer_name,
            "authenticated but send failed".to_string(),
        );
    }

    let _ = ws_stream.close(None).await;
}
