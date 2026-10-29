use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const UID: u32 = 42;

pub struct Notifier {
    hwnd: isize,
    active: bool,
    timer: usize,
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
        Self { hwnd: hwnd as isize, active: false, timer: 0 }
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
                self.timer = SetTimer(null_mut(), 0, 7000, None);
            }
        }
    }

    /// Vrai si `id` est le timer de nettoyage en cours (l'ID est attribué par SetTimer).
    pub fn owns_timer(&self, id: usize) -> bool {
        self.timer != 0 && self.timer == id
    }

    pub fn cleanup(&mut self) {
        if self.active {
            let d = self.data();
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &d);
                if self.timer != 0 {
                    KillTimer(null_mut(), self.timer);
                }
            }
            self.active = false;
            self.timer = 0;
        }
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        self.cleanup();
    }
}
