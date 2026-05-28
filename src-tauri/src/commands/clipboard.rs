use std::time::Duration;
use tauri::State;

use crate::domain::models::{IncomingImage, TransportMessage};
use crate::domain::state::SharedState;
use crate::network::transport::send_transport_payload_to_peer;
use crate::services::hashing::{compute_text_hash, compute_image_hash, remember_hash};
use crate::services::logging::{format_backend_event, log_backend, now_ms, push_diagnostic};

#[tauri::command]
pub fn consume_remote_text(state: State<'_, SharedState>) -> Result<Option<String>, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.pending_remote_text.take())
}

#[tauri::command]
pub fn consume_remote_image(state: State<'_, SharedState>) -> Result<Option<IncomingImage>, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.pending_remote_image.take())
}

pub fn push_local_text_clipboard_impl(
    text: String,
    state_clone: SharedState,
) -> Result<(), String> {
    let (sender_id, targets, hash, timestamp_ms, allow_sync) = {
        let mut s = state_clone.lock().map_err(|e| e.to_string())?;

        let hash = compute_text_hash(&s.device_name, &text);
        let timestamp_ms = now_ms();

        if s.recent_hashes.contains(&hash) {
            s.sync_dropped_count += 1;
            let event =
                format_backend_event("FAILED", "TEXT_SEND_LOCAL", "duplicate local hash dropped");
            log_backend(&event);
            push_diagnostic(&mut s, event);
            return Ok(());
        }

        remember_hash(&mut s, hash);
        let targets = s
            .discovered
            .iter()
            .filter_map(|(name, addr)| {
                s.settings.trusted_peers.get(name).map(|token| {
                    (name.clone(), addr.clone(), token.clone())
                })
            })
            .filter(|(name, _, _)| !name.contains(&s.device_name))
            .collect::<Vec<(String, String, String)>>();

        (
            s.device_name.clone(),
            targets,
            hash,
            timestamp_ms,
            s.sync_enabled
                && (s.is_app_foreground || s.settings.background_mode_enabled),
        )
    };

    if !allow_sync {
        if let Ok(mut s) = state_clone.lock() {
            let event = format_backend_event(
                "FAILED",
                "TEXT_SEND_LOCAL",
                "blocked: sync disabled or backgrounded",
            );
            log_backend(&event);
            push_diagnostic(&mut s, event);
        }
        return Ok(());
    }

    let payload = TransportMessage::SyncText {
        sender_id: sender_id.clone(),
        timestamp_ms,
        message_hash: hash,
        text,
    };

    tauri::async_runtime::spawn(async move {
        let mut tasks = vec![];
        for (peer_name, addr, token) in targets {
            let s_clone = state_clone.clone();
            let p_clone = payload.clone();
            let n_clone = peer_name.clone();
            let sender_id_clone = sender_id.clone();
            
            tasks.push(tauri::async_runtime::spawn(async move {
                send_transport_payload_to_peer(
                    n_clone,
                    addr,
                    sender_id_clone,
                    token,
                    p_clone,
                    s_clone,
                )
                .await;
            }));
        }

        for t in tasks {
            let _ = tokio::time::timeout(Duration::from_secs(10), t).await;
        }
    });

    Ok(())
}

#[tauri::command]
pub fn push_local_text_clipboard(
    text: String,
    state: State<'_, SharedState>,
) -> Result<(), String> {
    push_local_text_clipboard_impl(text, state.inner().clone())
}

pub fn push_local_image_payload_impl(
    mime_type: String,
    image_base64: String,
    state_clone: SharedState,
) -> Result<(), String> {
    let (sender_id, targets, hash, timestamp_ms, allow_sync) = {
        let mut s = state_clone.lock().map_err(|e| e.to_string())?;

        let hash = compute_image_hash(&s.device_name, &mime_type, &image_base64);
        let timestamp_ms = now_ms();

        let size_kb = (image_base64.len() * 3 / 4) as u32 / 1024;
        if size_kb > s.settings.max_image_size_kb {
            s.sync_dropped_count += 1;
            let event = format_backend_event(
                "FAILED",
                "IMAGE_SEND_LOCAL",
                &format!("image too large ({} KB)", size_kb),
            );
            log_backend(&event);
            push_diagnostic(&mut s, event);
            return Err(format!("Image exceeds maximum size ({} KB).", size_kb));
        }

        if s.recent_hashes.contains(&hash) {
            s.sync_dropped_count += 1;
            let event =
                format_backend_event("FAILED", "IMAGE_SEND_LOCAL", "duplicate local hash dropped");
            log_backend(&event);
            push_diagnostic(&mut s, event);
            return Ok(());
        }

        remember_hash(&mut s, hash);
        let targets = s
            .discovered
            .iter()
            .filter_map(|(name, addr)| {
                s.settings.trusted_peers.get(name).map(|token| {
                    (name.clone(), addr.clone(), token.clone())
                })
            })
            .filter(|(name, _, _)| !name.contains(&s.device_name))
            .collect::<Vec<(String, String, String)>>();

        (
            s.device_name.clone(),
            targets,
            hash,
            timestamp_ms,
            s.sync_enabled
                && (s.is_app_foreground || s.settings.background_mode_enabled),
        )
    };

    if !allow_sync {
        if let Ok(mut s) = state_clone.lock() {
            let event = format_backend_event(
                "FAILED",
                "IMAGE_SEND_LOCAL",
                "blocked: sync disabled or backgrounded",
            );
            log_backend(&event);
            push_diagnostic(&mut s, event);
        }
        return Ok(());
    }

    let payload = TransportMessage::SyncImage {
        sender_id: sender_id.clone(),
        timestamp_ms,
        message_hash: hash,
        mime_type,
        image_base64,
    };

    tauri::async_runtime::spawn(async move {
        let mut tasks = vec![];
        for (peer_name, addr, token) in targets {
            let s_clone = state_clone.clone();
            let p_clone = payload.clone();
            let n_clone = peer_name.clone();
            let sender_id_clone = sender_id.clone();
            
            tasks.push(tauri::async_runtime::spawn(async move {
                send_transport_payload_to_peer(
                    n_clone,
                    addr,
                    sender_id_clone,
                    token,
                    p_clone,
                    s_clone,
                )
                .await;
            }));
        }

        for t in tasks {
            let _ = tokio::time::timeout(Duration::from_secs(10), t).await;
        }
    });

    Ok(())
}

#[tauri::command]
pub fn push_local_image_payload(
    mime_type: String,
    image_base64: String,
    state: State<'_, SharedState>,
) -> Result<(), String> {
    push_local_image_payload_impl(mime_type, image_base64, state.inner().clone())
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
#[tauri::command]
pub fn read_clipboard_text() -> Result<Option<String>, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    match clipboard.get_text() {
        Ok(text) => Ok(Some(text)),
        Err(arboard::Error::ContentNotAvailable) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
#[tauri::command]
pub fn write_clipboard_text(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub fn read_clipboard_text(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    match app.clipboard().read_text() {
        Ok(text) => {
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text))
            }
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("empty") || err_str.contains("no text") || err_str.contains("not available") || err_str.contains("Null") || err_str.contains("null") {
                Ok(None)
            } else {
                Err(err_str)
            }
        }
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
#[tauri::command]
pub fn write_clipboard_text(app: tauri::AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}
