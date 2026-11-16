use crate::{platform, toggle_main_window, show_main_window, AppState};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_autostart::ManagerExt;

pub struct TrayHandles {
    stats_item: MenuItem<Wry>,
    accessibility_item: MenuItem<Wry>,
    autostart_item: CheckMenuItem<Wry>,
}

impl TrayHandles {
    pub fn refresh(&self, app: &AppHandle) {
        let state = app.state::<AppState>();
        let stats = state.storage.stats();
        let _ = self.stats_item.set_text(format!(
            "历史记录: {} 条 (已置顶: {})",
            stats.total, stats.pinned
        ));

        let trusted = platform::is_accessibility_trusted(false);
        let perm_label = if trusted {
            "辅助功能: 已授权 ✓"
        } else {
            "辅助功能: 未授权 (点击设置)"
        };
        let _ = self.accessibility_item.set_text(perm_label);

        let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
        let _ = self.autostart_item.set_checked(autostart_enabled);
    }
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let stats = state.storage.stats();
    let trusted = platform::is_accessibility_trusted(false);
    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);

    let show_item = MenuItemBuilder::with_id("show", "显示 ClipFlow").build(app)?;
    let stats_item = MenuItemBuilder::with_id(
        "stats",
        format!("历史记录: {} 条 (已置顶: {})", stats.total, stats.pinned),
    )
    .enabled(false)
    .build(app)?;
    let clear_item = MenuItemBuilder::with_id("clear_unpinned", "清空未置顶历史").build(app)?;
    let accessibility_item = MenuItemBuilder::with_id(
        "accessibility",
        if trusted {
            "辅助功能: 已授权 ✓"
        } else {
            "辅助功能: 未授权 (点击设置)"
        },
    )
    .build(app)?;
    let autostart_item = CheckMenuItemBuilder::with_id("autostart", "开机自动启动")
        .checked(autostart_enabled)
        .build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "退出 ClipFlow").build(app)?;

    let mut menu_builder = MenuBuilder::new(app)
        .item(&show_item)
        .separator()
        .item(&stats_item)
        .item(&clear_item)
        .separator();

    #[cfg(target_os = "macos")]
    {
        menu_builder = menu_builder.item(&accessibility_item);
    }

    let menu = menu_builder
        .item(&autostart_item)
        .separator()
        .item(&quit_item)
        .build()?;

    let handles = TrayHandles {
        stats_item,
        accessibility_item,
        autostart_item,
    };
    app.manage(handles);

    let tray_icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    TrayIconBuilder::with_id("clipflow-tray")
        .icon(tray_icon)
        .tooltip("ClipFlow")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                show_main_window(app);
            }
            "clear_unpinned" => {
                let state = app.state::<AppState>();
                state.storage.clear_unpinned();
                if let Some(handles) = app.try_state::<TrayHandles>() {
                    handles.refresh(app);
                }
                let _ = app.emit("clipboard://cleared", ());
            }
            "accessibility" => {
                let _ = platform::is_accessibility_trusted(true);
                platform::open_accessibility_settings();
            }
            "autostart" => {
                let autolaunch = app.autolaunch();
                let currently_enabled = autolaunch.is_enabled().unwrap_or(false);
                if currently_enabled {
                    let _ = autolaunch.disable();
                } else {
                    let _ = autolaunch.enable();
                }
                if let Some(handles) = app.try_state::<TrayHandles>() {
                    handles.refresh(app);
                }
            }
            "quit" => {
                let state = app.state::<AppState>();
                state.monitor.stop_monitoring();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    toggle_main_window(app);
                }
                TrayIconEvent::Click {
                    button: MouseButton::Right,
                    button_state: MouseButtonState::Down,
                    ..
                } => {
                    if let Some(handles) = app.try_state::<TrayHandles>() {
                        handles.refresh(app);
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}
