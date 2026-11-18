use crate::convert::{html_to_markdown, markdown_to_html};

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

/// Markdown for the HTML held by the clipboard. Platforms report a missing HTML flavor
/// differently, so every read error is treated as "no HTML".
fn markdown_from_html(html: Option<&str>) -> Result<String, String> {
    let html = html.filter(|h| !h.trim().is_empty()).ok_or("the clipboard has no HTML (copy formatted text from a mail or a web page)")?;
    let saver = std::sync::Arc::new(crate::images::ImageSaver::new(crate::paths::images_dir(&crate::paths::process_env)));
    let markdown = html_to_markdown(html, std::sync::Arc::new(move |src: &str| saver.save(src)))?;
    if markdown.trim().is_empty() {
        return Err("the copied HTML has no text".into());
    }
    Ok(markdown)
}

/// Writes the HTML (CF_HTML) while keeping the original Markdown as the plain-text flavor.
#[cfg(not(target_os = "linux"))]
pub fn convert_clipboard() -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    let text = read_markdown(&mut cb)?;
    let html = markdown_to_html(&text);
    cb.set_html(html, Some(text)).map_err(|e| format!("could not write the clipboard: {e}"))
}

/// Replaces the clipboard with the Markdown of its HTML flavor (plain text only).
#[cfg(not(target_os = "linux"))]
pub fn convert_clipboard_to_markdown() -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    let markdown = markdown_from_html(cb.get().html().ok().as_deref())?;
    cb.set_text(markdown).map_err(|e| format!("could not write the clipboard: {e}"))
}

/// Text and HTML flavors, to notice when a simulated copy has changed the clipboard.
#[cfg(not(target_os = "linux"))]
#[derive(PartialEq)]
struct Snapshot {
    text: Option<String>,
    html: Option<String>,
}

#[cfg(not(target_os = "linux"))]
fn snapshot(cb: &mut arboard::Clipboard) -> Snapshot {
    Snapshot { text: cb.get_text().ok(), html: cb.get().html().ok() }
}

/// Copies what is selected in the focused window, then replaces the clipboard with its Markdown.
///
/// If the copy changes nothing (nothing selected, or the same content as before) the HTML
/// already on the clipboard is used, so copying first and then pressing the hotkey works too.
#[cfg(not(target_os = "linux"))]
pub fn copy_selection_to_markdown() -> Result<(), String> {
    use std::time::{Duration, Instant};

    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    let before = snapshot(&mut cb);
    crate::platform::copy()?;
    // The source application writes the clipboard asynchronously: wait for a change that
    // carries HTML, within a second.
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut now = snapshot(&mut cb);
    while (now == before || now.html.is_none()) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
        now = snapshot(&mut cb);
    }
    let markdown = markdown_from_html(now.html.as_deref()).map_err(|e| {
        if now.html.is_none() && now.text != before.text { "the selection has no HTML formatting".to_string() } else { e }
    })?;
    cb.set_text(markdown).map_err(|e| format!("could not write the clipboard: {e}"))
}

/// Argument of the hidden mode in which a detached child owns the clipboard.
#[cfg(target_os = "linux")]
pub const HOLD_ARG: &str = "--hold-clipboard";

/// Same, for a child that serves plain text only (the Markdown made from HTML).
#[cfg(target_os = "linux")]
pub const HOLD_TEXT_ARG: &str = "--hold-text";

/// On Linux the clipboard content lives only as long as its owner. The conversion is therefore
/// handed to a detached copy of this binary (`HOLD_ARG`) that serves it until another client
/// takes the clipboard, so the caller returns immediately.
#[cfg(target_os = "linux")]
pub fn convert_clipboard() -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    let text = read_markdown(&mut cb)?;
    let html = markdown_to_html(&text);
    spawn_holder(&mut cb, HOLD_ARG, &text, |cb| cb.get().html().is_ok_and(|h| h == html))
}

/// Replaces the clipboard with the Markdown of its HTML flavor (plain text only), served by a
/// detached holder like `convert_clipboard`.
#[cfg(target_os = "linux")]
pub fn convert_clipboard_to_markdown() -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    let markdown = markdown_from_html(cb.get().html().ok().as_deref())?;
    spawn_holder(&mut cb, HOLD_TEXT_ARG, &markdown, |cb| cb.get_text().is_ok_and(|t| t == markdown))
}

/// Starts the holder with `payload` on its stdin and returns once `owns_clipboard` sees the
/// result, so a paste sent right after gets the new content.
#[cfg(target_os = "linux")]
fn spawn_holder(
    cb: &mut arboard::Clipboard,
    arg: &str,
    payload: &str,
    owns_clipboard: impl Fn(&mut arboard::Clipboard) -> bool,
) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe().map_err(|e| format!("could not locate the executable: {e}"))?;
    let mut child = Command::new(exe)
        .arg(arg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .process_group(0) // own group: survives the caller's terminal or service teardown
        .spawn()
        .map_err(|e| format!("could not start the clipboard holder: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("could not open the holder's stdin")?;
    stdin.write_all(payload.as_bytes()).map_err(|e| format!("could not feed the clipboard holder: {e}"))?;
    drop(stdin);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return Err(holder_failure(&mut child, "the clipboard holder exited early"));
        }
        if owns_clipboard(cb) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // Still running without owning the clipboard: stop it so it does not linger, then report.
    let _ = child.kill();
    let _ = child.wait();
    Err(holder_failure(&mut child, "the clipboard holder did not take ownership in time"))
}

/// Appends whatever the (finished) holder wrote to stderr to the error message.
#[cfg(target_os = "linux")]
fn holder_failure(child: &mut std::process::Child, msg: &str) -> String {
    use std::io::Read;
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    let stderr = stderr.trim();
    if stderr.is_empty() { msg.to_string() } else { format!("{msg}: {stderr}") }
}

/// Child side of `convert_clipboard`: reads the Markdown from stdin, publishes it and blocks
/// until another client replaces the clipboard.
#[cfg(target_os = "linux")]
pub fn hold_clipboard() -> Result<(), String> {
    use arboard::SetExtLinux;

    let text = read_stdin()?;
    let html = markdown_to_html(&text);
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    cb.set().wait().html(html, Some(text)).map_err(|e| format!("could not write the clipboard: {e}"))
}

/// Child side of `convert_clipboard_to_markdown`: publishes the text from stdin as is.
#[cfg(target_os = "linux")]
pub fn hold_text() -> Result<(), String> {
    use arboard::SetExtLinux;

    let text = read_stdin()?;
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    cb.set().wait().text(text).map_err(|e| format!("could not write the clipboard: {e}"))
}

#[cfg(target_os = "linux")]
fn read_stdin() -> Result<String, String> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).map_err(|e| format!("could not read stdin: {e}"))?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_from_html_reports_missing_html() {
        assert!(markdown_from_html(None).unwrap_err().contains("no HTML"));
        assert!(markdown_from_html(Some("  \n")).unwrap_err().contains("no HTML"));
        assert!(markdown_from_html(Some("<p> </p>")).unwrap_err().contains("no text"));
        assert_eq!(markdown_from_html(Some("<p>a <b>b</b></p>")).unwrap(), "a **b**");
    }
}
