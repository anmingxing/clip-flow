use crate::clipboard::ClipboardMonitor;
use crate::models::{ClipContentType, ClipItem};
use crate::platform::{self, FrontmostAppInfo};
use crate::storage::StorageService;
use arboard::{Clipboard, ImageData};
use image::ImageFormat;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub struct PasteService {
    storage: Arc<StorageService>,
    monitor: Arc<ClipboardMonitor>,
    previous_app: Mutex<Option<FrontmostAppInfo>>,
}

impl PasteService {
    pub fn new(storage: Arc<StorageService>, monitor: Arc<ClipboardMonitor>) -> Self {
        Self {
            storage,
            monitor,
            previous_app: Mutex::new(None),
        }
    }

    pub fn record_frontmost_app(&self) {
        let mut guard = self
            .previous_app
            .lock()
            .expect("previous_app mutex poisoned");
        *guard = platform::get_frontmost_app().filter(|app| !platform::is_self_app(app));
    }

    pub fn paste_item(
        &self,
        app_handle: &AppHandle,
        id: &str,
        plain_text_only: bool,
        auto_paste: bool,
    ) -> Result<(), String> {
        let item = self
            .storage
            .get_by_id(id)
            .ok_or_else(|| format!("Clip item not found: {id}"))?;

        let target_app = if auto_paste {
            Some(
                self.previous_app
                    .lock()
                    .expect("previous_app mutex poisoned")
                    .clone()
                    .ok_or_else(|| {
                        "无法确定粘贴目标，请先切换到目标应用后重新打开 ClipFlow".to_string()
                    })?,
            )
        } else {
            None
        };

        self.monitor.mark_self_written(&item.content_hash);
        if item.content_type == ClipContentType::Image {
            self.monitor.mark_self_written_image(
                item.image_width.unwrap_or(0),
                item.image_height.unwrap_or(0),
            );
        }

        write_item_to_clipboard(&self.storage, &item, plain_text_only)?;

        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.hide();
        }

        if !auto_paste {
            return Ok(());
        }

        let app_handle = app_handle.clone();
        std::thread::spawn(move || {
            let result = paste_to_target(
                target_app.as_ref(),
                platform::activate_app,
                platform::get_frontmost_app,
                platform::simulate_paste_shortcut,
            );
            if let Err(err) = result {
                eprintln!("[PasteService] {err}");
                let _ = app_handle.emit("clipboard://error", err);
            }
        });

        Ok(())
    }

    pub fn copy_plain_text(&self, text: &str) -> Result<(), String> {
        let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
        clipboard
            .set_text(text.to_string())
            .map_err(|e| format!("Failed to copy text: {e}"))
    }
}

fn paste_to_target(
    target: Option<&FrontmostAppInfo>,
    activate: impl FnOnce(&FrontmostAppInfo) -> bool,
    frontmost: impl FnOnce() -> Option<FrontmostAppInfo>,
    send_paste: impl FnOnce(),
) -> Result<(), String> {
    let target = target.ok_or("无法确定粘贴目标，已取消自动粘贴；内容已复制")?;
    if platform::is_self_app(target) || !activate(target) {
        return Err("无法激活目标应用，已取消自动粘贴；内容已复制".into());
    }
    std::thread::sleep(Duration::from_millis(80));
    let focused = frontmost().ok_or("无法确认目标应用焦点，已取消自动粘贴；内容已复制")?;
    if target.pid <= 0
        || focused.pid != target.pid
        || (target.window_handle != 0 && focused.window_handle != target.window_handle)
    {
        return Err("目标应用未获得焦点，已取消自动粘贴；内容已复制".into());
    }
    send_paste();
    Ok(())
}

fn write_item_to_clipboard(
    storage: &StorageService,
    item: &ClipItem,
    plain_text_only: bool,
) -> Result<(), String> {
    match item.content_type {
        ClipContentType::Text | ClipContentType::Color => {
            if let Some(ref text) = item.text {
                let mut clipboard =
                    Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
                clipboard
                    .set_text(text.clone())
                    .map_err(|e| format!("Failed to write text: {e}"))?;
            }
        }
        ClipContentType::RichText => {
            if plain_text_only {
                if let Some(ref text) = item.text {
                    let mut clipboard =
                        Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
                    clipboard
                        .set_text(text.clone())
                        .map_err(|e| format!("Failed to write plain text: {e}"))?;
                }
            } else {
                let rtf_bytes = item.rich_text_bytes();
                let wrote_native = platform::write_platform_files_and_rtf(
                    None,
                    rtf_bytes.as_deref(),
                    item.text.as_deref(),
                );
                if !wrote_native {
                    return Err("无法写入富文本剪贴板，请重试或使用 Shift+Enter 粘贴纯文本".into());
                }
            }
        }
        ClipContentType::Image => {
            if let Some(ref file_name) = item.image_file_name {
                let png_bytes = storage
                    .read_attachment_bytes(file_name)
                    .ok_or_else(|| format!("Image attachment missing: {file_name}"))?;
                let decoded = image::load_from_memory_with_format(&png_bytes, ImageFormat::Png)
                    .map_err(|e| format!("Failed to decode PNG image: {e}"))?;
                let rgba = decoded.to_rgba8();
                let (width, height) = rgba.dimensions();
                let img_data = ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: Cow::Owned(rgba.into_raw()),
                };
                let mut clipboard =
                    Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
                clipboard
                    .set_image(img_data)
                    .map_err(|e| format!("Failed to write image: {e}"))?;
            }
        }
        ClipContentType::FileURL => {
            if let Some(ref paths) = item.file_paths {
                let wrote_native = platform::write_platform_files_and_rtf(Some(paths), None, None);
                if !wrote_native {
                    return Err("无法写入文件剪贴板，请重试".into());
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn failed_or_missing_target_never_sends_paste() {
        let sent = Cell::new(false);
        let target = FrontmostAppInfo {
            pid: -1,
            ..Default::default()
        };
        assert!(
            paste_to_target(None, |_| true, || Some(target.clone()), || sent.set(true)).is_err()
        );
        assert!(paste_to_target(
            Some(&target),
            |_| false,
            || Some(target.clone()),
            || sent.set(true)
        )
        .is_err());
        assert!(!sent.get());
    }

    #[test]
    fn paste_requires_the_expected_window_to_be_focused() {
        let target = FrontmostAppInfo {
            pid: i32::MAX,
            window_handle: 10,
            ..Default::default()
        };
        let wrong = FrontmostAppInfo {
            window_handle: 11,
            ..target.clone()
        };
        let sent = Cell::new(false);
        assert!(
            paste_to_target(Some(&target), |_| true, || Some(wrong), || sent.set(true)).is_err()
        );
        assert!(!sent.get());
        assert!(paste_to_target(
            Some(&target),
            |_| true,
            || Some(target.clone()),
            || sent.set(true)
        )
        .is_ok());
        assert!(sent.get());
    }
}
