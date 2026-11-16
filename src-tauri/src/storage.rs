use crate::models::{ClipContentType, ClipItem, HistoryStats};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::SecondsFormat;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

pub struct StorageService {
    base_dir: PathBuf,
    data_file: PathBuf,
    attachments_dir: PathBuf,
    max_capacity: usize,
    items: Mutex<Vec<ClipItem>>,
}

impl StorageService {
    pub fn new() -> Self {
        let base_dir = Self::default_base_dir();
        Self::with_base_dir(base_dir, 1000)
    }

    pub fn with_base_dir(base_dir: PathBuf, max_capacity: usize) -> Self {
        let data_file = base_dir.join("clips.json");
        let attachments_dir = base_dir.join("attachments");

        let _ = fs::create_dir_all(&base_dir);
        let _ = fs::create_dir_all(&attachments_dir);

        let initial_items = Self::load_from_disk(&data_file);

        Self {
            base_dir,
            data_file,
            attachments_dir,
            max_capacity,
            items: Mutex::new(initial_items),
        }
    }

    fn default_base_dir() -> PathBuf {
        let data_root = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        let modern_dir = data_root.join("clip-flow");
        let legacy_dir = data_root.join("ClipFlow");

        if legacy_dir.exists() && !modern_dir.exists() {
            let _ = fs::rename(&legacy_dir, &modern_dir);
        }

        modern_dir
    }

    fn load_from_disk(data_file: &Path) -> Vec<ClipItem> {
        if !data_file.exists() {
            return Vec::new();
        }

        match fs::read(data_file) {
            Ok(bytes) => serde_json::from_slice::<Vec<ClipItem>>(&bytes).unwrap_or_else(|err| {
                eprintln!("[StorageService] Failed to parse clips.json: {err}. Starting fresh.");
                Vec::new()
            }),
            Err(err) => {
                eprintln!("[StorageService] Failed to read clips.json: {err}. Starting fresh.");
                Vec::new()
            }
        }
    }

    fn persist_locked(&self, items: &[ClipItem]) {
        let _ = fs::create_dir_all(&self.base_dir);
        match serde_json::to_vec_pretty(items) {
            Ok(bytes) => {
                let tmp_path = self.base_dir.join(format!("clips.{}.tmp", Uuid::new_v4()));
                if let Err(err) = fs::write(&tmp_path, bytes) {
                    eprintln!("[StorageService] Failed to write temp clips file: {err}");
                    return;
                }
                if let Err(err) = fs::rename(&tmp_path, &self.data_file) {
                    eprintln!("[StorageService] Failed to atomically replace clips.json: {err}");
                    let _ = fs::remove_file(&tmp_path);
                }
            }
            Err(err) => {
                eprintln!("[StorageService] Failed to serialize clips: {err}");
            }
        }
    }

    pub fn save(&self, mut item: ClipItem) -> ClipItem {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        let now = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

        if let Some(existing_idx) = guard
            .iter()
            .position(|existing| existing.content_hash == item.content_hash)
        {
            let mut existing = guard.remove(existing_idx);
            existing.created_at = now;
            if let Some(source_name) = item.source_app_name {
                existing.source_app_name = Some(source_name);
            }
            if let Some(source_bundle) = item.source_app_bundle_id {
                existing.source_app_bundle_id = Some(source_bundle);
            }
            guard.insert(0, existing.clone());
            self.persist_locked(&guard);
            return existing;
        }

        item.created_at = now;
        guard.insert(0, item.clone());

        while guard.len() > self.max_capacity {
            if let Some(last_unpinned) = guard.iter().rposition(|entry| !entry.is_pinned) {
                let removed = guard.remove(last_unpinned);
                self.delete_attachment_if_any(&removed);
            } else {
                break;
            }
        }

        self.persist_locked(&guard);
        item
    }

    pub fn get_by_id(&self, id: &str) -> Option<ClipItem> {
        let guard = self.items.lock().expect("storage mutex poisoned");
        guard.iter().find(|item| item.id == id).cloned()
    }

    pub fn has_hash(&self, content_hash: &str) -> bool {
        let guard = self.items.lock().expect("storage mutex poisoned");
        guard.iter().any(|item| item.content_hash == content_hash)
    }

    pub fn fetch_history(
        &self,
        query: Option<&str>,
        content_type: Option<ClipContentType>,
        pinned_only: bool,
        limit: usize,
    ) -> Vec<ClipItem> {
        let guard = self.items.lock().expect("storage mutex poisoned");
        let normalized_query = query
            .map(|q| q.trim().to_lowercase())
            .filter(|q| !q.is_empty());

        let mut filtered: Vec<ClipItem> = guard
            .iter()
            .filter(|item| !pinned_only || item.is_pinned)
            .filter(|item| content_type.map_or(true, |t| item.content_type == t))
            .filter(|item| {
                let Some(ref q) = normalized_query else {
                    return true;
                };
                if item
                    .text
                    .as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(q))
                {
                    return true;
                }
                if item
                    .file_paths
                    .as_ref()
                    .is_some_and(|paths| paths.iter().any(|p| p.to_lowercase().contains(q)))
                {
                    return true;
                }
                if item
                    .source_app_name
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains(q))
                {
                    return true;
                }
                false
            })
            .cloned()
            .collect();

        filtered.sort_by(|a, b| {
            b.is_pinned
                .cmp(&a.is_pinned)
                .then_with(|| b.created_at.cmp(&a.created_at))
        });

        filtered.truncate(limit);
        filtered
    }

    pub fn stats(&self) -> HistoryStats {
        let guard = self.items.lock().expect("storage mutex poisoned");
        let total = guard.len();
        let pinned = guard.iter().filter(|item| item.is_pinned).count();
        HistoryStats { total, pinned }
    }

    pub fn toggle_pin(&self, id: &str) -> bool {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        if let Some(item) = guard.iter_mut().find(|item| item.id == id) {
            item.is_pinned = !item.is_pinned;
            self.persist_locked(&guard);
            true
        } else {
            false
        }
    }

    pub fn delete(&self, id: &str) -> bool {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        if let Some(idx) = guard.iter().position(|item| item.id == id) {
            let removed = guard.remove(idx);
            self.delete_attachment_if_any(&removed);
            self.persist_locked(&guard);
            true
        } else {
            false
        }
    }

    pub fn clear_unpinned(&self) {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        let mut retained = Vec::with_capacity(guard.len());
        for item in guard.drain(..) {
            if item.is_pinned {
                retained.push(item);
            } else {
                self.delete_attachment_if_any(&item);
            }
        }
        *guard = retained;
        self.persist_locked(&guard);
    }

    pub fn clear_all(&self) {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        for item in guard.drain(..) {
            self.delete_attachment_if_any(&item);
        }
        self.persist_locked(&guard);
    }

    pub fn save_image_png(&self, png_bytes: &[u8]) -> Option<String> {
        let _ = fs::create_dir_all(&self.attachments_dir);
        let file_name = format!("{}.png", Uuid::new_v4());
        let file_path = self.attachments_dir.join(&file_name);
        let tmp_path = self
            .attachments_dir
            .join(format!("{}.tmp", Uuid::new_v4()));

        if let Err(err) = fs::write(&tmp_path, png_bytes) {
            eprintln!("[StorageService] Failed to save temp image: {err}");
            return None;
        }
        if let Err(err) = fs::rename(&tmp_path, &file_path) {
            eprintln!("[StorageService] Failed to save image: {err}");
            let _ = fs::remove_file(&tmp_path);
            return None;
        }

        Some(file_name)
    }

    pub fn get_attachment_path(&self, file_name: &str) -> Option<PathBuf> {
        let mut components = Path::new(file_name).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(single)), None) => Some(self.attachments_dir.join(single)),
            _ => None,
        }
    }

    pub fn read_attachment_bytes(&self, file_name: &str) -> Option<Vec<u8>> {
        let path = self.get_attachment_path(file_name)?;
        fs::read(path).ok()
    }

    pub fn get_attachment_base64(&self, file_name: &str) -> Option<String> {
        let bytes = self.read_attachment_bytes(file_name)?;
        Some(format!("data:image/png;base64,{}", BASE64.encode(bytes)))
    }

    fn delete_attachment_if_any(&self, item: &ClipItem) {
        if let Some(ref file_name) = item.image_file_name {
            if let Some(path) = self.get_attachment_path(file_name) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

impl Default for StorageService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_storage(max_capacity: usize) -> (StorageService, PathBuf) {
        let dir = std::env::temp_dir().join(format!("clipflow-test-{}", Uuid::new_v4()));
        let storage = StorageService::with_base_dir(dir.clone(), max_capacity);
        (storage, dir)
    }

    #[test]
    fn test_save_deduplication_and_pin_order() {
        let (storage, dir) = temp_storage(10);

        let a = ClipItem::new(
            ClipContentType::Text,
            Some("alpha".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("Notes".into()),
            None,
        );
        let b = ClipItem::new(
            ClipContentType::Text,
            Some("beta".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("Safari".into()),
            None,
        );
        let a_dup = ClipItem::new(
            ClipContentType::Text,
            Some("alpha".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("Terminal".into()),
            None,
        );

        let saved_a = storage.save(a);
        let saved_b = storage.save(b);
        let saved_a2 = storage.save(a_dup);

        assert_eq!(saved_a.id, saved_a2.id);
        assert_eq!(saved_a2.source_app_name.as_deref(), Some("Terminal"));

        let history = storage.fetch_history(None, None, false, 10);
        assert_eq!(history.len(), 2);

        storage.toggle_pin(&saved_b.id);
        let pinned = storage.fetch_history(None, None, true, 10);
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].id, saved_b.id);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_capacity_trimming_preserves_pinned_and_cleans_attachments() {
        let (storage, dir) = temp_storage(2);

        let png_name = storage.save_image_png(&[1, 2, 3, 4]).unwrap();
        let img_path = storage.get_attachment_path(&png_name).unwrap();
        assert!(img_path.exists());

        let img_item = ClipItem::new(
            ClipContentType::Image,
            None,
            None,
            Some(png_name),
            Some(10),
            Some(10),
            None,
            None,
            None,
            None,
        );
        let pinned_item = ClipItem::new(
            ClipContentType::Text,
            Some("keep me".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let newest_item = ClipItem::new(
            ClipContentType::Text,
            Some("newest".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        storage.save(img_item);
        let saved_pinned = storage.save(pinned_item);
        storage.toggle_pin(&saved_pinned.id);
        storage.save(newest_item);

        let history = storage.fetch_history(None, None, false, 10);
        assert_eq!(history.len(), 2);
        assert!(history.iter().any(|i| i.id == saved_pinned.id));
        assert!(!img_path.exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_search_and_clear_unpinned() {
        let (storage, dir) = temp_storage(10);

        let item1 = storage.save(ClipItem::new(
            ClipContentType::Text,
            Some("hello rust world".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("VS Code".into()),
            None,
        ));
        let _item2 = storage.save(ClipItem::new(
            ClipContentType::Color,
            Some("#0A84FF".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            Some("Figma".into()),
            None,
        ));

        storage.toggle_pin(&item1.id);

        let search_res = storage.fetch_history(Some("RUST"), None, false, 10);
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].id, item1.id);

        let color_res = storage.fetch_history(None, Some(ClipContentType::Color), false, 10);
        assert_eq!(color_res.len(), 1);

        storage.clear_unpinned();
        let remaining = storage.fetch_history(None, None, false, 10);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, item1.id);

        let _ = fs::remove_dir_all(dir);
    }
}
