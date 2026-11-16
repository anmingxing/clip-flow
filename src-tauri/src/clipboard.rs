use crate::models::{is_color_format, ClipContentType, ClipItem};
use crate::platform;
use crate::storage::StorageService;
use arboard::Clipboard;
use image::{ImageBuffer, ImageFormat, Rgba};
use std::collections::HashSet;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const EXCLUDED_BUNDLE_IDS: &[&str] = &[
    "com.agilebits.onepassword",
    "com.1password.1password",
    "com.1password.onepassword",
    "com.bitwarden.desktop",
    "org.keepassxc.keepassxc",
    "com.apple.keychainaccess",
    "com.lastpass.LastPass",
    "com.enpass.Enpass",
];

const EXCLUDED_APP_NAMES: &[&str] = &[
    "1password",
    "bitwarden",
    "keepassxc",
    "keepass",
    "lastpass",
    "enpass",
    "keychain access",
];

pub struct ClipboardMonitor {
    storage: Arc<StorageService>,
    self_written_hashes: Mutex<HashSet<String>>,
    self_written_image_hint: Mutex<Option<(Instant, u32, u32)>>,
    running: AtomicBool,
}

impl ClipboardMonitor {
    pub fn new(storage: Arc<StorageService>) -> Self {
        Self {
            storage,
            self_written_hashes: Mutex::new(HashSet::new()),
            self_written_image_hint: Mutex::new(None),
            running: AtomicBool::new(false),
        }
    }

    pub fn mark_self_written(&self, hash: &str) {
        let mut guard = self
            .self_written_hashes
            .lock()
            .expect("self_written_hashes mutex poisoned");
        guard.insert(hash.to_string());
    }

    pub fn mark_self_written_image(&self, width: u32, height: u32) {
        let mut guard = self
            .self_written_image_hint
            .lock()
            .expect("self_written_image_hint mutex poisoned");
        *guard = Some((Instant::now(), width, height));
    }

    fn consume_self_written_image(&self, width: u32, height: u32) -> bool {
        let mut guard = self
            .self_written_image_hint
            .lock()
            .expect("self_written_image_hint mutex poisoned");
        if let Some((at, w, h)) = *guard {
            if at.elapsed() < Duration::from_millis(1200) && w == width && h == height {
                *guard = None;
                return true;
            }
        }
        false
    }

    fn consume_self_written(&self, hash: &str) -> bool {
        let mut guard = self
            .self_written_hashes
            .lock()
            .expect("self_written_hashes mutex poisoned");
        guard.remove(hash)
    }

    pub fn start_monitoring(self: &Arc<Self>, app_handle: AppHandle) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let monitor = Arc::clone(self);
        let _ = std::thread::Builder::new()
            .name("clipflow-monitor".into())
            .spawn(move || {
                let mut clipboard = Clipboard::new().ok();
                let mut last_change_count = platform::get_clipboard_change_count();
                let mut last_polled_hash: Option<String> = None;

                while monitor.running.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(250));

                    let change_count = platform::get_clipboard_change_count();
                    if change_count > 0 {
                        if change_count == last_change_count {
                            continue;
                        }
                        last_change_count = change_count;
                    }

                    if platform::is_clipboard_concealed() {
                        continue;
                    }

                    let frontmost = platform::get_frontmost_app();
                    if let Some(ref app) = frontmost {
                        if is_excluded_app(app) {
                            continue;
                        }
                    }

                    if clipboard.is_none() {
                        clipboard = Clipboard::new().ok();
                    }

                    let Some(ref mut cb) = clipboard else {
                        continue;
                    };

                    let source_bundle = frontmost.as_ref().and_then(|a| a.bundle_id.clone());
                    let source_name = frontmost.as_ref().and_then(|a| a.name.clone());

                    if let Some(item) = monitor.extract_item(cb, source_bundle, source_name) {
                        if change_count <= 0
                            && last_polled_hash.as_deref() == Some(item.content_hash.as_str())
                        {
                            continue;
                        }
                        last_polled_hash = Some(item.content_hash.clone());

                        if monitor.consume_self_written(&item.content_hash) {
                            continue;
                        }

                        let saved = monitor.storage.save(item);
                        let _ = app_handle.emit("clipboard://updated", &saved);
                    }
                }
            });
    }

    pub fn stop_monitoring(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn extract_item(
        &self,
        clipboard: &mut Clipboard,
        source_bundle: Option<String>,
        source_name: Option<String>,
    ) -> Option<ClipItem> {
        let (file_paths, rtf_data) = platform::read_platform_files_and_rtf();

        // A. File URLs
        if let Some(paths) = file_paths {
            if !paths.is_empty() {
                let text = paths.join("\n");
                return Some(ClipItem::new(
                    ClipContentType::FileURL,
                    Some(text),
                    None,
                    None,
                    None,
                    None,
                    Some(paths),
                    source_bundle,
                    source_name,
                    None,
                ));
            }
        }

        // B. Image
        if let Ok(img) = clipboard.get_image() {
            if img.width > 0 && img.height > 0 {
                let width = img.width as u32;
                let height = img.height as u32;
                if self.consume_self_written_image(width, height) {
                    return None;
                }
                let raw_bytes = img.bytes.into_owned();
                if let Some(rgba_img) = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, raw_bytes)
                {
                    let mut png_bytes = Vec::new();
                    if rgba_img
                        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
                        .is_ok()
                    {
                        let mut hash_input = b"image:".to_vec();
                        hash_input.extend_from_slice(&png_bytes);
                        let img_hash = ClipItem::sha256_hex(&hash_input);

                        // Avoid writing duplicate PNG files to disk if already present or self-written
                        let already_exists = self.storage.has_hash(&img_hash) || {
                            let guard = self
                                .self_written_hashes
                                .lock()
                                .expect("self_written_hashes mutex poisoned");
                            guard.contains(&img_hash)
                        };

                        let file_name = if already_exists {
                            None
                        } else {
                            self.storage.save_image_png(&png_bytes)
                        };

                        if already_exists || file_name.is_some() {
                            return Some(ClipItem::new(
                                ClipContentType::Image,
                                None,
                                None,
                                file_name,
                                Some(width),
                                Some(height),
                                None,
                                source_bundle,
                                source_name,
                                Some(img_hash),
                            ));
                        }
                    }
                }
            }
        }

        // C & D. RichText / Color / Plain Text
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                let trimmed = text.trim();
                let content_type = if is_color_format(trimmed) {
                    ClipContentType::Color
                } else if rtf_data.is_some() {
                    ClipContentType::RichText
                } else {
                    ClipContentType::Text
                };

                return Some(ClipItem::new(
                    content_type,
                    Some(text),
                    rtf_data,
                    None,
                    None,
                    None,
                    None,
                    source_bundle,
                    source_name,
                    None,
                ));
            }
        }

        None
    }
}

fn is_excluded_app(app: &platform::FrontmostAppInfo) -> bool {
    if let Some(ref bundle_id) = app.bundle_id {
        if EXCLUDED_BUNDLE_IDS.iter().any(|&id| id == bundle_id) {
            return true;
        }
    }
    if let Some(ref name) = app.name {
        let lower = name.to_lowercase();
        if EXCLUDED_APP_NAMES.iter().any(|&n| lower.contains(n)) {
            return true;
        }
    }
    false
}
