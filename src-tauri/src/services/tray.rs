use crate::domain::state::SharedState;
use crate::services::logging::{format_backend_event, log_backend, push_diagnostic};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

const TRAY_ID: &str = "clipsync-tray";
const MENU_OPEN_ID: &str = "tray_open";
const MENU_TOGGLE_SYNC_ID: &str = "tray_toggle_sync";
const MENU_QUIT_ID: &str = "tray_quit";

fn build_tray_status_text(state: &SharedState) -> String {
    let Ok(s) = state.lock() else {
        return "ClipSync | status unavailable".to_string();
    };

    let discovered_count = s.discovered.len();
    let authenticated_count = s
        .transport_status
        .values()
        .filter(|status| status.contains("authenticated"))
        .count();

    format!(
        "ClipSync | Sync: {} | Discovered: {} | Authenticated: {}",
        if s.sync_enabled { "ON" } else { "OFF" },
        discovered_count,
        authenticated_count
    )
}

pub fn refresh_tray_state(app: &tauri::AppHandle, state: &SharedState) {
    let text = build_tray_status_text(state);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(text.as_str()));
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn toggle_sync_from_tray(app: &tauri::AppHandle, state: &SharedState) {
    let mut event_to_log: Option<String> = None;

    if let Ok(mut s) = state.lock() {
        if !s.sync_enabled && !s.paired {
            let event = format_backend_event(
                "FAILED",
                "SYNC_TOGGLE_TRAY",
                "blocked: pairing required before enabling sync",
            );
            push_diagnostic(&mut s, event.clone());
            event_to_log = Some(event);
        } else {
            s.sync_enabled = !s.sync_enabled;
            let event = format_backend_event(
                "SUCCESS",
                "SYNC_TOGGLE_TRAY",
                if s.sync_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
            );
            push_diagnostic(&mut s, event.clone());
            event_to_log = Some(event);
        }
    }

    if let Some(event) = event_to_log {
        log_backend(&event);
    }

    refresh_tray_state(app, state);
}

pub fn setup_desktop_tray(app: &tauri::AppHandle, state: SharedState) -> Result<(), String> {
    let open_item = MenuItem::with_id(app, MENU_OPEN_ID, "Open", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let toggle_sync_item =
        MenuItem::with_id(app, MENU_TOGGLE_SYNC_ID, "Sync On/Off", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT_ID, "Quit", true, None::<&str>)
        .map_err(|e| e.to_string())?;

    let menu = Menu::with_items(app, &[&open_item, &toggle_sync_item, &quit_item])
        .map_err(|e| e.to_string())?;

    let state_for_menu = state.clone();
    let state_for_click = state.clone();

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            MENU_OPEN_ID => {
                show_main_window(app);
            }
            MENU_TOGGLE_SYNC_ID => {
                toggle_sync_from_tray(app, &state_for_menu);
            }
            MENU_QUIT_ID => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(&tray.app_handle());
                refresh_tray_state(&tray.app_handle(), &state_for_click);
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    tray_builder.build(app).map_err(|e| e.to_string())?;
    refresh_tray_state(app, &state);
    Ok(())
}
