mod clipboard;
mod commands;
mod models;
mod paste;
mod platform;
mod storage;
mod tray;

use clipboard::ClipboardMonitor;
use paste::PasteService;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use storage::StorageService;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub struct AppState {
    pub storage: Arc<StorageService>,
    pub monitor: Arc<ClipboardMonitor>,
    pub paste: Arc<PasteService>,
    pub last_shown_at: Mutex<Option<Instant>>,
}

pub fn show_main_window(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.paste.record_frontmost_app();
    {
        let mut shown = state
            .last_shown_at
            .lock()
            .expect("last_shown_at mutex poisoned");
        *shown = Some(Instant::now());
    }

    if let Some(handles) = app.try_state::<tray::TrayHandles>() {
        handles.refresh(app);
    }

    if let Some(window) = app.get_webview_window("main") {
        if let (Ok(cursor), Ok(monitors), Ok(win_size)) = (
            app.cursor_position(),
            window.available_monitors(),
            window.outer_size(),
        ) {
            let target_monitor = monitors.into_iter().find(|m| {
                let pos = m.position();
                let size = m.size();
                cursor.x >= pos.x as f64
                    && cursor.x < (pos.x + size.width as i32) as f64
                    && cursor.y >= pos.y as f64
                    && cursor.y < (pos.y + size.height as i32) as f64
            });

            if let Some(monitor) = target_monitor {
                let pos = monitor.position();
                let size = monitor.size();
                let scale = monitor.scale_factor();
                let x = pos.x + ((size.width as i32 - win_size.width as i32) / 2);
                let y_offset = (60.0 * scale) as i32;
                let min_y = pos.y + (28.0 * scale) as i32;
                let y = (pos.y + ((size.height as i32 - win_size.height as i32) / 2) - y_offset)
                    .max(min_y);
                let _ = window.set_position(PhysicalPosition::new(x, y));
            }
        }

        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("window://shown", ());
    }
}

pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            hide_main_window(app);
        } else {
            show_main_window(app);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let storage = Arc::new(StorageService::new());
    let monitor = Arc::new(ClipboardMonitor::new(Arc::clone(&storage)));
    let paste = Arc::new(PasteService::new(
        Arc::clone(&storage),
        Arc::clone(&monitor),
    ));

    let state = AppState {
        storage,
        monitor: Arc::clone(&monitor),
        paste,
        last_shown_at: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_main_window(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_handle = app.handle().clone();

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_visible_on_all_workspaces(true);
            }

            tray::setup_tray(&app_handle)?;

            let gs = app.global_shortcut();
            for shortcut in ["CommandOrControl+Shift+V", "Alt+V"] {
                if let Err(err) = gs.register(shortcut) {
                    eprintln!("[HotKey] Failed to register {shortcut}: {err}");
                }
            }

            let app_state = app.state::<AppState>();
            app_state.monitor.start_monitoring(app_handle);

            println!("[clip-flow] Ready and running in background.");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::Focused(false) = event {
                    let app = window.app_handle();
                    let state = app.state::<AppState>();
                    let recently_shown = state
                        .last_shown_at
                        .lock()
                        .expect("last_shown_at mutex poisoned")
                        .is_some_and(|t| t.elapsed() < Duration::from_millis(160));
                    if !recently_shown {
                        let _ = window.hide();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_history,
            commands::get_stats,
            commands::get_attachment_base64,
            commands::paste_item,
            commands::copy_text,
            commands::toggle_pin,
            commands::delete_item,
            commands::clear_unpinned,
            commands::check_accessibility_permission,
            commands::request_accessibility_permission,
            commands::hide_window,
            commands::open_external_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running clip-flow");
}
