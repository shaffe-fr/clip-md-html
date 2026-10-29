use pulldown_cmark::{html, Options, Parser};

/// Semantic HTML with no styling, so the target editor applies its own defaults.
pub fn markdown_to_html(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(markdown, options);
    let mut out = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut out, parser);
    out
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
}
