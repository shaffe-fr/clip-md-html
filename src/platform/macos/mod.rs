pub mod autostart;
mod notify;
mod paste;

pub use notify::Notifier;
pub use paste::{paste, paste_permission_problem};

use crate::app::{App, Msg, Wake};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
use winit::window::WindowId;

pub const DEFAULT_HOTKEY: &str = "Cmd+Alt+V";


/// Exclusive file lock: `None` if another instance is already running.
pub fn single_instance() -> Option<()> {
    let dir = crate::paths::log_dir(&crate::paths::process_env);
    let _ = std::fs::create_dir_all(&dir);
    let file = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(dir.join("instance.lock")).ok()?;
    file.try_lock().ok()?;
    // Leaked on purpose: closing the file would release the lock.
    std::mem::forget(file);
    Some(())
}

struct Handler {
    app: Option<App>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    wake: Wake,
}

impl ApplicationHandler<()> for Handler {
    // The tray must be created on the main thread once NSApplication is running.
    fn resumed(&mut self, _el: &ActiveEventLoop) {
        if self.app.is_none() {
            self.app = Some(App::new(self.tx.clone(), self.wake.clone()));
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}

    fn user_event(&mut self, el: &ActiveEventLoop, _event: ()) {
        let Some(app) = self.app.as_mut() else { return };
        while let Ok(m) = self.rx.try_recv() {
            if app.handle(m) {
                el.exit();
                return;
            }
        }
    }
}

/// Accessory activation policy: menu bar icon only, no Dock icon.
pub fn run() {
    let mut builder = EventLoop::<()>::with_user_event();
    builder.with_activation_policy(ActivationPolicy::Accessory);
    let event_loop = builder.build().expect("failed to create event loop");
    let proxy = event_loop.create_proxy();
    let (tx, rx) = channel();
    let wake: Wake = Arc::new(move || {
        let _ = proxy.send_event(());
    });
    let mut handler = Handler { app: None, tx, rx, wake };
    let _ = event_loop.run_app(&mut handler);
}
