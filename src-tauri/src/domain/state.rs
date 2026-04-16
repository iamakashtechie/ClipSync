use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::domain::models::{AppSettings, IncomingImage};

pub struct AppState {
    pub sync_enabled: bool,
    pub paired: bool,
    pub discovered: HashMap<String, String>,
    pub discovered_last_seen_ms: HashMap<String, u64>,
    pub transport_status: HashMap<String, String>,
    pub settings: AppSettings,
    pub device_name: String,
    pub recent_hashes: VecDeque<u64>,
    pub sync_sent_count: u64,
    pub sync_received_count: u64,
    pub sync_dropped_count: u64,
    pub sync_rejected_stale_count: u64,
    pub pending_remote_text: Option<String>,
    pub pending_remote_image: Option<IncomingImage>,
    pub last_applied_timestamp_ms: u64,
    pub last_applied_sender: String,
    pub diagnostic_events: VecDeque<String>,
    pub is_app_foreground: bool,
    pub last_visibility_report_ms: u64,
    pub last_auth_success_ms: u64,
    pub stale_peers_pruned: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            sync_enabled: true,
            paired: false,
            discovered: HashMap::new(),
            discovered_last_seen_ms: HashMap::new(),
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
            is_app_foreground: true,
            last_visibility_report_ms: 0,
            last_auth_success_ms: 0,
            stale_peers_pruned: 0,
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;
