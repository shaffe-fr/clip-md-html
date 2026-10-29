use std::time::{Duration, Instant};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

const DUMMY_VK: u16 = 0xE8; // touche non assignée, sert à neutraliser l'activation du menu par Alt

fn is_down(vk: u16) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    }
}

fn send(inputs: &[INPUT]) {
    unsafe {
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

/// Attend le relâchement de Win/Alt/Maj/Ctrl ; au-delà du délai, les relâche synthétiquement.
fn release_modifiers() {
    const MODS: [u16; 8] = [VK_LWIN, VK_RWIN, VK_LMENU, VK_RMENU, VK_LSHIFT, VK_RSHIFT, VK_LCONTROL, VK_RCONTROL];
    let deadline = Instant::now() + Duration::from_millis(600);
    while Instant::now() < deadline && MODS.iter().any(|&m| is_down(m)) {
        std::thread::sleep(Duration::from_millis(10));
    }
    for &m in MODS.iter().filter(|&&m| is_down(m)) {
        // Touche factice avant de relâcher Alt/Win : évite l'ouverture d'un menu / du menu Démarrer.
        if matches!(m, VK_LMENU | VK_RMENU | VK_LWIN | VK_RWIN) {
            send(&[key(DUMMY_VK, false), key(DUMMY_VK, true)]);
        }
        send(&[key(m, true)]);
    }
}

/// Envoie Ctrl+V à la fenêtre au premier plan.
pub fn paste() -> Result<(), String> {
    release_modifiers();
    std::thread::sleep(Duration::from_millis(30));
    send(&[key(VK_CONTROL, false), key(b'V' as u16, false), key(b'V' as u16, true), key(VK_CONTROL, true)]);
    Ok(())
}
