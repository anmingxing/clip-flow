use crate::clipboard::ClipboardMonitor;
use crate::models::{ClipContentType, ClipItem};
use crate::platform::{self, FrontmostAppInfo};
use crate::storage::StorageService;
use arboard::{Clipboard, ImageData};
use image::ImageFormat;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};

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
        if let Some(app) = platform::get_frontmost_app() {
            if !platform::is_self_app(&app) {
                let mut guard = self
                    .previous_app
                    .lock()
                    .expect("previous_app mutex poisoned");
                *guard = Some(app);
            }
        }
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

        let target_app = {
            let guard = self
                .previous_app
                .lock()
                .expect("previous_app mutex poisoned");
            guard.clone()
        };

        std::thread::spawn(move || {
            if let Some(ref target) = target_app {
                let _ = platform::activate_app(target);
            }
            std::thread::sleep(Duration::from_millis(80));
            platform::simulate_paste_shortcut();
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
                    if let Some(ref text) = item.text {
                        let mut clipboard =
                            Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
                        clipboard
                            .set_text(text.clone())
                            .map_err(|e| format!("Failed to write text fallback: {e}"))?;
                    }
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
                    let mut clipboard =
                        Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
                    clipboard
                        .set_text(paths.join("\n"))
                        .map_err(|e| format!("Failed to write file paths: {e}"))?;
                }
            }
        }
    }

    Ok(())
}
