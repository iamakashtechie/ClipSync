use tauri::State;

use crate::domain::models::{IncomingImage, TransportMessage};
use crate::domain::state::SharedState;
use crate::network::set_transport_status;
use crate::network::transport::send_transport_payload_to_peer;
use crate::services::hashing::{compute_image_hash, compute_text_hash, remember_hash};
use crate::services::logging::{
    format_backend_event, log_backend, log_backend_event_with_state, now_ms, push_diagnostic,
};
use crate::services::security::is_private_or_loopback;

#[tauri::command]
pub fn consume_remote_text(state: State<'_, SharedState>) -> Result<Option<String>, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let consumed = s.pending_remote_text.take();
    if let Some(text) = &consumed {
        let event = format_backend_event("SUCCESS", "TEXT_CONSUMED", &format!("len={}", text.len()));
        log_backend(&event);
        push_diagnostic(&mut s, event);
    }
    Ok(consumed)
}

#[tauri::command]
pub fn consume_remote_image(state: State<'_, SharedState>) -> Result<Option<IncomingImage>, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let consumed = s.pending_remote_image.take();
    if let Some(image) = &consumed {
        let event = format_backend_event(
            "SUCCESS",
            "IMAGE_CONSUMED",
            &format!("mime_type={} bytes(base64)={}", image.mime_type, image.image_base64.len()),
        );
        log_backend(&event);
        push_diagnostic(&mut s, event);
    }
    Ok(consumed)
}

#[tauri::command]
pub async fn push_local_text_clipboard(content: String, state: State<'_, SharedState>) -> Result<(), String> {
    if content.trim().is_empty() {
        log_backend_event_with_state(state.inner(), "FAILED", "TEXT_SEND_LOCAL", "empty payload");
        return Ok(());
    }

    let (sender_id, pairing_code, peers, message_hash, timestamp_ms, allow_sync) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let hash = compute_text_hash(&s.device_name, &content);
        let timestamp_ms = now_ms();

        if s.recent_hashes.contains(&hash) {
            s.sync_dropped_count += 1;
            let event = format_backend_event("FAILED", "TEXT_SEND_LOCAL", "duplicate local hash dropped");
            log_backend(&event);
            push_diagnostic(&mut s, event);
            return Ok(());
        }

        remember_hash(&mut s, hash);
        let discovered = s
            .discovered
            .iter()
            .filter(|(name, _)| !name.contains(&s.device_name))
            .map(|(name, addr)| (name.clone(), addr.clone()))
            .collect::<Vec<(String, String)>>();

        (
            s.device_name.clone(),
            s.settings.pairing_code.clone(),
            discovered,
            hash,
            timestamp_ms,
            s.sync_enabled && s.paired && (s.is_app_foreground || s.settings.background_mode_enabled),
        )
    };

    if !allow_sync {
        if let Ok(mut s) = state.lock() {
            let event = format_backend_event(
                "FAILED",
                "TEXT_SEND_LOCAL",
                "blocked: app background and background mode disabled",
            );
            log_backend(&event);
            push_diagnostic(&mut s, event);
        }
        return Ok(());
    }

    log_backend_event_with_state(
        state.inner(),
        "INFO",
        "TEXT_SEND_LOCAL",
        &format!("dispatching to {} peer(s)", peers.len()),
    );

    for (peer_name, addr) in peers {
        let host = addr.split(':').next().unwrap_or_default();
        if !is_private_or_loopback(host) {
            set_transport_status(state.inner(), peer_name, "skipped: non-local address".to_string());
            continue;
        }

        send_transport_payload_to_peer(
            peer_name,
            addr,
            sender_id.clone(),
            pairing_code.clone(),
            TransportMessage::SyncText {
                sender_id: sender_id.clone(),
                timestamp_ms,
                message_hash,
                text: content.clone(),
            },
            state.inner().clone(),
        )
        .await;
    }

    Ok(())
}

#[tauri::command]
pub async fn push_local_image_payload(
    image_base64: String,
    mime_type: String,
    state: State<'_, SharedState>,
) -> Result<(), String> {
    if image_base64.trim().is_empty() || mime_type.trim().is_empty() {
        log_backend_event_with_state(state.inner(), "FAILED", "IMAGE_SEND_LOCAL", "empty image payload or mime type");
        return Ok(());
    }

    let (sender_id, pairing_code, peers, message_hash, timestamp_ms, allow_sync) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let hash = compute_image_hash(&s.device_name, &mime_type, &image_base64);
        let timestamp_ms = now_ms();

        if s.recent_hashes.contains(&hash) {
            s.sync_dropped_count += 1;
            let event = format_backend_event("FAILED", "IMAGE_SEND_LOCAL", "duplicate local hash dropped");
            log_backend(&event);
            push_diagnostic(&mut s, event);
            return Ok(());
        }

        remember_hash(&mut s, hash);
        let discovered = s
            .discovered
            .iter()
            .filter(|(name, _)| !name.contains(&s.device_name))
            .map(|(name, addr)| (name.clone(), addr.clone()))
            .collect::<Vec<(String, String)>>();

        (
            s.device_name.clone(),
            s.settings.pairing_code.clone(),
            discovered,
            hash,
            timestamp_ms,
            s.sync_enabled && s.paired && (s.is_app_foreground || s.settings.background_mode_enabled),
        )
    };

    if !allow_sync {
        if let Ok(mut s) = state.lock() {
            let event = format_backend_event(
                "FAILED",
                "IMAGE_SEND_LOCAL",
                "blocked: app background and background mode disabled",
            );
            log_backend(&event);
            push_diagnostic(&mut s, event);
        }
        return Ok(());
    }

    log_backend_event_with_state(
        state.inner(),
        "INFO",
        "IMAGE_SEND_LOCAL",
        &format!("dispatching to {} peer(s)", peers.len()),
    );

    for (peer_name, addr) in peers {
        let host = addr.split(':').next().unwrap_or_default();
        if !is_private_or_loopback(host) {
            set_transport_status(state.inner(), peer_name, "skipped: non-local address".to_string());
            continue;
        }

        send_transport_payload_to_peer(
            peer_name,
            addr,
            sender_id.clone(),
            pairing_code.clone(),
            TransportMessage::SyncImage {
                sender_id: sender_id.clone(),
                timestamp_ms,
                message_hash,
                mime_type: mime_type.clone(),
                image_base64: image_base64.clone(),
            },
            state.inner().clone(),
        )
        .await;
    }

    Ok(())
}
