use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::domain::models::TransportMessage;
use crate::domain::state::SharedState;
use crate::network::set_transport_status;
use crate::services::logging::{
    format_backend_event, log_backend, log_backend_event_with_state, now_ms, push_diagnostic,
};

pub async fn send_pairing_request(peer_name: String, addr: String, token: String, state: SharedState) {
    let local_name = {
        let Ok(s) = state.lock() else { return; };
        s.device_name.clone()
    };

    let ws_url = format!("ws://{addr}");
    let connect_result = tokio::time::timeout(Duration::from_secs(3), connect_async(&ws_url)).await;
    let Ok(Ok((mut ws_stream, _))) = connect_result else {
        log_backend_event_with_state(&state, "FAILED", "PAIRING_REQ", &format!("peer={} connect failed", peer_name));
        return;
    };

    let req = TransportMessage::PairingRequest {
        device_name: local_name,
        token,
    };
    
    if let Ok(req_text) = serde_json::to_string(&req) {
        let _ = ws_stream.send(Message::Text(req_text.into())).await;
    }
    
    let _ = ws_stream.close(None).await;
}

pub async fn send_pairing_response(
    peer_name: String,
    addr: String,
    accepted: bool,
    state: SharedState,
) {
    let local_name = {
        let Ok(s) = state.lock() else { return; };
        s.device_name.clone()
    };

    let ws_url = format!("ws://{addr}");
    let connect_result = tokio::time::timeout(Duration::from_secs(3), connect_async(&ws_url)).await;
    let Ok(Ok((mut ws_stream, _))) = connect_result else {
        log_backend_event_with_state(&state, "FAILED", "PAIRING_RESP_SEND", &format!("peer={} connect failed", peer_name));
        return;
    };

    let resp = TransportMessage::PairingResponse {
        device_name: local_name,
        accepted,
    };
    
    if let Ok(resp_text) = serde_json::to_string(&resp) {
        let _ = ws_stream.send(Message::Text(resp_text.into())).await;
    }
    
    let _ = ws_stream.close(None).await;
}

pub async fn send_transport_payload_to_peer(
    peer_name: String,
    addr: String,
    sender_id: String,
    token: String,
    transport_payload: TransportMessage,
    state: SharedState,
) {
    let ws_url = format!("ws://{addr}");
    let connect_result = tokio::time::timeout(Duration::from_secs(3), connect_async(&ws_url)).await;
    let Ok(Ok((mut ws_stream, _))) = connect_result else {
        log_backend_event_with_state(&state, "FAILED", "PAYLOAD_SEND", &format!("peer={} connect failed", peer_name));
        set_transport_status(&state, peer_name, "connect failed".to_string());
        return;
    };

    let hello = TransportMessage::Hello {
        device_name: sender_id.clone(),
        token,
    };

    let Ok(hello_text) = serde_json::to_string(&hello) else {
        log_backend_event_with_state(&state, "FAILED", "PAYLOAD_SEND", "hello serialization failed");
        let _ = ws_stream.close(None).await;
        return;
    };

    if ws_stream.send(Message::Text(hello_text.into())).await.is_err() {
        log_backend_event_with_state(&state, "FAILED", "PAYLOAD_SEND", "send hello failed");
        set_transport_status(&state, peer_name, "send hello failed".to_string());
        let _ = ws_stream.close(None).await;
        return;
    }

    let ack_msg = tokio::time::timeout(Duration::from_secs(4), ws_stream.next()).await;
    let Ok(Some(Ok(Message::Text(ack_payload)))) = ack_msg else {
        log_backend_event_with_state(&state, "FAILED", "PAYLOAD_SEND", "ack timeout");
        set_transport_status(&state, peer_name, "ack timeout".to_string());
        let _ = ws_stream.close(None).await;
        return;
    };

    let accepted = matches!(
        serde_json::from_str::<TransportMessage>(&ack_payload),
        Ok(TransportMessage::HelloAck { accepted: true, .. })
    );

    if !accepted {
        log_backend_event_with_state(&state, "FAILED", "PAYLOAD_SEND", "rejected: pairing mismatch");
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
        log_backend_event_with_state(&state, "FAILED", "PAYLOAD_SEND", "payload serialization failed");
        let _ = ws_stream.close(None).await;
        return;
    };

    if ws_stream.send(Message::Text(sync_text.into())).await.is_ok() {
        set_transport_status(&state, peer_name.clone(), format!("authenticated + sent {payload_label}"));
        if let Ok(mut s) = state.lock() {
            s.sync_sent_count += 1;
            s.last_auth_success_ms = now_ms();
        }
    } else {
        set_transport_status(&state, peer_name, "authenticated but send failed".to_string());
    }

    let _ = ws_stream.close(None).await;
}
