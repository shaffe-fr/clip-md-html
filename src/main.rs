#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod clipboard;
mod config;
mod convert;
mod icon;
mod platform;

fn main() {
    if std::env::args().any(|a| a == "--once") {
        if let Err(e) = clipboard::convert_clipboard() {
            config::log(&format!("--once : {e}"));
            std::process::exit(1);
        }
        return;
    }
    if platform::single_instance().is_none() {
        return;
    }
    platform::run();
}
