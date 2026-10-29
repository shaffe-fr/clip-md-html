//! OS backends. Windows and macOS expose the resident-app interface:
//! `DEFAULT_HOTKEY`, `config_dir`, `single_instance`, `paste`, `paste_permission_problem`,
//! `autostart`, `Notifier`, `run`. Linux is CLI-only and exposes just `paste`.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
compile_error!("clip-md-html only supports Windows, macOS and Linux");
