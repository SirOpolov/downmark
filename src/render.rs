use pulldown_cmark::{Options, Parser, html};

pub fn to_html(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES;
    let mut out = String::with_capacity(markdown.len() * 3 / 2);
    html::push_html(&mut out, Parser::new_ext(markdown, options));
    out
}

const STYLE: &str = "
:root { color-scheme: light dark; }
body { font: 16px/1.6 sans-serif; max-width: 860px; margin: 0 auto; }
pre, code { font-family: monospace; background: rgba(127,127,127,.15); border-radius: 4px; }
code { padding: .1em .3em; }
pre { padding: 12px; overflow-x: auto; }
pre code { padding: 0; background: none; }
table { border-collapse: collapse; }
th, td { border: 1px solid rgba(127,127,127,.4); padding: 4px 10px; }
blockquote { margin: 0; padding-left: 1em; border-left: 4px solid rgba(127,127,127,.4); opacity: .85; }
img { max-width: 100%; }
.footnote-definition { font-size: .9em; margin-top: 1em; }
.footnote-definition p { display: inline; }
@media print { :root { color-scheme: light; } }
a { color: #3584e4; }
";

pub const TOP_MARGIN: i32 = 8;

pub fn page(body: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><style>{STYLE}\
         body {{ padding: {TOP_MARGIN}px 24px 24px; }}\
         body > :first-child {{ margin-top: 0; }}</style></head><body>{body}</body></html>"
    )
}

pub fn update_script(body: &str) -> String {
    format!("document.body.innerHTML = {};", serde_json::Value::from(body))
}

#[cfg(test)]
mod tests {
    use super::{page, to_html, update_script};

    #[test]
    fn renders_heading() {
        assert_eq!(to_html("# Hi"), "<h1>Hi</h1>\n");
    }

    #[test]
    fn renders_table() {
        let html = to_html("| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>2</td>"));
    }

    #[test]
    fn renders_task_list() {
        let html = to_html("- [x] done\n- [ ] todo");
        assert!(html.contains(r#"<input disabled="" type="checkbox" checked=""/>"#));
        assert!(html.contains(r#"<input disabled="" type="checkbox"/>"#));
    }

    #[test]
    fn renders_fenced_code_with_language() {
        let html = to_html("```rust\nfn main() {}\n```");
        assert!(html.contains(r#"<code class="language-rust">fn main() {}"#));
    }

    #[test]
    fn renders_strikethrough() {
        assert_eq!(to_html("~~old~~"), "<p><del>old</del></p>\n");
    }

    #[test]
    fn update_script_embeds_html_as_json_string() {
        let html = "<p>\"quote\" `tick` </script>\n</p>";
        let script = update_script(html);
        let literal = script
            .strip_prefix("document.body.innerHTML = ")
            .and_then(|s| s.strip_suffix(';'))
            .unwrap();
        assert_eq!(serde_json::from_str::<String>(literal).unwrap(), html);
    }

    #[test]
    fn page_first_line_starts_at_editor_top_margin() {
        let html = page("");
        assert!(html.contains(&format!("padding: {}px 24px 24px", super::TOP_MARGIN)));
        assert!(html.contains("body > :first-child { margin-top: 0; }"));
    }

    #[test]
    fn page_prints_in_light_colors() {
        assert!(page("").contains("@media print { :root { color-scheme: light; } }"));
    }

    #[test]
    fn page_contains_body_html() {
        assert!(page("<h1>Hi</h1>").contains("<body><h1>Hi</h1></body>"));
    }

    #[test]
    fn escapes_html_in_code() {
        assert!(to_html("`<script>`").contains("&lt;script&gt;"));
    }
}
