use std::process::Command;

/// Notifications via le centre de notifications (osascript), sans dépendance.
pub struct Notifier;

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

impl Notifier {
    pub fn new() -> Self {
        Notifier
    }

    pub fn show(&mut self, title: &str, msg: &str) {
        let script = format!("display notification \"{}\" with title \"{}\"", escape(msg), escape(title));
        let _ = Command::new("osascript").arg("-e").arg(script).spawn();
    }
}
