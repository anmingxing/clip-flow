use super::FrontmostAppInfo;
use std::process::Command;

pub fn get_frontmost_app() -> Option<FrontmostAppInfo> {
    let output = Command::new("xdotool")
        .args(["getactivewindow"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let win_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let window_handle = win_str.parse::<isize>().unwrap_or(0);

    let pid = Command::new("xdotool")
        .args(["getwindowpid", &win_str])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<i32>()
                .ok()
        })
        .unwrap_or(0);

    let name = Command::new("xdotool")
        .args(["getwindowname", &win_str])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    Some(FrontmostAppInfo {
        bundle_id: None,
        name,
        pid,
        window_handle,
    })
}

pub fn activate_app(app_info: &FrontmostAppInfo) -> bool {
    if app_info.window_handle <= 0 {
        return false;
    }
    Command::new("xdotool")
        .args(["windowactivate", "--sync", &app_info.window_handle.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn get_clipboard_change_count() -> i64 {
    // Linux X11/Wayland polls arboard content directly when 0 is returned.
    0
}

pub fn is_clipboard_concealed() -> bool {
    false
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

pub fn simulate_paste_shortcut() {
    let _ = Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .spawn();
}
