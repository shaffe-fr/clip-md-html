#![cfg_attr(windows, windows_subsystem = "windows")]
#[cfg(any(windows, target_os = "macos"))]
mod app;
mod clipboard;
#[cfg(any(windows, target_os = "macos"))]
mod config;
mod convert;
#[cfg(any(windows, target_os = "macos"))]
mod icon;
#[cfg(any(windows, target_os = "macos"))]
mod icon_pixels;
mod platform;

#[cfg(any(windows, target_os = "macos"))]
fn main() {
    if std::env::args().any(|a| a == "--once") {
        if let Err(e) = clipboard::convert_clipboard() {
            config::log(&format!("--once: {e}"));
            std::process::exit(1);
        }
        return;
    }
    if platform::single_instance().is_none() {
        return;
    }
    platform::run();
}

#[cfg(target_os = "linux")]
const USAGE: &str = "\
clip-md-html converts the Markdown in the clipboard to HTML (text/html, plain text kept).

Usage:
  clip-md-html --once     convert the clipboard and return; a background process keeps serving it
  clip-md-html --paste    same, then send Ctrl+V with wtype, ydotool or xdotool

There is no tray icon or built-in hotkey on Linux: bind one of these commands to a custom
keyboard shortcut in your desktop environment (see the README).";

#[cfg(target_os = "linux")]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let result = if has(clipboard::HOLD_ARG) {
        clipboard::hold_clipboard()
    } else if has("--once") || has("--paste") {
        clipboard::convert_clipboard().and_then(|()| if has("--paste") { paste() } else { Ok(()) })
    } else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    if let Err(e) = result {
        eprintln!("clip-md-html: {e}");
        std::process::exit(1);
    }
}

/// Gives the freshly written clipboard a moment to propagate, then pastes.
#[cfg(target_os = "linux")]
fn paste() -> Result<(), String> {
    std::thread::sleep(std::time::Duration::from_millis(50));
    platform::paste()
}
