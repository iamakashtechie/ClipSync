#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

const CLIPSYNC_SERVICE_TYPE: &str = "_clipsync._tcp.local.";
const CLIPSYNC_WS_PORT: u16 = 9876;
const CLIPSYNC_UDP_DISCOVERY_PORT: u16 = 9877;

#[derive(Serialize, Deserialize, Clone)]
struct AppSettings {
    max_image_size_kb: u32,
    pairing_code: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            max_image_size_kb: 2048,
            pairing_code: "".to_string(),
        }
    }
}

struct AppState {
    sync_enabled: bool,
    paired: bool,
    discovered: HashMap<String, String>,
    settings: AppSettings,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sync_enabled: true,
            paired: false,
            discovered: HashMap::new(),
            settings: AppSettings::default(),
        }
    }
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
    Ok(serde_json::json!({
        "status": if !devices.is_empty() { "connected" } else { "searching" },
        "sync_enabled": state.sync_enabled,
        "paired": state.paired,
        "devices": devices
    }))
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
    };
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
            toggle_sync,
            get_settings,
            save_settings,
            validate_pairing
        ])
        .setup(|app| {
            let settings = load_settings(app.handle());
            let state: State<'_, SharedState> = app.state();
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.settings = settings;
            drop(s);

            let host_name = whoami::fallible::hostname().unwrap_or_else(|_| "unknown-host".to_string());
            let device_name = format!("clipsync-{host_name}");
            let state_clone: SharedState = app.state::<SharedState>().inner().clone();
            start_mdns_discovery(state_clone.clone(), device_name.clone());
            start_udp_fallback_discovery(state_clone, device_name);

            println!("ClipSync v0.1 started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
