use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const TIMER_ID: usize = 0xC11B;
const UID: u32 = 42;

pub struct Notifier {
    hwnd: isize,
    active: bool,
}

fn fill(dst: &mut [u16], s: &str) {
    let max = dst.len() - 1;
    for (i, c) in s.encode_utf16().take(max).enumerate() {
        dst[i] = c;
    }
}

impl Notifier {
    pub fn new() -> Self {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let hwnd = unsafe {
            CreateWindowExW(0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0, null_mut(), null_mut(), null_mut(), null_mut())
        };
        Self { hwnd: hwnd as isize, active: false }
    }

    fn data(&self) -> NOTIFYICONDATAW {
        let mut d: NOTIFYICONDATAW = unsafe { zeroed() };
        d.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        d.hWnd = self.hwnd as _;
        d.uID = UID;
        d
    }

    /// Affiche un ballon Windows ; l'icône temporaire est retirée par `cleanup` (timer).
    pub fn show(&mut self, title: &str, msg: &str) {
        self.cleanup();
        let mut d = self.data();
        d.uFlags = NIF_ICON | NIF_TIP | NIF_INFO;
        d.hIcon = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };
        d.dwInfoFlags = NIIF_INFO;
        fill(&mut d.szTip, "clip-md-html");
        fill(&mut d.szInfoTitle, title);
        fill(&mut d.szInfo, msg);
        unsafe {
            if Shell_NotifyIconW(NIM_ADD, &d) != 0 {
                self.active = true;
                SetTimer(null_mut(), TIMER_ID, 7000, None);
            }
        }
    }

    pub fn cleanup(&mut self) {
        if self.active {
            let d = self.data();
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &d);
                KillTimer(null_mut(), TIMER_ID);
            }
            self.active = false;
        }
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        self.cleanup();
    }
}
