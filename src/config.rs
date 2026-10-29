use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use crate::platform::DEFAULT_HOTKEY;
use crate::platform::config_dir;

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
    config_dir().join("config.toml")
}

fn template() -> String {
    format!(
        "# Raccourci global pour convertir et coller le Markdown du presse-papiers.\n\
# Modificateurs : Win (ou Cmd sur macOS), Alt (Option sur macOS), Ctrl, Shift.\n\
# Exemples : \"Win+Alt+V\", \"Cmd+Alt+V\", \"Win+Shift+M\".\n\
# Éviter Ctrl+Alt (AltGr sur AZERTY) et Ctrl+Shift+V (collage sans format dans Thunderbird).\n\
hotkey = \"{DEFAULT_HOTKEY}\"\n"
    )
}

/// Journal minimal (fichier `clip-md-html.log` à côté de la config).
pub fn log(msg: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(config_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(config_dir().join("clip-md-html.log")) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

/// Charge la config, la crée avec les valeurs par défaut au premier lancement.
pub fn load() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).unwrap_or_default(),
        Err(_) => {
            let _ = std::fs::create_dir_all(config_dir());
            let _ = std::fs::write(&path, template());
            Config::default()
        }
    }
}

/// Analyse et valide un raccourci. Retourne un message d'erreur lisible sinon.
pub fn parse_hotkey(spec: &str) -> Result<HotKey, String> {
    let normalized = normalize(spec);
    let hk: HotKey = normalized.parse().map_err(|e| format!("raccourci « {spec} » invalide ({e})"))?;
    let m = hk.mods;
    if cfg!(windows) && m.contains(Modifiers::CONTROL) && m.contains(Modifiers::ALT) {
        return Err("Ctrl+Alt est interdit (conflit AltGr sur AZERTY)".into());
    }
    if m == (Modifiers::CONTROL | Modifiers::SHIFT) && hk.key == Code::KeyV {
        return Err("Ctrl+Maj+V est réservé (collage sans format de Thunderbird)".into());
    }
    if m.is_empty() {
        return Err("au moins un modificateur est requis".into());
    }
    Ok(hk)
}

/// Rend les noms plus tolérants : "Win" -> "super", lettre seule -> "KeyX", chiffre -> "DigitN".
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
