use std::fs;
use std::path::PathBuf;

use crate::domain::models::AppSettings;
use tauri::Manager;

pub fn settings_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    dir.push("settings.json");
    Ok(dir)
}

pub fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    let Ok(path) = settings_file_path(app) else {
        return AppSettings::default();
    };
    let Ok(content) = fs::read_to_string(path) else {
        return AppSettings::default();
    };

    serde_json::from_str::<AppSettings>(&content).unwrap_or_default()
}

pub fn save_settings_to_disk(app: &tauri::AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_file_path(app)?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn effective_device_name(settings: &AppSettings, host_name: &str) -> String {
    let custom = settings.device_name_override.trim();
    if custom.is_empty() {
        format!("clipsync-{host_name}")
    } else {
        custom.to_string()
    }
}
