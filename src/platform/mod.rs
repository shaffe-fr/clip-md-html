//! OS backends. Each one exposes the same interface:
//! `DEFAULT_HOTKEY`, `config_dir`, `single_instance`, `paste`, `paste_permission_problem`,
//! `autostart`, `Notifier`, `run`.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("clip-md-html only supports Windows and macOS");
