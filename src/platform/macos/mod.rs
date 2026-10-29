pub mod autostart;
mod notify;
mod paste;

pub use notify::Notifier;
pub use paste::{paste, paste_permission_problem};

use crate::app::{App, Msg, Wake};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
use winit::window::WindowId;

pub const DEFAULT_HOTKEY: &str = "Cmd+Alt+V";

pub fn config_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    home.join("Library/Application Support/clip-md-html")
}

/// Verrou de fichier exclusif ; retourne `None` si une autre instance tourne déjà.
pub fn single_instance() -> Option<()> {
    let _ = std::fs::create_dir_all(config_dir());
    let file = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(config_dir().join("instance.lock")).ok()?;
    file.try_lock().ok()?;
    // Le fichier (donc le verrou) reste ouvert jusqu'à la fin du processus.
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
    // Le tray doit être créé une fois NSApplication démarré, sur le thread principal.
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

/// Boucle d'événements sur le thread principal, sans icône dans le Dock (Accessory).
pub fn run() {
    let mut builder = EventLoop::<()>::with_user_event();
    builder.with_activation_policy(ActivationPolicy::Accessory);
    let event_loop = builder.build().expect("boucle d'événements");
    let proxy = event_loop.create_proxy();
    let (tx, rx) = channel();
    let wake: Wake = Arc::new(move || {
        let _ = proxy.send_event(());
    });
    let mut handler = Handler { app: None, tx, rx, wake };
    let _ = event_loop.run_app(&mut handler);
}
