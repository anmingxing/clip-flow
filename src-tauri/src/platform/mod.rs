#[derive(Debug, Clone, Default)]
pub struct FrontmostAppInfo {
    pub bundle_id: Option<String>,
    pub name: Option<String>,
    pub pid: i32,
    #[allow(dead_code)]
    pub window_handle: isize,
}

pub fn is_self_app(app: &FrontmostAppInfo) -> bool {
    if app.pid > 0 && app.pid as u32 == std::process::id() {
        return true;
    }
    if let Some(ref bundle_id) = app.bundle_id {
        if bundle_id == "com.anmingxing.clip-flow" {
            return true;
        }
    }
    false
}

#[cfg(any(target_os = "windows", target_os = "linux", test))]
mod file_paths;
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod native_clipboard;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod fallback {
    use super::FrontmostAppInfo;

    pub fn get_frontmost_app() -> Option<FrontmostAppInfo> {
        None
    }

    pub fn activate_app(_app: &FrontmostAppInfo) -> bool {
        false
    }

    pub fn is_clipboard_concealed() -> bool {
        false
    }

    pub fn get_clipboard_change_count() -> i64 {
        0
    }

    pub fn read_platform_files_and_rtf() -> (Option<Vec<String>>, Option<Vec<u8>>) {
        (None, None)
    }

    pub fn write_platform_files_and_rtf(
        _files: Option<&[String]>,
        _rtf: Option<&[u8]>,
        _text: Option<&str>,
    ) -> bool {
        false
    }

    pub fn is_accessibility_trusted(_prompt: bool) -> bool {
        true
    }

    pub fn open_accessibility_settings() {}

    pub fn simulate_paste_shortcut() {}
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub use fallback::*;
