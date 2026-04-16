use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppSettings {
    pub max_image_size_kb: u32,
    pub pairing_code: String,
    pub device_name_override: String,
    pub background_mode_enabled: bool,
    pub windows_start_on_login: bool,
    pub dev_mode_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            max_image_size_kb: 2048,
            pairing_code: "".to_string(),
            device_name_override: "".to_string(),
            background_mode_enabled: true,
            windows_start_on_login: false,
            dev_mode_enabled: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IncomingImage {
    pub mime_type: String,
    pub image_base64: String,
}

#[derive(Serialize, Deserialize)]
pub struct UdpDiscoveryBeacon {
    pub name: String,
    pub ws_port: u16,
    pub is_reply: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransportMessage {
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
