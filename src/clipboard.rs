use crate::convert::markdown_to_html;

/// Convertit le texte du presse-papiers en HTML (CF_HTML) en gardant le Markdown en texte brut.
pub fn convert_clipboard() -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| format!("presse-papiers inaccessible : {e}"))?;
    let text = match cb.get_text() {
        Ok(t) => t,
        Err(arboard::Error::ContentNotAvailable) => return Err("le presse-papiers ne contient pas de texte".into()),
        Err(e) => return Err(format!("lecture du presse-papiers impossible : {e}")),
    };
    if text.trim().is_empty() {
        return Err("le presse-papiers est vide".into());
    }
    let html = markdown_to_html(&text);
    cb.set_html(html, Some(text)).map_err(|e| format!("écriture du presse-papiers impossible : {e}"))
}
