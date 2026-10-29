//! Linux: simulated Ctrl+V through an external tool, picked by session type.
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq, Clone, Copy)]
enum Session {
    Wayland,
    X11,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Tool {
    Wtype,
    Ydotool,
    Xdotool,
}

impl Tool {
    fn program(self) -> &'static str {
        match self {
            Tool::Wtype => "wtype",
            Tool::Ydotool => "ydotool",
            Tool::Xdotool => "xdotool",
        }
    }

    fn args(self) -> &'static [&'static str] {
        match self {
            Tool::Wtype => &["-M", "ctrl", "v", "-m", "ctrl"],
            // evdev codes: 29 = left ctrl, 47 = v
            Tool::Ydotool => &["key", "29:1", "47:1", "47:0", "29:0"],
            Tool::Xdotool => &["key", "--clearmodifiers", "ctrl+v"],
        }
    }
}

fn session(xdg_session_type: Option<&str>, wayland_display: Option<&str>, display: Option<&str>) -> Option<Session> {
    let set = |v: Option<&str>| v.is_some_and(|v| !v.is_empty());
    match xdg_session_type {
        Some("wayland") => return Some(Session::Wayland),
        Some("x11") => return Some(Session::X11),
        _ => {}
    }
    if set(wayland_display) {
        Some(Session::Wayland)
    } else if set(display) {
        Some(Session::X11)
    } else {
        None
    }
}

/// Candidates in order of preference.
fn candidates(session: Session) -> &'static [Tool] {
    match session {
        Session::Wayland => &[Tool::Wtype, Tool::Ydotool],
        Session::X11 => &[Tool::Xdotool, Tool::Ydotool],
    }
}

fn in_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
}

/// Sends Ctrl+V to the focused window with the first available tool.
pub fn paste() -> Result<(), String> {
    let var = |k: &str| std::env::var(k).ok();
    let (xdg, wl, x) = (var("XDG_SESSION_TYPE"), var("WAYLAND_DISPLAY"), var("DISPLAY"));
    let session = session(xdg.as_deref(), wl.as_deref(), x.as_deref())
        .ok_or("no graphical session found (WAYLAND_DISPLAY and DISPLAY are unset)")?;
    let tools = candidates(session);
    let tool = tools.iter().copied().find(|t| in_path(t.program())).ok_or_else(|| {
        let names: Vec<_> = tools.iter().map(|t| t.program()).collect();
        format!("clipboard converted; install one of {} to paste automatically, or press Ctrl+V yourself", names.join(", "))
    })?;
    let status = Command::new(tool.program())
        .args(tool.args())
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("could not run {}: {e}", tool.program()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} failed ({status})", tool.program()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_session() {
        assert_eq!(session(Some("wayland"), None, Some(":0")), Some(Session::Wayland));
        assert_eq!(session(Some("x11"), Some("wayland-0"), Some(":0")), Some(Session::X11));
        assert_eq!(session(None, Some("wayland-1"), None), Some(Session::Wayland));
        assert_eq!(session(Some("tty"), None, Some(":0")), Some(Session::X11));
        assert_eq!(session(None, Some(""), Some(":0")), Some(Session::X11));
        assert_eq!(session(None, None, None), None);
    }

    #[test]
    fn tool_order() {
        assert_eq!(candidates(Session::Wayland)[0], Tool::Wtype);
        assert_eq!(candidates(Session::X11)[0], Tool::Xdotool);
        assert_eq!(Tool::Xdotool.args().last(), Some(&"ctrl+v"));
    }
}
