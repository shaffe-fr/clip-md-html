use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use crate::platform::DEFAULT_HOTKEY;
use crate::paths::{self, Migration};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub hotkey: String,
}

impl Default for Config {
    fn default() -> Self {
        Self { hotkey: DEFAULT_HOTKEY.to_string() }
    }
}


pub fn config_path() -> PathBuf {
    paths::config_path()
}

fn template() -> String {
    format!(
        "# clip-md-html config: ~/.config/clip-md-html/config.toml (or $XDG_CONFIG_HOME/clip-md-html/config.toml).\n\
# Global hotkey that converts the clipboard Markdown and pastes it.\n\
# Modifiers: Win (Cmd on macOS), Alt (Option on macOS), Ctrl, Shift.\n\
# Examples: \"Win+Alt+V\", \"Cmd+Alt+V\", \"Win+Shift+M\".\n\
# Avoid Ctrl+Alt (AltGr on AZERTY layouts) and Ctrl+Shift+V (Thunderbird's paste as plain text).\n\
hotkey = \"{DEFAULT_HOTKEY}\"\n"
    )
}

pub fn log(msg: &str) {
    use std::io::Write;
    let path = paths::log_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

/// Writes the default template on first run. The error is set only when the file exists but cannot be parsed.
pub fn load() -> (Config, Option<String>) {
    let path = config_path();
    if let Some(legacy) = paths::legacy_config_path() {
        match paths::migrate(&path, &legacy) {
            Migration::Copied => log(&format!("config migrated from {} to {}", legacy.display(), path.display())),
            Migration::LegacyIgnored => log(&format!("legacy config {} is ignored; using {}", legacy.display(), path.display())),
            Migration::Failed(e) => log(&format!("could not migrate config from {}: {e}", legacy.display())),
            Migration::None => {}
        }
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => match toml::from_str(&text) {
            Ok(c) => (c, None),
            Err(e) => (Config::default(), Some(format!("invalid config.toml ({})", e.message()))),
        },
        Err(_) => {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&path, template());
            (Config::default(), None)
        }
    }
}

pub fn parse_hotkey(spec: &str) -> Result<HotKey, String> {
    let normalized = normalize(spec);
    let hk: HotKey = normalized.parse().map_err(|e| format!("invalid hotkey \"{spec}\" ({e})"))?;
    let m = hk.mods;
    if cfg!(windows) && m.contains(Modifiers::CONTROL) && m.contains(Modifiers::ALT) {
        return Err("Ctrl+Alt is not allowed (conflicts with AltGr on AZERTY layouts)".into());
    }
    if m == (Modifiers::CONTROL | Modifiers::SHIFT) && hk.key == Code::KeyV {
        return Err("Ctrl+Shift+V is reserved (Thunderbird's paste as plain text)".into());
    }
    if m.is_empty() {
        return Err("at least one modifier is required".into());
    }
    Ok(hk)
}

/// Accepts friendlier names than global-hotkey: "Win" -> "super", single letter -> "KeyX", digit -> "DigitN".
fn normalize(spec: &str) -> String {
    spec.split('+')
        .map(|t| {
            let t = t.trim();
            match t.to_ascii_lowercase().as_str() {
                "win" | "windows" | "cmd" | "command" | "meta" => "super".to_string(),
                _ if t.len() == 1 && t.chars().all(|c| c.is_ascii_alphabetic()) => {
                    format!("Key{}", t.to_ascii_uppercase())
                }
                _ if t.len() == 1 && t.chars().all(|c| c.is_ascii_digit()) => format!("Digit{t}"),
                _ => t.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        let hk = parse_hotkey(DEFAULT_HOTKEY).unwrap();
        assert_eq!(hk.key, Code::KeyV);
        assert!(hk.mods.contains(Modifiers::META | Modifiers::ALT));
    }

    #[test]
    fn rejects_forbidden() {
        assert!(!cfg!(windows) || parse_hotkey("Ctrl+Alt+V").is_err());
        assert!(parse_hotkey("Ctrl+Shift+V").is_err());
        assert!(parse_hotkey("V").is_err());
    }
}
