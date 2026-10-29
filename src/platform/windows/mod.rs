pub mod autostart;
mod notify;
mod paste;

pub use notify::Notifier;
pub use paste::paste;

use crate::app::App;
use std::path::PathBuf;
use std::ptr::null;
use std::sync::{mpsc, Arc};
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentThreadId};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const DEFAULT_HOTKEY: &str = "Win+Alt+V";

pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("clip-md-html")
}

/// Named mutex: `None` if another instance is already running.
pub fn single_instance() -> Option<()> {
    let name: Vec<u16> = "Local\\clip-md-html-single-instance\0".encode_utf16().collect();
    unsafe {
        // The handle is intentionally leaked so the mutex lives as long as the process.
        let h = CreateMutexW(null(), 0, name.as_ptr());
        if h.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
            return None;
        }
    }
    Some(())
}

/// Win32 message loop; tray-icon and global-hotkey deliver their events through it.
pub fn run() {
    let (tx, rx) = mpsc::channel();
    let tid = unsafe { GetCurrentThreadId() };
    let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || unsafe {
        PostThreadMessageW(tid, WM_APP, 0, 0);
    });
    let mut app = App::new(tx, wake);
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    loop {
        let r = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
        if r <= 0 {
            return;
        }
        if msg.message == WM_TIMER && msg.hwnd.is_null() && app.notifier.owns_timer(msg.wParam) {
            app.notifier.cleanup();
            continue;
        }
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        while let Ok(m) = rx.try_recv() {
            if app.handle(m) {
                return;
            }
        }
    }
}

pub fn paste_permission_problem() -> Option<String> {
    None
}
