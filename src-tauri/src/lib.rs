#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};

const CLIPSYNC_SERVICE_TYPE: &str = "_clipsync._tcp.local.";
const CLIPSYNC_WS_PORT: u16 = 9876;
const CLIPSYNC_UDP_DISCOVERY_PORT: u16 = 9877;

#[derive(Serialize, Deserialize, Clone)]
struct AppSettings {
    max_image_size_kb: u32,
    pairing_code: String,
    device_name_override: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            max_image_size_kb: 2048,
            pairing_code: "".to_string(),
            device_name_override: "".to_string(),
        }
    }
}

struct AppState {
    sync_enabled: bool,
    paired: bool,
    discovered: HashMap<String, String>,
    transport_status: HashMap<String, String>,
    settings: AppSettings,
    device_name: String,
    recent_hashes: VecDeque<u64>,
    sync_sent_count: u64,
    sync_received_count: u64,
    sync_dropped_count: u64,
    sync_rejected_stale_count: u64,
    pending_remote_text: Option<String>,
    pending_remote_image: Option<IncomingImage>,
    last_applied_timestamp_ms: u64,
    last_applied_sender: String,
    diagnostic_events: VecDeque<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sync_enabled: true,
            paired: false,
            discovered: HashMap::new(),
            transport_status: HashMap::new(),
            settings: AppSettings::default(),
            device_name: "".to_string(),
            recent_hashes: VecDeque::new(),
            sync_sent_count: 0,
            sync_received_count: 0,
            sync_dropped_count: 0,
            sync_rejected_stale_count: 0,
            pending_remote_text: None,
            pending_remote_image: None,
            last_applied_timestamp_ms: 0,
            last_applied_sender: "".to_string(),
            diagnostic_events: VecDeque::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct IncomingImage {
    mime_type: String,
    image_base64: String,
}

type SharedState = Arc<Mutex<AppState>>;

fn settings_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    dir.push("settings.json");
    Ok(dir)
}

fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    let Ok(path) = settings_file_path(app) else {
        return AppSettings::default();
    };
    let Ok(content) = fs::read_to_string(path) else {
        return AppSettings::default();
    };

    serde_json::from_str::<AppSettings>(&content).unwrap_or_default()
}

fn save_settings_to_disk(app: &tauri::AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_file_path(app)?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn effective_device_name(settings: &AppSettings, host_name: &str) -> String {
    let custom = settings.device_name_override.trim();
    if custom.is_empty() {
        format!("clipsync-{host_name}")
    } else {
        custom.to_string()
    }
}

#[derive(Serialize, Deserialize)]
struct UdpDiscoveryBeacon {
    name: String,
    ws_port: u16,
    is_reply: bool,
}

fn upsert_discovered_device(state: &SharedState, device_name: String, addr: String) {
    if let Ok(mut s) = state.lock() {
        s.discovered.insert(device_name, addr);
    }
}

fn set_transport_status(state: &SharedState, peer: String, status: String) {
    if let Ok(mut s) = state.lock() {
        s.transport_status.insert(peer, status);
    }
}

fn compute_text_hash(sender: &str, text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sender.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

fn compute_image_hash(sender: &str, mime_type: &str, image_base64: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sender.hash(&mut hasher);
    mime_type.hash(&mut hasher);
    image_base64.hash(&mut hasher);
    hasher.finish()
}

fn is_private_or_loopback(ip: &str) -> bool {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        Ok(IpAddr::V6(v6)) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
        Err(_) => false,
    }
}

fn remember_hash(state: &mut AppState, hash: u64) {
    const MAX_RECENT_HASHES: usize = 64;
    if state.recent_hashes.contains(&hash) {
        return;
    }
    state.recent_hashes.push_back(hash);
    if state.recent_hashes.len() > MAX_RECENT_HASHES {
        state.recent_hashes.pop_front();
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn push_diagnostic(state: &mut AppState, event: String) {
    const MAX_EVENTS: usize = 120;
    state.diagnostic_events.push_back(event);
    if state.diagnostic_events.len() > MAX_EVENTS {
        state.diagnostic_events.pop_front();
    }
}

fn should_accept_incoming(state: &mut AppState, sender_id: &str, timestamp_ms: u64) -> bool {
    if timestamp_ms > state.last_applied_timestamp_ms {
        state.last_applied_timestamp_ms = timestamp_ms;
        state.last_applied_sender = sender_id.to_string();
        return true;
    }

    if timestamp_ms == state.last_applied_timestamp_ms && sender_id > state.last_applied_sender.as_str() {
        state.last_applied_sender = sender_id.to_string();
        return true;
    }

    false
}

fn start_mdns_discovery(state: SharedState, device_name: String) {
    let daemon = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mDNS daemon init failed: {e}");
            return;
        }
    };

    let host_name = format!("{device_name}.local.");
    let service_info = match ServiceInfo::new(
        CLIPSYNC_SERVICE_TYPE,
        &device_name,
        &host_name,
        "",
        CLIPSYNC_WS_PORT,
        None,
    ) {
        Ok(info) => info,
        Err(e) => {
            eprintln!("mDNS service info creation failed: {e}");
            return;
        }
    };

    if let Err(e) = daemon.register(service_info) {
        eprintln!("mDNS register failed: {e}");
        return;
    }

    let receiver = match daemon.browse(CLIPSYNC_SERVICE_TYPE) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("mDNS browse failed: {e}");
            return;
        }
    };

    thread::spawn(move || {
        let _keep_alive = daemon;

        while let Ok(event) = receiver.recv() {
            if let ServiceEvent::ServiceResolved(info) = event {
                let resolved_name = info.get_fullname().to_string();
                if resolved_name.contains(&device_name) {
                    continue;
                }

                let first_addr = info.get_addresses().iter().next().map(|a| a.to_string());
                if let Some(ip) = first_addr {
                    let addr = format!("{}:{}", ip, info.get_port());
                    upsert_discovered_device(&state, resolved_name, addr);
                }
            }
        }
    });
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TransportMessage {
    Hello {
        device_name: String,
        pairing_code: String,
    },
    HelloAck {
        device_name: String,
        accepted: bool,
        reason: String,
    },
    SyncText {
        sender_id: String,
        timestamp_ms: u64,
        message_hash: u64,
        text: String,
    },
    SyncImage {
        sender_id: String,
        timestamp_ms: u64,
        message_hash: u64,
        mime_type: String,
        image_base64: String,
    },
}

async fn handle_incoming_transport_connection(
    stream: tokio::net::TcpStream,
    state: SharedState,
) {
    let Ok(peer_addr) = stream.peer_addr() else {
        return;
    };

    let Ok(mut ws_stream) = accept_async(stream).await else {
        return;
    };

    let next_msg = tokio::time::timeout(Duration::from_secs(4), ws_stream.next()).await;
    let incoming = match next_msg {
        Ok(Some(Ok(msg))) => msg,
        _ => {
            let _ = ws_stream.close(None).await;
            return;
        }
    };

    let Message::Text(payload) = incoming else {
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
        set_transport_status(&state, peer_label, "authenticated (inbound)".to_string());

        let maybe_sync = tokio::time::timeout(Duration::from_secs(2), ws_stream.next()).await;
        if let Ok(Some(Ok(Message::Text(payload)))) = maybe_sync {
            if let Ok(TransportMessage::SyncText {
                sender_id,
                timestamp_ms,
                message_hash,
                text,
            }) = serde_json::from_str::<TransportMessage>(&payload)
            {
                if let Ok(mut s) = state.lock() {
                    if s.recent_hashes.contains(&message_hash) {
                        s.sync_dropped_count += 1;
                        push_diagnostic(&mut s, format!("dropped duplicate text hash={} from {}", message_hash, sender_id));
                    } else if !should_accept_incoming(&mut s, &sender_id, timestamp_ms) {
                        s.sync_rejected_stale_count += 1;
                        push_diagnostic(&mut s, format!("rejected stale text from {} at {}", sender_id, timestamp_ms));
                    } else {
                        remember_hash(&mut s, message_hash);
                        s.pending_remote_text = Some(text);
                        s.sync_received_count += 1;
                        push_diagnostic(&mut s, format!("accepted text from {} at {}", sender_id, timestamp_ms));
                    }
                    s.transport_status.insert(
                        format!("{} ({})", sender_id, peer_addr.ip()),
                        "authenticated (inbound) + synced text".to_string(),
                    );
                }
            }

            if let Ok(TransportMessage::SyncImage {
                sender_id,
                timestamp_ms,
                message_hash,
                mime_type,
                image_base64,
            }) = serde_json::from_str::<TransportMessage>(&payload)
            {
                if let Ok(mut s) = state.lock() {
                    if s.recent_hashes.contains(&message_hash) {
                        s.sync_dropped_count += 1;
                        push_diagnostic(&mut s, format!("dropped duplicate image hash={} from {}", message_hash, sender_id));
                    } else if !should_accept_incoming(&mut s, &sender_id, timestamp_ms) {
                        s.sync_rejected_stale_count += 1;
                        push_diagnostic(&mut s, format!("rejected stale image from {} at {}", sender_id, timestamp_ms));
                    } else {
                        remember_hash(&mut s, message_hash);
                        s.pending_remote_image = Some(IncomingImage {
                            mime_type,
                            image_base64,
                        });
                        s.sync_received_count += 1;
                        push_diagnostic(&mut s, format!("accepted image from {} at {}", sender_id, timestamp_ms));
                    }
                    s.transport_status.insert(
                        format!("{} ({})", sender_id, peer_addr.ip()),
                        "authenticated (inbound) + synced image".to_string(),
                    );
                }
            }
        }
    } else {
        set_transport_status(&state, peer_label, "rejected: pairing mismatch".to_string());
    }

    let _ = ws_stream.close(None).await;
}

fn start_transport_server(state: SharedState) {
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

async fn attempt_outbound_handshake(peer_name: String, addr: String, state: SharedState) {
    let (local_name, local_code) = {
        let Ok(s) = state.lock() else {
            return;
        };
        (s.device_name.clone(), s.settings.pairing_code.clone())
    };

    if local_name.is_empty() || local_code.is_empty() {
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
        set_transport_status(&state, peer_name, "connect failed".to_string());
        return;
    };

    let hello = TransportMessage::Hello {
        device_name: local_name,
        pairing_code: local_code,
    };
    let Ok(hello_text) = serde_json::to_string(&hello) else {
        set_transport_status(&state, peer_name, "local serialization failed".to_string());
        let _ = ws_stream.close(None).await;
        return;
    };

    if ws_stream.send(Message::Text(hello_text.into())).await.is_err() {
        set_transport_status(&state, peer_name, "send hello failed".to_string());
        let _ = ws_stream.close(None).await;
        return;
    }

    let ack_msg = tokio::time::timeout(Duration::from_secs(4), ws_stream.next()).await;
    let Ok(Some(Ok(Message::Text(payload)))) = ack_msg else {
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
                set_transport_status(
                    &state,
                    peer_name,
                    format!("authenticated with {device_name}"),
                );
            } else {
                set_transport_status(&state, peer_name, format!("rejected: {reason}"));
            }
        }
        _ => {
            set_transport_status(&state, peer_name, "invalid ack".to_string());
        }
    }

    let _ = ws_stream.close(None).await;
}

async fn send_transport_payload_to_peer(
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
        set_transport_status(&state, peer_name, "connect failed".to_string());
        return;
    };

    let hello = TransportMessage::Hello {
        device_name: sender_id.clone(),
        pairing_code,
    };
    let Ok(hello_text) = serde_json::to_string(&hello) else {
        let _ = ws_stream.close(None).await;
        return;
    };
    if ws_stream.send(Message::Text(hello_text.into())).await.is_err() {
        set_transport_status(&state, peer_name, "send hello failed".to_string());
        let _ = ws_stream.close(None).await;
        return;
    }

    let ack_msg = tokio::time::timeout(Duration::from_secs(4), ws_stream.next()).await;
    let Ok(Some(Ok(Message::Text(ack_payload)))) = ack_msg else {
        set_transport_status(&state, peer_name, "ack timeout".to_string());
        let _ = ws_stream.close(None).await;
        return;
    };

    let accepted = matches!(
        serde_json::from_str::<TransportMessage>(&ack_payload),
        Ok(TransportMessage::HelloAck { accepted: true, .. })
    );

    if !accepted {
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
        let _ = ws_stream.close(None).await;
        return;
    };

    if ws_stream.send(Message::Text(sync_text.into())).await.is_ok() {
        let peer_for_diag = peer_name.clone();
        set_transport_status(
            &state,
            peer_name,
            format!("authenticated + sent {payload_label}"),
        );
        if let Ok(mut s) = state.lock() {
            s.sync_sent_count += 1;
            push_diagnostic(&mut s, format!("sent {payload_label} to {}", peer_for_diag));
        }
    } else {
        set_transport_status(&state, peer_name, "authenticated but send failed".to_string());
    }

    let _ = ws_stream.close(None).await;
}

fn start_transport_handshake_loop(state: SharedState) {
    tauri::async_runtime::spawn(async move {
        loop {
            let peers = {
                let Ok(s) = state.lock() else {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                };

                s.discovered
                    .iter()
                    .filter(|(name, _)| !name.contains(&s.device_name))
                    .map(|(name, addr)| (name.clone(), addr.clone()))
                    .collect::<Vec<(String, String)>>()
            };

            for (peer_name, addr) in peers {
                let host = addr.split(':').next().unwrap_or_default();
                if !is_private_or_loopback(host) {
                    set_transport_status(
                        &state,
                        peer_name,
                        "skipped: non-local address".to_string(),
                    );
                    continue;
                }
                attempt_outbound_handshake(peer_name, addr, state.clone()).await;
            }

            tokio::time::sleep(Duration::from_secs(4)).await;
        }
    });
}

fn start_udp_fallback_discovery(state: SharedState, device_name: String) {
    thread::spawn(move || {
        let socket = match UdpSocket::bind(("0.0.0.0", CLIPSYNC_UDP_DISCOVERY_PORT)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("UDP discovery bind failed: {e}");
                return;
            }
        };

        if let Err(e) = socket.set_broadcast(true) {
            eprintln!("UDP discovery broadcast setup failed: {e}");
            return;
        }
        if let Err(e) = socket.set_read_timeout(Some(Duration::from_secs(1))) {
            eprintln!("UDP discovery read timeout setup failed: {e}");
            return;
        }

        let beacon = UdpDiscoveryBeacon {
            name: device_name.clone(),
            ws_port: CLIPSYNC_WS_PORT,
            is_reply: false,
        };
        let beacon_payload = match serde_json::to_vec(&beacon) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("UDP discovery beacon encoding failed: {e}");
                return;
            }
        };

        let mut last_broadcast = Instant::now() - Duration::from_secs(10);
        let mut recv_buf = [0u8; 1024];

        loop {
            if last_broadcast.elapsed() >= Duration::from_secs(5) {
                let _ = socket.send_to(
                    &beacon_payload,
                    ("255.255.255.255", CLIPSYNC_UDP_DISCOVERY_PORT),
                );
                last_broadcast = Instant::now();
            }

            if let Ok((len, src_addr)) = socket.recv_from(&mut recv_buf) {
                if let Ok(remote) = serde_json::from_slice::<UdpDiscoveryBeacon>(&recv_buf[..len]) {
                    if remote.name == device_name {
                        continue;
                    }

                    let peer_addr = format!("{}:{}", src_addr.ip(), remote.ws_port);
                    upsert_discovered_device(&state, remote.name, peer_addr);

                    // Reply to non-reply beacons so discovery becomes symmetric even when
                    // one side's broadcast path is unreliable on hotspot/Wi-Fi setups.
                    if !remote.is_reply {
                        let reply = UdpDiscoveryBeacon {
                            name: device_name.clone(),
                            ws_port: CLIPSYNC_WS_PORT,
                            is_reply: true,
                        };

                        if let Ok(reply_payload) = serde_json::to_vec(&reply) {
                            let _ = socket.send_to(
                                &reply_payload,
                                (src_addr.ip(), CLIPSYNC_UDP_DISCOVERY_PORT),
                            );
                        }
                    }
                }
            }
        }
    });
}

#[tauri::command]
fn get_status(state: State<'_, SharedState>) -> Result<serde_json::Value, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let devices: Vec<String> = state.discovered.keys().cloned().collect();
    let peer_transport = state.transport_status.clone();
    Ok(serde_json::json!({
        "status": if !devices.is_empty() { "connected" } else { "searching" },
        "sync_enabled": state.sync_enabled,
        "paired": state.paired,
        "devices": devices,
        "peer_transport": peer_transport,
        "sync_stats": {
            "sent": state.sync_sent_count,
            "received": state.sync_received_count,
            "dropped": state.sync_dropped_count,
            "stale_rejected": state.sync_rejected_stale_count
        }
    }))
}

#[tauri::command]
fn get_diagnostics(state: State<'_, SharedState>) -> Result<Vec<String>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.diagnostic_events.iter().cloned().collect())
}

#[tauri::command]
fn consume_remote_text(state: State<'_, SharedState>) -> Result<Option<String>, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.pending_remote_text.take())
}

#[tauri::command]
fn consume_remote_image(state: State<'_, SharedState>) -> Result<Option<IncomingImage>, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.pending_remote_image.take())
}

#[tauri::command]
async fn push_local_text_clipboard(content: String, state: State<'_, SharedState>) -> Result<(), String> {
    if content.trim().is_empty() {
        return Ok(());
    }

    let (sender_id, pairing_code, peers, message_hash, timestamp_ms, allow_sync) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let hash = compute_text_hash(&s.device_name, &content);
        let timestamp_ms = now_ms();

        if s.recent_hashes.contains(&hash) {
            s.sync_dropped_count += 1;
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
            s.sync_enabled && s.paired,
        )
    };

    if !allow_sync {
        return Ok(());
    }

    for (peer_name, addr) in peers {
        let host = addr.split(':').next().unwrap_or_default();
        if !is_private_or_loopback(host) {
            set_transport_status(
                state.inner(),
                peer_name,
                "skipped: non-local address".to_string(),
            );
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
async fn push_local_image_payload(
    image_base64: String,
    mime_type: String,
    state: State<'_, SharedState>,
) -> Result<(), String> {
    if image_base64.trim().is_empty() || mime_type.trim().is_empty() {
        return Ok(());
    }

    let (sender_id, pairing_code, peers, message_hash, timestamp_ms, allow_sync) = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let hash = compute_image_hash(&s.device_name, &mime_type, &image_base64);
        let timestamp_ms = now_ms();

        if s.recent_hashes.contains(&hash) {
            s.sync_dropped_count += 1;
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
            s.sync_enabled && s.paired,
        )
    };

    if !allow_sync {
        return Ok(());
    }

    for (peer_name, addr) in peers {
        let host = addr.split(':').next().unwrap_or_default();
        if !is_private_or_loopback(host) {
            set_transport_status(
                state.inner(),
                peer_name,
                "skipped: non-local address".to_string(),
            );
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

#[tauri::command]
fn toggle_sync(enabled: bool, state: State<'_, SharedState>) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if enabled && !s.paired {
        return Err("Pairing required before enabling sync".to_string());
    }
    s.sync_enabled = enabled;
    println!("Sync turned {}", if enabled { "ON" } else { "OFF" });
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<'_, SharedState>) -> Result<AppSettings, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.settings.clone())
}

#[tauri::command]
fn save_settings(
    max_image_size_kb: u32,
    pairing_code: String,
    device_name_override: String,
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if !pairing_code.chars().all(|c| c.is_ascii_digit()) || pairing_code.len() != 4 {
        return Err("Pairing code must be exactly 4 digits".to_string());
    }

    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.settings = AppSettings {
        max_image_size_kb,
        pairing_code,
        device_name_override,
    };
    let host_name = whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
    s.device_name = effective_device_name(&s.settings, &host_name);
    s.transport_status.clear();
    s.paired = false;
    s.sync_enabled = false;
    save_settings_to_disk(&app, &s.settings)
}

#[tauri::command]
fn validate_pairing(code: String, state: State<'_, SharedState>) -> Result<bool, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let ok = !s.settings.pairing_code.is_empty() && s.settings.pairing_code == code;
    s.paired = ok;
    if !ok {
        s.sync_enabled = false;
    }
    Ok(ok)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_state = Arc::new(Mutex::new(AppState::default()));

    tauri::Builder::default()
        .manage(initial_state)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_diagnostics,
            consume_remote_text,
            consume_remote_image,
            push_local_text_clipboard,
            push_local_image_payload,
            toggle_sync,
            get_settings,
            save_settings,
            validate_pairing
        ])
        .setup(|app| {
            let settings = load_settings(app.handle());
            let host_name =
                whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
            let device_name = effective_device_name(&settings, &host_name);

            let state: State<'_, SharedState> = app.state();
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.settings = settings;
            s.device_name = device_name.clone();
            drop(s);

            let state_clone: SharedState = app.state::<SharedState>().inner().clone();
            start_mdns_discovery(state_clone.clone(), device_name.clone());
            start_udp_fallback_discovery(state_clone, device_name);

            let transport_state: SharedState = app.state::<SharedState>().inner().clone();
            start_transport_server(transport_state.clone());
            start_transport_handshake_loop(transport_state);

            println!("ClipSync v0.1 started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
