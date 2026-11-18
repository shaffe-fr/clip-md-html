use htmd::element_handler::{Context, HandlerResult, Handlers};
use htmd::node::Node;
use htmd::options::{BulletListMarker, LinkStyle, Options};
use htmd::{Element, HtmlToMarkdown};
use pulldown_cmark::{html, Options as MdOptions, Parser};
use std::path::PathBuf;
use std::sync::Arc;

/// Stores an embedded image (`data:` or `file:` source) and returns where it now lives.
pub type SaveImage = Arc<dyn Fn(&str) -> Option<PathBuf> + Send + Sync>;

/// Semantic HTML with no styling, so the target editor applies its own defaults.
pub fn markdown_to_html(markdown: &str) -> String {
    let options = MdOptions::ENABLE_TABLES | MdOptions::ENABLE_STRIKETHROUGH | MdOptions::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(markdown, options);
    let mut out = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut out, parser);
    out
}

/// Markdown from the HTML flavor of a copy made in a mail client, a browser or Word.
///
/// Beyond the standard tags it understands what Outlook and Word emit: list paragraphs
/// (`mso-list`), highlighted runs (kept as `<mark>`), monospace runs (inline code), and images
/// embedded in the mail, handed to `save_image` and linked by absolute path.
pub fn html_to_markdown(html: &str, save_image: SaveImage) -> Result<String, String> {
    // Non-breaking spaces are layout in mail HTML and noise in Markdown.
    let html = html.replace("&nbsp;", " ").replace('\u{a0}', " ");
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            bullet_list_marker: BulletListMarker::Dash,
            ul_bullet_spacing: 1,
            ol_number_spacing: 1,
            link_style: LinkStyle::InlinedPreferAutolinks,
            ..Options::default()
        })
        .skip_tags(vec!["style", "script", "title", "xml", "noscript"])
        .add_handler(vec!["img"], move |handlers: &dyn Handlers, element: &Element| image(handlers, element, &save_image))
        .add_handler(vec!["span", "font", "mark"], inline_run)
        .add_handler(vec!["p"], word_list_item)
        .build();
    let markdown = converter.convert(html.as_str()).map_err(|e| format!("could not convert the HTML: {e}"))?;
    Ok(tidy(&markdown))
}

fn tidy(markdown: &str) -> String {
    // Word splits one highlight into several runs; join them back into one.
    let markdown = markdown.replace("</mark><mark>", "").replace("</mark> <mark>", " ");
    markdown.lines().map(unescape_bold_numbering).collect::<Vec<_>>().join("\n")
}

/// htmd escapes a leading `1.` as if it could open a list; behind `**` it cannot, so a bold
/// numbered title (common in mails) reads better without the backslash.
fn unescape_bold_numbering(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("**") {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && rest[digits..].starts_with("\\. ") {
            return format!("**{}. {}", &rest[..digits], &rest[digits + 3..]);
        }
    }
    line.to_string()
}

/// Value of a CSS property in a lowercased inline `style`.
fn style_value<'a>(style: &'a str, property: &str) -> Option<&'a str> {
    style.split(';').filter_map(|d| d.split_once(':')).find(|(k, _)| k.trim() == property).map(|(_, v)| v.trim())
}

fn style_of(node: &Node<'_>) -> String {
    node.attr("style").map(|s| s.to_ascii_lowercase()).unwrap_or_default()
}

/// Word's bullet or number, written as text in a span the browser is meant to hide.
fn is_list_marker(node: &Node<'_>) -> bool {
    node.tag_name() == Some("span") && style_value(&style_of(node), "mso-list") == Some("ignore")
}

fn is_highlighted(node: &Node<'_>, style: &str) -> bool {
    if node.tag_name() == Some("mark") {
        return true;
    }
    if let Some(v) = style_value(style, "mso-highlight") {
        return !matches!(v, "" | "auto" | "none");
    }
    ["background", "background-color"].iter().filter_map(|p| style_value(style, p)).any(|v| {
        let v: String = v.trim_end_matches("!important").chars().filter(|c| !c.is_whitespace()).collect();
        let unpainted = ["", "transparent", "none", "inherit", "initial", "window", "white", "#fff", "#ffffff", "rgb(255,255,255)"];
        !(unpainted.contains(&v.as_str()) || v.starts_with("url(") || v.starts_with("rgba(") && v.ends_with(",0)"))
    })
}

fn is_monospace(node: &Node<'_>, style: &str) -> bool {
    const FAMILIES: [&str; 7] = ["mono", "courier", "consolas", "lucida console", "menlo", "monaco", "cascadia"];
    let family = style_value(style, "font-family").map(str::to_string).or_else(|| node.attr("face").map(|f| f.to_ascii_lowercase()));
    family.is_some_and(|f| FAMILIES.iter().any(|name| f.contains(name)))
}

/// Text of a run made only of inline formatting; `None` as soon as it holds a line break or a block.
fn inline_text(node: &Node<'_>, out: &mut String) -> Option<()> {
    const INLINE: [&str; 11] = ["span", "font", "b", "strong", "i", "em", "u", "mark", "sup", "sub", "o:p"];
    if let Some(text) = node.text() {
        out.push_str(&text);
        return Some(());
    }
    if node.tag_name().is_some_and(|t| !INLINE.contains(&t)) {
        return None;
    }
    if is_list_marker(node) {
        return Some(());
    }
    for child in node.children().iter() {
        inline_text(&child, out)?;
    }
    Some(())
}

fn code_span(text: &str) -> String {
    let longest_run = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest_run + 1);
    let pad = if text.starts_with('`') || text.ends_with('`') { " " } else { "" };
    format!("{fence}{pad}{text}{pad}{fence}")
}

/// Runs `wrap` on the text without its surrounding whitespace, which stays outside the markup.
fn wrap_trimmed(text: &str, wrap: impl FnOnce(&str) -> String) -> String {
    let core = text.trim();
    if core.is_empty() {
        return text.to_string();
    }
    let lead = &text[..text.len() - text.trim_start().len()];
    let trail = &text[text.trim_end().len()..];
    format!("{lead}{}{trail}", wrap(core))
}

/// `span`, `font` and `mark`: Word's hidden list markers, monospace runs and highlights.
fn inline_run(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    let node = element.node;
    if is_list_marker(node) {
        return Some(String::new().into());
    }
    let style = style_of(node);
    let mut content = None;
    if !element.context.literal && is_monospace(node, &style) {
        let mut text = String::new();
        if inline_text(node, &mut text).is_some() {
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            if !text.is_empty() {
                content = Some(code_span(&text));
            }
        }
    }
    let content = match content {
        Some(c) => c,
        None => handlers.fallback(element)?.content,
    };
    if element.context.literal || !is_highlighted(node, &style) {
        return Some(content.into());
    }
    Some(wrap_trimmed(&content, |core| format!("<mark>{core}</mark>")).into())
}

/// Web images stay links. Embedded ones (`data:`, Outlook's `file:` temp copies) are saved and
/// linked by absolute path; references to mail parts (`cid:`, `imap:`, ...) cannot be resolved.
fn image(handlers: &dyn Handlers, element: &Element, save_image: &SaveImage) -> Option<HandlerResult> {
    let src = element.node.attr("src").map(|s| s.trim().to_string()).unwrap_or_default();
    let lower = src.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return handlers.fallback(element);
    }
    let Some(path) = save_image(&src) else {
        return Some("[image]".into());
    };
    let alt = element.node.attr("alt").map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).unwrap_or_else(|| "image".into());
    let alt = alt.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]");
    let dest = path.to_string_lossy().replace('\\', "/");
    let dest = if dest.contains([' ', '(', ')', '<', '>']) { format!("<{}>", dest.replace('<', "%3C").replace('>', "%3E")) } else { dest };
    Some(format!("![{alt}]({dest})").into())
}

/// Marker text and nesting level of a Word list paragraph, which is a `<p>` and not a `<li>`.
fn word_list_marker(node: &Node<'_>) -> Option<String> {
    if is_list_marker(node) {
        let mut text = String::new();
        for child in node.children().iter() {
            inline_text(&child, &mut text);
        }
        return Some(text.trim().to_string());
    }
    node.children().iter().find_map(|child| word_list_marker(&child))
}

/// `p`: Word list paragraphs, and no output at all for the empty spacer paragraphs mail clients add.
fn word_list_item(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    let style = style_of(element.node);
    let list = style_value(&style, "mso-list").filter(|v| !v.starts_with("none"));
    let marker = list.and_then(|_| word_list_marker(element.node));
    let (Some(list), Some(marker)) = (list, marker) else {
        let paragraph = handlers.fallback(element)?;
        return Some(if paragraph.content.trim().is_empty() { String::new().into() } else { paragraph });
    };
    let level = list
        .split_whitespace()
        .find_map(|t| t.strip_prefix("level"))
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let ordered = marker.ends_with(['.', ')']) && marker.chars().any(char::is_alphanumeric);
    let marker = if ordered { marker.as_str() } else { "-" };
    let content = handlers.walk_children_content(element.node, Context::INLINE);
    let indent = " ".repeat(4 * (level - 1));
    // Only a trailing newline: the walk adds the newlines on both sides of a boundary, so
    // consecutive items stay in one tight list while the paragraphs around keep their blank line.
    Some(format!("{indent}{marker} {}\n", content.trim()).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_common_blocks() {
        let html = markdown_to_html("# T\n\n**b** ~~s~~\n\n- a\n- [x] b\n\n|a|b|\n|-|-|\n|1|2|\n");
        assert!(html.contains("<h1>T</h1>"));
        assert!(html.contains("<strong>b</strong>"));
        assert!(html.contains("<del>s</del>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("checkbox"));
        assert!(!html.contains("style="));
    }

    fn md(html: &str) -> String {
        html_to_markdown(html, Arc::new(|_: &str| None)).unwrap()
    }

    #[test]
    fn html_to_markdown_common_blocks() {
        let out = md("<h1>T</h1><p><b>b</b> <i>i</i> <a href=\"https://example.com/x\">link</a></p><ul><li>a</li><li>b</li></ul>");
        assert_eq!(out, "# T\n\n**b** *i* [link](https://example.com/x)\n\n- a\n- b");
    }

    #[test]
    fn html_to_markdown_table() {
        let out = md("<table><tr><th>a</th><th>b</th></tr><tr><td>1</td><td>2</td></tr></table>");
        assert!(out.contains("| a | b |") && out.contains("| 1 | 2 |"), "{out}");
    }

    #[test]
    fn outlook_header_and_paragraphs() {
        let out = md("<p class=MsoNormal>Bonjour<o:p></o:p></p><p class=MsoNormal><o:p>&nbsp;</o:p></p>\
                      <p class=MsoNormal>Cordialement<o:p></o:p></p>\
                      <div style='border:none;border-top:solid #E1E1E1 1.0pt;padding:3.0pt 0cm 0cm 0cm'>\
                      <p class=MsoNormal><b>De&nbsp;:</b> Jean<br><b>Objet&nbsp;:</b> Relance<o:p></o:p></p></div>");
        assert_eq!(out, "Bonjour\n\nCordialement\n\n**De :** Jean  \n**Objet :** Relance");
    }

    #[test]
    fn outlook_message_with_highlights_code_and_image() {
        let html = "<!--StartFragment--><p class=MsoNormal>Frédéric,<o:p></o:p></p><p class=MsoNormal><o:p>&nbsp;</o:p></p>\
        <p class=MsoNormal>Pour rappel, voici les points en attente.<o:p></o:p></p>\
        <p class=MsoNormal><b>1. Lecture de <span style='font-family:\"Courier New\"'>CDE.TXT</span></b><o:p></o:p></p>\
        <p class=MsoNormal>NEO lit-il <span style='font-family:\"Courier New\"'>CDE.TXT</span> en continu (toutes les 15 minutes)&nbsp;?<o:p></o:p></p>\
        <p class=MsoNormal><span style='background:yellow;mso-highlight:yellow'>L/V de 08h à 18h</span><o:p></o:p></p>\
        <p class=MsoNormal><span style='background:yellow;mso-highlight:yellow'>C’est faux.&nbsp; La commande a était importé.</span></p>\
        <p class=MsoNormal><img width=380 height=170 src=\"file:///C:/Users/x/AppData/Local/Temp/msohtmlclip1/01/clip_image001.png\" alt=\"Une image contenant texte\"><o:p></o:p></p><!--EndFragment-->";
        assert_eq!(
            md(html),
            "Frédéric,\n\nPour rappel, voici les points en attente.\n\n**1. Lecture de `CDE.TXT`**\n\n\
             NEO lit-il `CDE.TXT` en continu (toutes les 15 minutes) ?\n\n<mark>L/V de 08h à 18h</mark>\n\n\
             <mark>C’est faux. La commande a était importé.</mark>\n\n[image]"
        );
    }

    #[test]
    fn keeps_highlight_as_mark_and_merges_runs() {
        let out = md("<p><span style='background:yellow;mso-highlight:yellow'>C’est</span>\
                      <span style='background:yellow;mso-highlight:yellow'> faux.</span> Suite</p>");
        assert_eq!(out, "<mark>C’est faux.</mark> Suite");
        assert_eq!(md("<p>a <mark> b </mark> c</p>"), "a <mark>b</mark> c");
        assert_eq!(md("<p><span style='background:white'>x</span><span style='mso-highlight:auto'>y</span></p>"), "xy");
    }

    #[test]
    fn monospace_runs_become_inline_code() {
        let out = md("<p><b>1. Lecture de <span style='font-family:\"Courier New\"'>CDE_TXT</span></b></p>");
        assert!(out.contains("`CDE_TXT`"), "{out}");
        // Several lines or blocks in a monospace wrapper stay as they are.
        let out = md("<font face='Courier New'><p>un</p><p>deux</p></font>");
        assert_eq!(out, "un\n\ndeux");
    }

    #[test]
    fn word_list_paragraphs_become_a_list() {
        let bullet = "<span style='font-family:Symbol'><span style='mso-list:Ignore'>·<span style='font:7.0pt \"Times New Roman\"'>&nbsp;&nbsp;&nbsp;</span></span></span>";
        let nested = "<span style='font-family:\"Courier New\"'><span style='mso-list:Ignore'>o<span>&nbsp;&nbsp;</span></span></span>";
        let number = "<span><span style='mso-list:Ignore'>2.<span>&nbsp;&nbsp;</span></span></span>";
        let out = md(&format!(
            "<p>Avant</p>\
             <p class=MsoListParagraph style='text-indent:-18.0pt;mso-list:l0 level1 lfo1'>{bullet}Un<o:p></o:p></p>\
             <p class=MsoListParagraph style='text-indent:-18.0pt;mso-list:l0 level2 lfo1'>{nested}Deux</p>\
             <p class=MsoListParagraph style='mso-list:l1 level1 lfo2'>{number}Trois</p>\
             <p>Après</p>"
        ));
        assert_eq!(out, "Avant\n\n- Un\n    - Deux\n2. Trois\n\nAprès");
    }

    #[test]
    fn unresolvable_images_are_placeholders_and_web_images_kept() {
        assert_eq!(md("<p>a <img src='cid:image001.png@01DC' alt='x'> b</p>"), "a [image] b");
        assert_eq!(md("<img src='file:///C:/Temp/clip_image001.png'>"), "[image]");
        assert_eq!(md("<img src='imap://host/INBOX?part=1.2'>"), "[image]");
        assert_eq!(md("<img src='https://example.com/a.png' alt='A'>"), "![A](https://example.com/a.png)");
    }

    #[test]
    fn saved_images_are_linked_by_absolute_path() {
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        let save: SaveImage = Arc::new(move |src: &str| {
            log.lock().unwrap().push(src.to_string());
            Some(PathBuf::from(if src.starts_with("data:") { r"C:\Users\K\Pictures\clip-md-html\20261118-161016-1.png" } else { "/home/k/My Pictures/x.png" }))
        });
        let out = html_to_markdown("<p>Voir <img src=\"data:image/png;base64,AAAA\"> et <img alt='Capture [1]' src='file:///tmp/a.png'></p>", save).unwrap();
        assert_eq!(out, "Voir ![image](C:/Users/K/Pictures/clip-md-html/20261118-161016-1.png) et ![Capture \\[1\\]](</home/k/My Pictures/x.png>)");
        assert_eq!(*seen.lock().unwrap(), ["data:image/png;base64,AAAA", "file:///tmp/a.png"]);
    }

    #[test]
    fn head_style_and_scripts_are_ignored() {
        assert_eq!(md("<html><head><title>t</title><style>p{color:red}</style></head><body><p>x</p><script>1</script></body></html>"), "x");
    }
}
