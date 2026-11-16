use super::file_paths::normalize_file_paths;
use clipboard_rs::{Clipboard, ClipboardContent, ClipboardContext};
use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "windows")]
const RTF_FORMAT: &str = "Rich Text Format";
#[cfg(target_os = "linux")]
const RTF_FORMAT: &str = "text/rtf";

// X11 needs the selection owner/event loop to survive the write operation.
// A shared context also serializes multi-format writes from IPC and polling.
fn with_clipboard<T>(action: impl FnOnce(&ClipboardContext) -> T) -> Option<T> {
    static CONTEXT: OnceLock<Mutex<Option<ClipboardContext>>> = OnceLock::new();
    let mut guard = CONTEXT.get_or_init(|| Mutex::new(None)).lock().ok()?;
    if guard.is_none() {
        *guard = ClipboardContext::new().ok();
    }
    guard.as_ref().map(action)
}

pub fn read_platform_files_and_rtf() -> (Option<Vec<String>>, Option<Vec<u8>>) {
    with_clipboard(|clipboard| {
        let files = normalize_file_paths(clipboard.get_files().unwrap_or_default());
        let rtf = clipboard
            .get_buffer(RTF_FORMAT)
            .ok()
            .filter(|data| !data.is_empty());
        ((!files.is_empty()).then_some(files), rtf)
    })
    .unwrap_or_default()
}

pub fn write_platform_files_and_rtf(
    files: Option<&[String]>,
    rtf: Option<&[u8]>,
    text: Option<&str>,
) -> bool {
    let mut contents = Vec::new();
    if let Some(files) = files.filter(|files| !files.is_empty()) {
        #[cfg(target_os = "linux")]
        let files = match super::file_paths::file_urls(files) {
            Some(files) => files,
            None => return false,
        };
        #[cfg(target_os = "windows")]
        let files = files.to_vec();
        contents.push(ClipboardContent::Files(files));
    }
    if let Some(rtf) = rtf {
        contents.push(ClipboardContent::Other(RTF_FORMAT.into(), rtf.to_vec()));
    }
    if let Some(text) = text {
        contents.push(ClipboardContent::Text(text.into()));
    }
    if contents.is_empty() {
        return false;
    }
    with_clipboard(|clipboard| clipboard.set(contents).is_ok()).unwrap_or(false)
}
