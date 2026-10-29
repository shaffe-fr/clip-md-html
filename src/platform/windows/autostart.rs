use std::ptr::null_mut;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::*;

const SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "clip-md-html";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn is_enabled() -> bool {
    unsafe {
        let mut key: HKEY = null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, wide(SUBKEY).as_ptr(), 0, KEY_READ, &mut key) != ERROR_SUCCESS {
            return false;
        }
        let r = RegQueryValueExW(key, wide(VALUE).as_ptr(), null_mut(), null_mut(), null_mut(), null_mut());
        RegCloseKey(key);
        r == ERROR_SUCCESS
    }
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    unsafe {
        let mut key: HKEY = null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, wide(SUBKEY).as_ptr(), 0, KEY_SET_VALUE, &mut key) != ERROR_SUCCESS {
            return Err("cannot open the Run registry key".into());
        }
        let r = if enabled {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let cmd = wide(&format!("\"{}\"", exe.display()));
            RegSetValueExW(key, wide(VALUE).as_ptr(), 0, REG_SZ, cmd.as_ptr() as *const u8, (cmd.len() * 2) as u32)
        } else {
            let r = RegDeleteValueW(key, wide(VALUE).as_ptr());
            if r == 2 { ERROR_SUCCESS } else { r } // ERROR_FILE_NOT_FOUND: already disabled
        };
        RegCloseKey(key);
        if r == ERROR_SUCCESS { Ok(()) } else { Err(format!("registry error {r}")) }
    }
}
