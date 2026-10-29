use crate::clipboard::convert_clipboard;
use crate::config::{self, log};
use crate::platform::{self, autostart, Notifier};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::sync::{mpsc::Sender, Arc};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub enum Msg {
    Hotkey,
    Menu(MenuId),
}

pub type Wake = Arc<dyn Fn() + Send + Sync>;

pub struct App {
    pub notifier: Notifier,
    _tray: TrayIcon,
    _manager: Option<GlobalHotKeyManager>,
    convert_id: MenuId,
    quit_id: MenuId,
    autostart_item: CheckMenuItem,
}

impl App {
    /// Must be created on the event loop thread: the tray and the hotkey are bound to it.
    pub fn new(tx: Sender<Msg>, wake: Wake) -> Self {
        let mut notifier = Notifier::new();

        let convert = MenuItem::new("Convert now", true, None);
        let autostart_item = CheckMenuItem::new(autostart_label(), true, autostart::is_enabled(), None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        let _ = menu.append_items(&[&convert, &autostart_item, &PredefinedMenuItem::separator(), &quit]);

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("clip-md-html")
            .with_icon(crate::icon::tray_icon())
            .build()
            .expect("failed to create tray icon");
        log("tray icon created");

        let (tx_m, wake_m) = (tx.clone(), wake.clone());
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let _ = tx_m.send(Msg::Menu(e.id));
            wake_m();
        }));
        GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
            if e.state == HotKeyState::Pressed {
                let _ = tx.send(Msg::Hotkey);
                wake();
            }
        }));

        let manager = register_hotkey(&mut notifier);
        check_paste_permission(&mut notifier);

        Self {
            notifier,
            _tray: tray,
            _manager: manager,
            convert_id: convert.id().clone(),
            quit_id: quit.id().clone(),
            autostart_item,
        }
    }

    /// Returns `true` to quit.
    pub fn handle(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Hotkey => self.convert(true),
            Msg::Menu(id) if id == self.convert_id => self.convert(false),
            Msg::Menu(id) if id == self.quit_id => return true,
            Msg::Menu(id) if id == *self.autostart_item.id() => {
                let want = self.autostart_item.is_checked();
                if let Err(e) = autostart::set_enabled(want) {
                    self.autostart_item.set_checked(!want);
                    self.notifier.show("clip-md-html", &format!("Could not change start at login: {e}"));
                }
            }
            Msg::Menu(_) => {}
        }
        false
    }

    fn convert(&mut self, paste: bool) {
        match convert_clipboard() {
            Ok(()) => {
                log("conversion done");
                if paste {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if let Err(e) = platform::paste() {
                        log(&format!("paste failed: {e}"));
                        self.notifier.show("clip-md-html", &e);
                    }
                }
            }
            Err(e) => {
                log(&format!("nothing to convert: {e}"));
                self.notifier.show("clip-md-html", &format!("Nothing to convert: {e}."));
            }
        }
    }
}

fn autostart_label() -> &'static str {
    if cfg!(windows) { "Start with Windows" } else { "Start at login" }
}

fn register_hotkey(notifier: &mut Notifier) -> Option<GlobalHotKeyManager> {
    let (cfg, cfg_err) = config::load();
    if let Some(e) = cfg_err {
        let msg = format!("{e}. Using default hotkey ({}). Config: {}", config::DEFAULT_HOTKEY, config::config_path().display());
        log(&msg);
        notifier.show("clip-md-html: configuration", &msg);
    }
    let hotkey = match config::parse_hotkey(&cfg.hotkey) {
        Ok(h) => h,
        Err(e) => {
            let msg = format!("{e}. Using default hotkey ({}). Config: {}", config::DEFAULT_HOTKEY, config::config_path().display());
            log(&msg);
            notifier.show("clip-md-html: configuration", &msg);
            config::parse_hotkey(config::DEFAULT_HOTKEY).expect("default hotkey is valid")
        }
    };
    let result = GlobalHotKeyManager::new().map_err(|e| e.to_string()).and_then(|m| {
        m.register(hotkey).map_err(|e| e.to_string())?;
        Ok(m)
    });
    match result {
        Ok(m) => {
            log(&format!("hotkey registered: {hotkey} (config: {})", cfg.hotkey));
            Some(m)
        }
        Err(e) => {
            let msg = format!(
                "Hotkey {} is unavailable ({e}). It may already be in use; change it in {}",
                cfg.hotkey,
                config::config_path().display()
            );
            log(&msg);
            notifier.show("clip-md-html: hotkey unavailable", &msg);
            None
        }
    }
}

fn check_paste_permission(notifier: &mut Notifier) {
    if let Some(msg) = platform::paste_permission_problem() {
        log(&msg);
        notifier.show("clip-md-html: permission required", &msg);
    }
}
