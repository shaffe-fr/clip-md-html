use crate::clipboard::{convert_clipboard, convert_clipboard_to_markdown, copy_selection_to_markdown};
use crate::config::{self, log};
use crate::platform::{self, autostart, Notifier};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::sync::{mpsc::Sender, Arc};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub enum Msg {
    /// Id of the hotkey that fired.
    Hotkey(u32),
    Menu(MenuId),
}

pub type Wake = Arc<dyn Fn() + Send + Sync>;

pub struct App {
    pub notifier: Notifier,
    _tray: TrayIcon,
    _manager: Option<GlobalHotKeyManager>,
    html_hotkey: Option<u32>,
    markdown_hotkey: Option<u32>,
    convert_id: MenuId,
    to_markdown_id: MenuId,
    quit_id: MenuId,
    autostart_item: CheckMenuItem,
}

impl App {
    /// Must be created on the event loop thread: the tray and the hotkey are bound to it.
    pub fn new(tx: Sender<Msg>, wake: Wake) -> Self {
        let mut notifier = Notifier::new();

        let convert = MenuItem::new("Replace copied Markdown with rich text", true, None);
        let to_markdown = MenuItem::new("Replace copied rich text with Markdown", true, None);
        let autostart_item = CheckMenuItem::new(autostart_label(), true, autostart::is_enabled(), None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        let _ = menu.append_items(&[&convert, &to_markdown, &autostart_item, &PredefinedMenuItem::separator(), &quit]);

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
                let _ = tx.send(Msg::Hotkey(e.id));
                wake();
            }
        }));

        let hotkeys = register_hotkeys(&mut notifier);
        check_paste_permission(&mut notifier);

        Self {
            notifier,
            _tray: tray,
            _manager: hotkeys.manager,
            html_hotkey: hotkeys.html,
            markdown_hotkey: hotkeys.markdown,
            convert_id: convert.id().clone(),
            to_markdown_id: to_markdown.id().clone(),
            quit_id: quit.id().clone(),
            autostart_item,
        }
    }

    /// Returns `true` to quit.
    pub fn handle(&mut self, msg: Msg) -> bool {
        match msg {
            Msg::Hotkey(id) if Some(id) == self.html_hotkey => self.convert(true),
            Msg::Hotkey(id) if Some(id) == self.markdown_hotkey => self.convert_to_markdown(true),
            Msg::Hotkey(_) => {}
            Msg::Menu(id) if id == self.convert_id => self.convert(false),
            Msg::Menu(id) if id == self.to_markdown_id => self.convert_to_markdown(false),
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

    /// With `copy_selection`, copies what is selected in the focused window first; otherwise
    /// converts the HTML already on the clipboard. There is nothing to see afterwards, hence the notification.
    fn convert_to_markdown(&mut self, copy_selection: bool) {
        let result = if copy_selection { copy_selection_to_markdown() } else { convert_clipboard_to_markdown() };
        match result {
            Ok(()) => {
                log("converted to Markdown");
                self.notifier.show("clip-md-html", "Copied as Markdown.");
            }
            Err(e) => {
                log(&format!("could not convert to Markdown: {e}"));
                self.notifier.show("clip-md-html", &format!("Could not convert to Markdown: {e}."));
            }
        }
    }
}

fn autostart_label() -> &'static str {
    if cfg!(windows) { "Start with Windows" } else { "Start at login" }
}

struct Hotkeys {
    manager: Option<GlobalHotKeyManager>,
    html: Option<u32>,
    markdown: Option<u32>,
}

fn register_hotkeys(notifier: &mut Notifier) -> Hotkeys {
    let (cfg, cfg_err) = config::load();
    if let Some(e) = cfg_err {
        let msg = format!(
            "{e}. Using default hotkeys ({}, {}). Config: {}",
            config::DEFAULT_HOTKEY,
            config::DEFAULT_MARKDOWN_HOTKEY,
            config::config_path().display()
        );
        log(&msg);
        notifier.show("clip-md-html: configuration", &msg);
    }
    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            let msg = format!("Hotkeys are unavailable ({e}). Change them in {}", config::config_path().display());
            log(&msg);
            notifier.show("clip-md-html: hotkey unavailable", &msg);
            return Hotkeys { manager: None, html: None, markdown: None };
        }
    };
    let html = register_hotkey(&manager, notifier, &cfg.hotkey, config::DEFAULT_HOTKEY);
    let markdown = register_hotkey(&manager, notifier, &cfg.markdown_hotkey, config::DEFAULT_MARKDOWN_HOTKEY);
    Hotkeys { manager: Some(manager), html, markdown }
}

/// Registers `spec`, or `default` when `spec` is invalid. Returns the id of the registered hotkey.
fn register_hotkey(manager: &GlobalHotKeyManager, notifier: &mut Notifier, spec: &str, default: &str) -> Option<u32> {
    let hotkey = match config::parse_hotkey(spec) {
        Ok(h) => h,
        Err(e) => {
            let msg = format!("{e}. Using default hotkey ({default}). Config: {}", config::config_path().display());
            log(&msg);
            notifier.show("clip-md-html: configuration", &msg);
            config::parse_hotkey(default).expect("default hotkey is valid")
        }
    };
    match manager.register(hotkey) {
        Ok(()) => {
            log(&format!("hotkey registered: {hotkey} (config: {spec})"));
            Some(hotkey.id())
        }
        Err(e) => {
            let msg = format!(
                "Hotkey {spec} is unavailable ({e}). It may already be in use; change it in {}",
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
