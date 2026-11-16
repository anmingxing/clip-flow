use super::FrontmostAppInfo;
use std::path::Path;
use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::System::DataExchange::{
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, RegisterClipboardFormatW,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
};

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn get_frontmost_app() -> Option<FrontmostAppInfo> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }

        let mut exe_name = None;
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !handle.is_null() {
            let mut buf = vec![0u16; 1024];
            let mut size = buf.len() as u32;
            if QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) != 0 && size > 0 {
                let full_path = String::from_utf16_lossy(&buf[..size as usize]);
                if let Some(stem) = Path::new(&full_path).file_stem().and_then(|s| s.to_str()) {
                    exe_name = Some(stem.to_string());
                }
            }
            CloseHandle(handle);
        }

        Some(FrontmostAppInfo {
            bundle_id: None,
            name: exe_name,
            pid: pid as i32,
            window_handle: hwnd as isize,
        })
    }
}

pub fn activate_app(app_info: &FrontmostAppInfo) -> bool {
    if app_info.window_handle == 0 {
        return false;
    }
    unsafe {
        let hwnd = app_info.window_handle as HWND;
        SetForegroundWindow(hwnd) != 0
    }
}

pub fn get_clipboard_change_count() -> i64 {
    unsafe { GetClipboardSequenceNumber() as i64 }
}

pub fn is_clipboard_concealed() -> bool {
    unsafe {
        let exclude_format = wide_null("ExcludeClipboardContentFromMonitorProcessing");
        let fmt_id = RegisterClipboardFormatW(exclude_format.as_ptr());
        if fmt_id != 0 && IsClipboardFormatAvailable(fmt_id) != 0 {
            return true;
        }

        let can_include_cloud = wide_null("CanIncludeInClipboardHistory");
        let _ = can_include_cloud;
        false
    }
}

pub use super::native_clipboard::{read_platform_files_and_rtf, write_platform_files_and_rtf};

pub fn is_accessibility_trusted(_prompt: bool) -> bool {
    true
}

pub fn open_accessibility_settings() {}

pub fn simulate_paste_shortcut() {
    const VK_V: u16 = 0x56;
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_V,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}
