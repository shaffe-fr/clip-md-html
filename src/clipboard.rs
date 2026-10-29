use crate::convert::markdown_to_html;

fn read_markdown(cb: &mut arboard::Clipboard) -> Result<String, String> {
    let text = match cb.get_text() {
        Ok(t) => t,
        Err(arboard::Error::ContentNotAvailable) => return Err("the clipboard does not contain text".into()),
        Err(e) => return Err(format!("could not read the clipboard: {e}")),
    };
    if text.trim().is_empty() {
        return Err("the clipboard is empty".into());
    }
    Ok(text)
}

/// Writes the HTML (CF_HTML) while keeping the original Markdown as the plain-text flavor.
#[cfg(not(target_os = "linux"))]
pub fn convert_clipboard() -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    let text = read_markdown(&mut cb)?;
    let html = markdown_to_html(&text);
    cb.set_html(html, Some(text)).map_err(|e| format!("could not write the clipboard: {e}"))
}

/// Argument of the hidden mode in which a detached child owns the clipboard.
#[cfg(target_os = "linux")]
pub const HOLD_ARG: &str = "--hold-clipboard";

/// On Linux the clipboard content lives only as long as its owner. The conversion is therefore
/// handed to a detached copy of this binary (`HOLD_ARG`) that serves it until another client
/// takes the clipboard, so the caller returns immediately.
#[cfg(target_os = "linux")]
pub fn convert_clipboard() -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    let text = read_markdown(&mut cb)?;
    let html = markdown_to_html(&text);

    let exe = std::env::current_exe().map_err(|e| format!("could not locate the executable: {e}"))?;
    let mut child = Command::new(exe)
        .arg(HOLD_ARG)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0) // own group: survives the caller's terminal or service teardown
        .spawn()
        .map_err(|e| format!("could not start the clipboard holder: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("could not open the holder's stdin")?;
    stdin.write_all(text.as_bytes()).map_err(|e| format!("could not feed the clipboard holder: {e}"))?;
    drop(stdin);

    // Return only once the child owns the clipboard, so a paste sent right after sees the HTML.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return Err("the clipboard holder exited early".into());
        }
        if cb.get().html().is_ok_and(|h| h == html) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Err("the clipboard holder did not take ownership in time".into())
}

/// Child side of `convert_clipboard`: reads the Markdown from stdin, publishes it and blocks
/// until another client replaces the clipboard.
#[cfg(target_os = "linux")]
pub fn hold_clipboard() -> Result<(), String> {
    use arboard::SetExtLinux;
    use std::io::Read;

    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).map_err(|e| format!("could not read stdin: {e}"))?;
    let html = markdown_to_html(&text);
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    cb.set().wait().html(html, Some(text)).map_err(|e| format!("could not write the clipboard: {e}"))
}
