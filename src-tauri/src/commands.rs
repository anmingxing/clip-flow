use crate::models::{ClipContentType, ClipItem, HistoryStats};
use crate::{hide_main_window, platform, AppState};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_history(
    state: State<'_, AppState>,
    query: Option<String>,
    content_type: Option<ClipContentType>,
    pinned_only: bool,
    limit: Option<usize>,
) -> Vec<ClipItem> {
    state
        .storage
        .fetch_history(query.as_deref(), content_type, pinned_only, limit.unwrap_or(100))
}

#[tauri::command]
pub fn get_stats(state: State<'_, AppState>) -> HistoryStats {
    state.storage.stats()
}

#[tauri::command]
pub fn get_attachment_base64(state: State<'_, AppState>, file_name: String) -> Option<String> {
    state.storage.get_attachment_base64(&file_name)
}

#[tauri::command]
pub fn paste_item(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
    plain_text_only: bool,
    auto_paste: bool,
) -> Result<(), String> {
    state
        .paste
        .paste_item(&app_handle, &id, plain_text_only, auto_paste)
}

#[tauri::command]
pub fn copy_text(state: State<'_, AppState>, text: String) -> Result<(), String> {
    state.paste.copy_plain_text(&text)
}

#[tauri::command]
pub fn toggle_pin(state: State<'_, AppState>, id: String) -> bool {
    state.storage.toggle_pin(&id)
}

#[tauri::command]
pub fn delete_item(state: State<'_, AppState>, id: String) -> bool {
    state.storage.delete(&id)
}

#[tauri::command]
pub fn clear_unpinned(state: State<'_, AppState>) {
    state.storage.clear_unpinned();
}

#[tauri::command]
pub fn check_accessibility_permission() -> bool {
    platform::is_accessibility_trusted(false)
}

#[tauri::command]
pub fn request_accessibility_permission() -> bool {
    let trusted = platform::is_accessibility_trusted(true);
    if !trusted {
        platform::open_accessibility_settings();
    }
    trusted
}

#[tauri::command]
pub fn hide_window(app_handle: AppHandle) {
    hide_main_window(&app_handle);
}

#[tauri::command]
pub fn open_external_url(app_handle: AppHandle, url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("Only http/https URLs are allowed".to_string());
    }
    app_handle
        .opener()
        .open_url(trimmed, None::<&str>)
        .map_err(|e| format!("Failed to open URL: {e}"))?;
    hide_main_window(&app_handle);
    Ok(())
}
