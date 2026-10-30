//! Where the config and the log live. Environment access is injected (`Get`) so the
//! resolution and the migration can be tested without touching the process environment.
#![cfg_attr(target_os = "linux", allow(dead_code))]
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const APP: &str = "clip-md-html";

pub type Get<'a> = &'a dyn Fn(&str) -> Option<OsString>;

pub fn process_env(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

fn home(get: Get) -> PathBuf {
    let var = |n: &str| get(n).filter(|v| !v.is_empty()).map(PathBuf::from);
    let found = if cfg!(windows) { var("USERPROFILE").or_else(|| var("HOME")) } else { var("HOME") };
    found.unwrap_or_else(std::env::temp_dir)
}

/// An XDG variable counts only when set to an absolute path.
fn absolute_env(get: Get, name: &str) -> Option<PathBuf> {
    get(name).map(PathBuf::from).filter(|p| p.is_absolute())
}

/// `$XDG_CONFIG_HOME/clip-md-html`, else `~/.config/clip-md-html`, on every OS.
pub fn config_dir(get: Get) -> PathBuf {
    absolute_env(get, "XDG_CONFIG_HOME").unwrap_or_else(|| home(get).join(".config")).join(APP)
}

/// Directory used before the XDG-style location; only read to migrate from it.
pub fn legacy_config_dir(get: Get) -> Option<PathBuf> {
    if cfg!(windows) {
        get("APPDATA").map(|b| PathBuf::from(b).join(APP))
    } else if cfg!(target_os = "macos") {
        Some(home(get).join("Library/Application Support").join(APP))
    } else {
        None
    }
}

/// State (log, instance lock): `%LOCALAPPDATA%`, `~/Library/Logs`, or `$XDG_STATE_HOME` (`~/.local/state`).
pub fn log_dir(get: Get) -> PathBuf {
    if cfg!(windows) {
        let base = get("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        base.join(APP)
    } else if cfg!(target_os = "macos") {
        home(get).join("Library/Logs").join(APP)
    } else {
        absolute_env(get, "XDG_STATE_HOME").unwrap_or_else(|| home(get).join(".local/state")).join(APP)
    }
}

pub fn config_path() -> PathBuf {
    config_dir(&process_env).join("config.toml")
}

pub fn legacy_config_path() -> Option<PathBuf> {
    legacy_config_dir(&process_env).map(|d| d.join("config.toml"))
}

pub fn log_path() -> PathBuf {
    log_dir(&process_env).join("clip-md-html.log")
}

#[derive(Debug, PartialEq, Eq)]
pub enum Migration {
    /// Nothing to do.
    None,
    /// The legacy file was copied to the new location.
    Copied,
    /// Both files exist: the new one is used.
    LegacyIgnored,
    Failed(String),
}

/// Copies the legacy config to `new` when only the legacy one exists. Never deletes anything.
pub fn migrate(new: &Path, legacy: &Path) -> Migration {
    if !legacy.is_file() {
        return Migration::None;
    }
    if new.exists() {
        return Migration::LegacyIgnored;
    }
    let copy = || -> std::io::Result<()> {
        if let Some(dir) = new.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(legacy, new).map(|_| ())
    };
    match copy() {
        Ok(()) => Migration::Copied,
        Err(e) => Migration::Failed(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs.iter().map(|(k, v)| (k.to_string(), OsString::from(*v))).collect();
        move |k| map.get(k).cloned()
    }

    fn abs(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn with_home(extra: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let h = abs("home");
        let h = h.to_str().unwrap().to_string();
        let mut pairs = vec![("HOME", h.as_str()), ("USERPROFILE", h.as_str())];
        pairs.extend_from_slice(extra);
        env(&pairs)
    }

    #[test]
    fn config_dir_defaults_to_dot_config() {
        assert_eq!(config_dir(&with_home(&[])), abs("home").join(".config").join(APP));
    }

    #[test]
    fn config_dir_uses_absolute_xdg_config_home() {
        let x = abs("xdg");
        let get = with_home(&[("XDG_CONFIG_HOME", x.to_str().unwrap())]);
        assert_eq!(config_dir(&get), x.join(APP));
    }

    #[test]
    fn config_dir_ignores_relative_or_empty_xdg_config_home() {
        for bad in ["relative/dir", ""] {
            let get = with_home(&[("XDG_CONFIG_HOME", bad)]);
            assert_eq!(config_dir(&get), abs("home").join(".config").join(APP));
        }
    }

    #[test]
    fn log_dir_is_state_not_config() {
        let get = with_home(&[("LOCALAPPDATA", abs("local").to_str().unwrap())]);
        let dir = log_dir(&get);
        assert_ne!(dir, config_dir(&get));
        if cfg!(windows) {
            assert_eq!(dir, abs("local").join(APP));
        } else if cfg!(target_os = "macos") {
            assert_eq!(dir, abs("home").join("Library/Logs").join(APP));
        } else {
            assert_eq!(dir, abs("home").join(".local/state").join(APP));
        }
    }

    #[test]
    fn xdg_state_home_applies_on_linux_only() {
        let x = abs("state");
        let get = with_home(&[("XDG_STATE_HOME", x.to_str().unwrap())]);
        if cfg!(not(any(windows, target_os = "macos"))) {
            assert_eq!(log_dir(&get), x.join(APP));
        } else {
            assert_ne!(log_dir(&get), x.join(APP));
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("clip-md-html-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn migrate_copies_legacy_and_keeps_it() {
        let d = scratch("copy");
        let (legacy, new) = (d.join("old/config.toml"), d.join("new/sub/config.toml"));
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, "hotkey = \"Win+Shift+M\"\n").unwrap();
        assert_eq!(migrate(&new, &legacy), Migration::Copied);
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "hotkey = \"Win+Shift+M\"\n");
        assert!(legacy.exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn migrate_prefers_new_when_both_exist() {
        let d = scratch("both");
        let (legacy, new) = (d.join("old.toml"), d.join("new.toml"));
        std::fs::write(&legacy, "old").unwrap();
        std::fs::write(&new, "new").unwrap();
        assert_eq!(migrate(&new, &legacy), Migration::LegacyIgnored);
        assert_eq!(std::fs::read_to_string(&new).unwrap(), "new");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn migrate_does_nothing_without_legacy() {
        let d = scratch("none");
        let new = d.join("new/config.toml");
        assert_eq!(migrate(&new, &d.join("missing.toml")), Migration::None);
        assert!(!new.exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
