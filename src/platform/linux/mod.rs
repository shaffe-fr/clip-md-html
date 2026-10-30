//! Linux: simulated Ctrl+V through an external tool, picked by session type.
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq, Clone, Copy)]
enum Session {
    Wayland,
    X11,
}

/// One way of invoking a paste tool. `ydotool` has two incompatible syntaxes, hence two entries.
#[derive(Debug, PartialEq, Clone, Copy)]
struct Attempt {
    program: &'static str,
    args: &'static [&'static str],
}

// Every attempt first releases the modifiers the user may still hold from the shortcut, then
// presses Ctrl+V.
const WTYPE: Attempt = Attempt {
    program: "wtype",
    args: &["-m", "logo", "-m", "alt", "-m", "shift", "-m", "ctrl", "-M", "ctrl", "v", "-m", "ctrl"],
};
// ydotool 1.x: evdev codes (125/126 meta, 56/100 alt, 42/54 shift, 29/97 ctrl, 47 = v).
const YDOTOOL_1X: Attempt = Attempt {
    program: "ydotool",
    args: &[
        "key", "125:0", "126:0", "56:0", "100:0", "42:0", "54:0", "29:0", "97:0", "29:1", "47:1", "47:0", "29:0",
    ],
};
// ydotool 0.1.x: key names.
const YDOTOOL_01X: Attempt = Attempt { program: "ydotool", args: &["key", "ctrl+v"] };
const XDOTOOL: Attempt = Attempt {
    program: "xdotool",
    args: &[
        "keyup", "Super_L", "Super_R", "Alt_L", "Alt_R", "Shift_L", "Shift_R", "Control_L", "Control_R", "key",
        "--clearmodifiers", "ctrl+v",
    ],
};

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

/// Attempts in order of preference.
fn candidates(session: Session) -> &'static [Attempt] {
    match session {
        Session::Wayland => &[WTYPE, YDOTOOL_1X, YDOTOOL_01X],
        Session::X11 => &[XDOTOOL, YDOTOOL_1X, YDOTOOL_01X],
    }
}

/// Environment access, so the selection logic can be tested without real tools.
trait Runner {
    fn available(&self, program: &str) -> bool;
    fn run(&self, attempt: &Attempt) -> Result<(), String>;
}

struct System;

impl Runner for System {
    fn available(&self, program: &str) -> bool {
        std::env::var_os("PATH")
            .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
    }

    fn run(&self, attempt: &Attempt) -> Result<(), String> {
        let out = Command::new(attempt.program)
            .args(attempt.args)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("could not run: {e}"))?;
        if out.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&out.stderr);
        let detail = stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
        Err(if detail.is_empty() { format!("failed ({})", out.status) } else { format!("failed ({}): {detail}", out.status) })
    }
}

/// Tries each installed candidate until one succeeds.
fn paste_with(session: Session, runner: &dyn Runner) -> Result<(), String> {
    let attempts = candidates(session);
    let mut failures: Vec<String> = Vec::new();
    for attempt in attempts.iter().filter(|a| runner.available(a.program)) {
        match runner.run(attempt) {
            Ok(()) => return Ok(()),
            Err(e) => failures.push(format!("{} {}: {e}", attempt.program, attempt.args[0])),
        }
    }
    if failures.is_empty() {
        let mut names: Vec<&str> = attempts.iter().map(|a| a.program).collect();
        names.dedup();
        return Err(format!(
            "clipboard converted; install one of {} to paste automatically, or press Ctrl+V yourself",
            names.join(", ")
        ));
    }
    Err(format!("clipboard converted but no paste tool worked (tried: {}); press Ctrl+V yourself", failures.join("; ")))
}

/// Sends Ctrl+V to the focused window with the first tool that works.
pub fn paste() -> Result<(), String> {
    let var = |k: &str| std::env::var(k).ok();
    let (xdg, wl, x) = (var("XDG_SESSION_TYPE"), var("WAYLAND_DISPLAY"), var("DISPLAY"));
    let session = session(xdg.as_deref(), wl.as_deref(), x.as_deref())
        .ok_or("no graphical session found (WAYLAND_DISPLAY and DISPLAY are unset)")?;
    paste_with(session, &System)
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

    struct Fake {
        installed: &'static [&'static str],
        /// Attempts (by args[1]) that fail; everything else succeeds.
        failing: Vec<&'static [&'static str]>,
        calls: std::cell::RefCell<Vec<&'static str>>,
    }

    impl Runner for Fake {
        fn available(&self, program: &str) -> bool {
            self.installed.contains(&program)
        }
        fn run(&self, attempt: &Attempt) -> Result<(), String> {
            self.calls.borrow_mut().push(attempt.program);
            if self.failing.contains(&attempt.args) { Err("failed (exit 1)".into()) } else { Ok(()) }
        }
    }

    fn fake(installed: &'static [&'static str], failing: Vec<&'static [&'static str]>) -> Fake {
        Fake { installed, failing, calls: Default::default() }
    }

    #[test]
    fn tool_order() {
        assert_eq!(candidates(Session::Wayland)[0], WTYPE);
        assert_eq!(candidates(Session::X11)[0], XDOTOOL);
        assert_eq!(XDOTOOL.args.last(), Some(&"ctrl+v"));
    }

    #[test]
    fn first_installed_tool_is_used() {
        let r = fake(&["wtype", "ydotool"], vec![]);
        assert!(paste_with(Session::Wayland, &r).is_ok());
        assert_eq!(*r.calls.borrow(), ["wtype"]);
    }

    #[test]
    fn failing_tool_falls_through_to_next() {
        let r = fake(&["wtype", "ydotool"], vec![WTYPE.args]);
        assert!(paste_with(Session::Wayland, &r).is_ok());
        assert_eq!(*r.calls.borrow(), ["wtype", "ydotool"]);
    }

    #[test]
    fn ydotool_falls_back_to_old_syntax() {
        let r = fake(&["ydotool"], vec![YDOTOOL_1X.args]);
        assert!(paste_with(Session::X11, &r).is_ok());
        assert_eq!(*r.calls.borrow(), ["ydotool", "ydotool"]);
    }

    #[test]
    fn all_failing_lists_what_was_tried() {
        let r = fake(&["wtype", "ydotool"], vec![WTYPE.args, YDOTOOL_1X.args, YDOTOOL_01X.args]);
        let e = paste_with(Session::Wayland, &r).unwrap_err();
        assert!(e.contains("no paste tool worked") && e.contains("wtype") && e.contains("ydotool"), "{e}");
    }

    #[test]
    fn nothing_installed_says_what_to_install() {
        let r = fake(&[], vec![]);
        let e = paste_with(Session::X11, &r).unwrap_err();
        assert!(e.contains("install one of xdotool, ydotool"), "{e}");
        assert!(r.calls.borrow().is_empty());
    }
}
