use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

const DUMMY_VK: u16 = 0xE8; // touche non assignée : empêche Alt/Win seuls d'ouvrir un menu

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    }
}

/// Envoie Ctrl+V à la fenêtre au premier plan en un seul appel `SendInput`.
///
/// Win/Alt/Maj sont encore physiquement enfoncés quand le raccourci se déclenche : on envoie
/// donc leur relâchement explicite dans le même lot, après une touche factice, puis Ctrl+V.
/// Aucune lecture de l'état du clavier, aucun hook.
pub fn paste() -> Result<(), String> {
    let releases = [VK_LWIN, VK_RWIN, VK_LMENU, VK_RMENU, VK_LSHIFT, VK_RSHIFT, VK_LCONTROL, VK_RCONTROL];
    let mut inputs = vec![key(DUMMY_VK, false), key(DUMMY_VK, true)];
    inputs.extend(releases.iter().map(|&vk| key(vk, true)));
    inputs.extend([key(VK_CONTROL, false), key(b'V' as u16, false), key(b'V' as u16, true), key(VK_CONTROL, true)]);
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err("collage automatique bloqué par Windows (fenêtre élevée ?)".into())
    }
}
