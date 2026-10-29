use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

const DUMMY_VK: u16 = 0xE8; // unassigned key: stops a lone Alt/Win release from opening a menu

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
        },
    }
}

/// Sends Ctrl+V to the foreground window in a single `SendInput` batch.
///
/// The hotkey modifiers are still physically held when it fires, so the batch releases them
/// explicitly (after a dummy key) before Ctrl+V. No keyboard state reads and no hooks, which
/// keeps antivirus heuristics quiet.
pub fn paste() -> Result<(), String> {
    let releases = [VK_LWIN, VK_RWIN, VK_LMENU, VK_RMENU, VK_LSHIFT, VK_RSHIFT, VK_LCONTROL, VK_RCONTROL];
    let mut inputs = vec![key(DUMMY_VK, false), key(DUMMY_VK, true)];
    inputs.extend(releases.iter().map(|&vk| key(vk, true)));
    inputs.extend([key(VK_CONTROL, false), key(b'V' as u16, false), key(b'V' as u16, true), key(VK_CONTROL, true)]);
    let sent = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err("automatic paste blocked by Windows (elevated target window?)".into())
    }
}
