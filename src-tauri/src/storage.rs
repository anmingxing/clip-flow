use crate::models::{ClipContentType, ClipItem, HistoryStats};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::SecondsFormat;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

pub struct StorageService {
    base_dir: PathBuf,
    data_file: PathBuf,
    attachments_dir: PathBuf,
    max_capacity: usize,
    items: Mutex<Vec<ClipItem>>,
    load_error: Option<String>,
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

        let (initial_items, load_error) = match Self::load_from_disk(&data_file) {
            Ok(items) => (items, None),
            Err(err) => {
                eprintln!("[StorageService] {err}");
                (Vec::new(), Some(err))
            }
        };

        Self {
            base_dir,
            data_file,
            attachments_dir,
            max_capacity,
            items: Mutex::new(initial_items),
            load_error,
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

    fn load_from_disk(data_file: &Path) -> Result<Vec<ClipItem>, String> {
        let bytes = match fs::read(data_file) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(format!("Cannot read history; writes disabled: {err}")),
        };
        match serde_json::from_slice::<Vec<ClipItem>>(&bytes) {
            Ok(mut items) => {
                // Migrate hashes written before RTF was included in deduplication.
                for item in &mut items {
                    if item.content_type == ClipContentType::RichText {
                        item.content_hash = ClipItem::rich_text_hash(
                            item.text.as_deref(),
                            item.rich_text_bytes().as_deref(),
                        );
                    }
                }
                Ok(items)
            }
            Err(err) => {
                let backup =
                    data_file.with_file_name(format!("clips.recovery.{}.json", Uuid::new_v4()));
                // Never overwrite an unreadable history unless a durable recovery
                // copy has been created. Keep attachments for manual recovery too.
                let mut file = File::create(&backup)
                    .map_err(|e| format!("History parse failed ({err}); cannot back up: {e}"))?;
                file.write_all(&bytes)
                    .and_then(|()| file.sync_all())
                    .map_err(|e| {
                        format!("Cannot preserve damaged history; writes disabled: {e}")
                    })?;
                eprintln!(
                    "[StorageService] Invalid history ({err}); preserved at {}",
                    backup.display()
                );
                Ok(Vec::new())
            }
        }
    }

    fn persist_locked(&self, items: &[ClipItem]) -> Result<(), String> {
        if let Some(ref err) = self.load_error {
            return Err(err.clone());
        }
        fs::create_dir_all(&self.base_dir)
            .map_err(|e| format!("Cannot create history directory: {e}"))?;
        let bytes =
            serde_json::to_vec_pretty(items).map_err(|e| format!("Cannot encode history: {e}"))?;
        let tmp_path = self.base_dir.join(format!("clips.{}.tmp", Uuid::new_v4()));
        let result = (|| {
            let mut file = File::create(&tmp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&tmp_path, &self.data_file)
        })();
        if let Err(err) = result {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("Cannot save history: {err}"));
        }
        Ok(())
    }

    pub fn save(&self, mut item: ClipItem) -> Result<ClipItem, String> {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        let mut next = guard.clone();
        let now = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

        if let Some(existing_idx) = next
            .iter()
            .position(|existing| existing.content_hash == item.content_hash)
        {
            let mut existing = next.remove(existing_idx);
            existing.created_at = now;
            if let Some(source_name) = item.source_app_name {
                existing.source_app_name = Some(source_name);
            }
            if let Some(source_bundle) = item.source_app_bundle_id {
                existing.source_app_bundle_id = Some(source_bundle);
            }
            next.insert(0, existing.clone());
            self.persist_locked(&next)?;
            *guard = next;
            return Ok(existing);
        }

        item.created_at = now;
        next.insert(0, item.clone());
        self.commit_locked(&mut guard, next)?;
        Ok(item)
    }

    fn commit_locked(
        &self,
        current: &mut Vec<ClipItem>,
        mut next: Vec<ClipItem>,
    ) -> Result<(), String> {
        let mut unpinned = 0;
        next.retain(|item| {
            if item.is_pinned {
                return true;
            }
            unpinned += 1;
            unpinned <= self.max_capacity
        });
        self.persist_locked(&next)?;
        for removed in current
            .iter()
            .filter(|old| !next.iter().any(|item| item.id == old.id))
        {
            self.delete_attachment_if_any(removed);
        }
        *current = next;
        Ok(())
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
            .filter(|item| content_type.is_none_or(|t| item.content_type == t))
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

    pub fn toggle_pin(&self, id: &str) -> Result<bool, String> {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        let mut next = guard.clone();
        if let Some(item) = next.iter_mut().find(|item| item.id == id) {
            item.is_pinned = !item.is_pinned;
            self.commit_locked(&mut guard, next)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        let mut next = guard.clone();
        if let Some(idx) = next.iter().position(|item| item.id == id) {
            next.remove(idx);
            self.commit_locked(&mut guard, next)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear_unpinned(&self) -> Result<(), String> {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        let retained = guard
            .iter()
            .filter(|item| item.is_pinned)
            .cloned()
            .collect();
        self.commit_locked(&mut guard, retained)
    }

    pub fn clear_all(&self) -> Result<(), String> {
        let mut guard = self.items.lock().expect("storage mutex poisoned");
        self.commit_locked(&mut guard, Vec::new())
    }

    pub fn save_image_png(&self, png_bytes: &[u8]) -> Option<String> {
        let _ = fs::create_dir_all(&self.attachments_dir);
        let file_name = format!("{}.png", Uuid::new_v4());
        let file_path = self.attachments_dir.join(&file_name);
        let tmp_path = self.attachments_dir.join(format!("{}.tmp", Uuid::new_v4()));

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

    fn text_item(text: &str) -> ClipItem {
        ClipItem::new(
            ClipContentType::Text,
            Some(text.into()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    fn rich_item(rtf: &[u8]) -> ClipItem {
        ClipItem::new(
            ClipContentType::RichText,
            Some("hello".into()),
            Some(rtf.to_vec()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[test]
    fn corrupt_history_is_preserved_before_new_records_are_saved() {
        let (storage, dir) = temp_storage(10);
        drop(storage);
        let original = br#"[{"id":"old record","type":"unknown"}]"#;
        fs::write(dir.join("clips.json"), original).unwrap();
        let storage = StorageService::with_base_dir(dir.clone(), 10);
        storage.save(text_item("new copy")).unwrap();
        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("clips.recovery.")
            })
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(backups[0].path()).unwrap(), original);
        assert_eq!(
            StorageService::with_base_dir(dir.clone(), 10).stats().total,
            1
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn read_failure_disables_overwriting_history() {
        let (storage, dir) = temp_storage(10);
        drop(storage);
        fs::create_dir(dir.join("clips.json")).unwrap();
        let storage = StorageService::with_base_dir(dir.clone(), 10);
        assert!(storage.save(text_item("must not overwrite")).is_err());
        assert_eq!(storage.stats().total, 0);
        assert!(dir.join("clips.json").is_dir());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_persistence_preserves_memory_and_image_attachments() {
        let (storage, dir) = temp_storage(10);
        let name = storage.save_image_png(&[1, 2, 3]).unwrap();
        let path = storage.get_attachment_path(&name).unwrap();
        let item = storage
            .save(ClipItem::new(
                ClipContentType::Image,
                None,
                None,
                Some(name),
                Some(1),
                Some(1),
                None,
                None,
                None,
                None,
            ))
            .unwrap();
        fs::rename(dir.join("clips.json"), dir.join("original.json")).unwrap();
        fs::create_dir(dir.join("clips.json")).unwrap();
        assert!(storage.delete(&item.id).is_err());
        assert!(storage.toggle_pin(&item.id).is_err());
        assert!(storage.clear_unpinned().is_err());
        assert!(storage.clear_all().is_err());
        assert!(storage.save(text_item("failed save")).is_err());
        assert_eq!(storage.stats().total, 1);
        assert!(!storage.get_by_id(&item.id).unwrap().is_pinned);
        assert!(path.exists());
        assert!(!fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| entry.path().extension().is_some_and(|ext| ext == "tmp")));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn different_rtf_formats_and_legacy_hashes_preserve_content() {
        let (storage, dir) = temp_storage(10);
        let mut legacy = rich_item(br"{\rtf1\b hello}");
        legacy.content_hash = ClipItem::calculate_hash(
            ClipContentType::RichText,
            legacy.text.as_deref(),
            None,
            None,
        );
        let id = legacy.id.clone();
        fs::write(
            dir.join("clips.json"),
            serde_json::to_vec(&vec![legacy]).unwrap(),
        )
        .unwrap();
        drop(storage);
        let storage = StorageService::with_base_dir(dir.clone(), 10);
        let same = storage.save(rich_item(br"{\rtf1\b hello}")).unwrap();
        assert_eq!(same.id, id);
        let italic = storage.save(rich_item(br"{\rtf1\i hello}")).unwrap();
        assert_ne!(italic.id, id);
        assert_eq!(italic.rich_text_bytes().unwrap(), br"{\rtf1\i hello}");
        assert_eq!(storage.stats().total, 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pinned_capacity_does_not_discard_new_copies() {
        let (storage, dir) = temp_storage(2);
        for text in ["pin one", "pin two"] {
            let item = storage.save(text_item(text)).unwrap();
            storage.toggle_pin(&item.id).unwrap();
        }
        let old = storage.save(text_item("old unpinned")).unwrap();
        storage.save(text_item("recent unpinned")).unwrap();
        let newest = storage.save(text_item("newest unpinned")).unwrap();
        assert_eq!(storage.stats().total, 4);
        assert_eq!(storage.stats().pinned, 2);
        assert!(storage.get_by_id(&old.id).is_none());
        assert!(storage.get_by_id(&newest.id).is_some());
        let pin = storage.fetch_history(None, None, true, 10)[0].clone();
        storage.toggle_pin(&pin.id).unwrap();
        let stats = storage.stats();
        assert_eq!(stats.total - stats.pinned, 2);
        fs::remove_dir_all(dir).unwrap();
    }

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

        let saved_a = storage.save(a).unwrap();
        let saved_b = storage.save(b).unwrap();
        let saved_a2 = storage.save(a_dup).unwrap();

        assert_eq!(saved_a.id, saved_a2.id);
        assert_eq!(saved_a2.source_app_name.as_deref(), Some("Terminal"));

        let history = storage.fetch_history(None, None, false, 10);
        assert_eq!(history.len(), 2);

        storage.toggle_pin(&saved_b.id).unwrap();
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

        storage.save(img_item).unwrap();
        let saved_pinned = storage.save(pinned_item).unwrap();
        storage.toggle_pin(&saved_pinned.id).unwrap();
        storage.save(newest_item).unwrap();
        storage.save(text_item("overflow")).unwrap();

        let history = storage.fetch_history(None, None, false, 10);
        assert_eq!(history.len(), 3);
        assert!(history.iter().any(|i| i.id == saved_pinned.id));
        assert!(!img_path.exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_search_and_clear_unpinned() {
        let (storage, dir) = temp_storage(10);

        let item1 = storage
            .save(ClipItem::new(
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
            ))
            .unwrap();
        let _item2 = storage
            .save(ClipItem::new(
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
            ))
            .unwrap();

        storage.toggle_pin(&item1.id).unwrap();

        let search_res = storage.fetch_history(Some("RUST"), None, false, 10);
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].id, item1.id);

        let color_res = storage.fetch_history(None, Some(ClipContentType::Color), false, 10);
        assert_eq!(color_res.len(), 1);

        storage.clear_unpinned().unwrap();
        let remaining = storage.fetch_history(None, None, false, 10);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, item1.id);

        let _ = fs::remove_dir_all(dir);
    }
}
