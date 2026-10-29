use std::path::PathBuf;

const LABEL: &str = "fr.shaffe.clip-md-html";

fn plist_path() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join("Library/LaunchAgents").join(format!("{LABEL}.plist"))
}

pub fn is_enabled() -> bool {
    plist_path().exists()
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The LaunchAgent is not loaded now; it takes effect at the next login.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let path = plist_path();
    if !enabled {
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n<dict>\n\
  <key>Label</key><string>{LABEL}</string>\n\
  <key>ProgramArguments</key><array><string>{}</string></array>\n\
  <key>RunAtLoad</key><true/>\n\
</dict>\n</plist>\n",
        xml_escape(&exe.display().to_string())
    );
    std::fs::create_dir_all(path.parent().ok_or("invalid LaunchAgents path")?).map_err(|e| e.to_string())?;
    std::fs::write(&path, plist).map_err(|e| e.to_string())
}
