//! Code spécifique à chaque OS. Chaque backend expose la même interface :
//! `DEFAULT_HOTKEY`, `config_dir`, `single_instance`, `paste`, `autostart`, `Notifier`, `run`.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("clip-md-html ne supporte que Windows et macOS");
