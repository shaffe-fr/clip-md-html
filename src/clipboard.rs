use crate::convert::markdown_to_html;

/// Writes the HTML (CF_HTML) while keeping the original Markdown as the plain-text flavor.
pub fn convert_clipboard() -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    let text = match cb.get_text() {
        Ok(t) => t,
        Err(arboard::Error::ContentNotAvailable) => return Err("the clipboard does not contain text".into()),
        Err(e) => return Err(format!("could not read the clipboard: {e}")),
    };
    if text.trim().is_empty() {
        return Err("the clipboard is empty".into());
    }
    let html = markdown_to_html(&text);
    cb.set_html(html, Some(text)).map_err(|e| format!("could not write the clipboard: {e}"))
}
