use std::ffi::c_void;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreateKeyboardEvent(source: *const c_void, keycode: u16, key_down: bool) -> *mut c_void;
    fn CGEventSetFlags(event: *mut c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut c_void);
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(cf: *const c_void);
}
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

const HID_EVENT_TAP: u32 = 0;
const KEYCODE_V: u16 = 9;
const FLAG_COMMAND: u64 = 0x0010_0000;

const HELP: &str = "Autorisez clip-md-html dans Réglages Système > Confidentialité et sécurité > Accessibilité \
pour permettre le collage automatique, puis relancez l'application.";

/// Message d'aide si l'autorisation Accessibilité manque (nécessaire pour simuler Cmd+V).
pub fn paste_permission_problem() -> Option<String> {
    if unsafe { AXIsProcessTrusted() } {
        None
    } else {
        Some(HELP.to_string())
    }
}

fn post_v(down: bool) {
    unsafe {
        let ev = CGEventCreateKeyboardEvent(std::ptr::null(), KEYCODE_V, down);
        if ev.is_null() {
            return;
        }
        CGEventSetFlags(ev, FLAG_COMMAND);
        CGEventPost(HID_EVENT_TAP, ev);
        CFRelease(ev);
    }
}
/// Envoie Cmd+V. Les flags sont fixés explicitement sur chaque événement (Cmd seul), donc
/// Option/Maj encore enfoncés physiquement n'altèrent pas le raccourci ; aucune lecture de l'état du clavier.
pub fn paste() -> Result<(), String> {
    if let Some(msg) = paste_permission_problem() {
        return Err(format!("Texte converti, mais collage automatique impossible. {msg}"));
    }
    std::thread::sleep(std::time::Duration::from_millis(80));
    post_v(true);
    post_v(false);
    Ok(())
}
