use gtk::glib;

const FENCES: &[(&str, &str)] = &[
    ("rust", "rust|rs"),
    ("python3", "python|py|python3"),
    ("js", "javascript|js|jsx|mjs"),
    ("typescript", "typescript|ts|tsx"),
    ("sh", "sh|bash|shell|zsh|console"),
    ("json", "json"),
    ("c", "c|h"),
    ("cpp", r"cpp|c\+\+|cc|cxx|hpp"),
    ("go", "go|golang"),
    ("java", "java"),
    ("kotlin", "kotlin|kt"),
    ("ruby", "ruby|rb"),
    ("lua", "lua"),
    ("html", "html|htm"),
    ("css", "css"),
    ("xml", "xml|svg"),
    ("yaml", "yaml|yml"),
    ("toml", "toml"),
    ("sql", "sql"),
    ("diff", "diff|patch"),
];

const LANGUAGE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<language id="downmark" name="Markdown (downmark)" version="2.0" hidden="true">
  <styles>
HEADING_STYLES    <style id="heading-marker" name="Heading Marker" map-to="def:comment"/>
    <style id="fence" name="Code Fence" map-to="def:comment"/>
    <style id="code-block" name="Code Block" map-to="def:preformatted-section"/>
    <style id="strikethrough" name="Strikethrough" map-to="def:deletion"/>
    <style id="list-marker" name="List Marker" map-to="def:list-marker"/>
    <style id="task" name="Task Checkbox" map-to="def:list-marker"/>
    <style id="table" name="Table" map-to="def:comment"/>
    <style id="footnote" name="Footnote" map-to="def:link-text"/>
  </styles>
  <definitions>
FENCES    <context id="fence" style-ref="code-block" class="no-spell-check">
      <start>^(`{3,}|~{3,}).*$</start>
      <end>^\%{1@start}[ \t]*$</end>
      <include>
        <context sub-pattern="0" where="start" style-ref="fence"/>
        <context sub-pattern="0" where="end" style-ref="fence"/>
      </include>
    </context>
HEADINGS    <context id="task">
      <match>^[ \t]*([-+*]|\d+[.)])[ \t]+(\[[ xX]\])(?=[ \t])</match>
      <include>
        <context sub-pattern="1" style-ref="list-marker"/>
        <context sub-pattern="2" style-ref="task"/>
      </include>
    </context>
    <context id="table-delimiter" style-ref="table">
      <match>^[ \t]*\|?([ \t]*:?-+:?[ \t]*\|)+([ \t]*:?-+:?[ \t]*)?$</match>
    </context>
    <context id="table-pipe" style-ref="table">
      <match>(?&lt;!\\)\|</match>
    </context>
    <context id="strikethrough" style-ref="strikethrough">
      <match>~~[^~ \t][^~]*?~~</match>
    </context>
    <context id="footnote" style-ref="footnote" class="no-spell-check">
      <match>\[\^[^\] \t]+\]:?</match>
    </context>
    <context id="downmark">
      <include>
CONTEXTS        <context ref="fence"/>
        <context ref="task"/>
        <context ref="table-delimiter"/>
        <context ref="table-pipe"/>
        <context ref="strikethrough"/>
        <context ref="footnote"/>
        <context ref="markdown:markdown"/>
      </include>
    </context>
  </definitions>
</language>
"#;

pub fn language() -> String {
    let mut heading_styles = String::new();
    let mut headings = String::new();
    let mut contexts = String::new();
    for level in 1..=6 {
        heading_styles += &format!("    <style id=\"h{level}\" name=\"Heading {level}\" map-to=\"def:heading\"/>\n");
        headings += &format!(
            "    <context id=\"h{level}\" style-ref=\"h{level}\">\n      <match>^(#{{{level}}})[ \\t]+.*$</match>\n      <include><context sub-pattern=\"1\" style-ref=\"heading-marker\"/></include>\n    </context>\n"
        );
        contexts += &format!("        <context ref=\"h{level}\"/>\n");
    }
    let mut fences = String::new();
    for (id, names) in FENCES {
        fences += &format!(
            "    <context id=\"fence-{id}\" class=\"no-spell-check\">\n      <start>^(`{{3,}}|~{{3,}})[ \\t]*(?i:{names})(?=[ \\t{{]|$).*$</start>\n      <end>^\\%{{1@start}}[ \\t]*$</end>\n      <include>\n        <context sub-pattern=\"0\" where=\"start\" style-ref=\"fence\"/>\n        <context sub-pattern=\"0\" where=\"end\" style-ref=\"fence\"/>\n        <context ref=\"{id}:{id}\"/>\n      </include>\n    </context>\n"
        );
        contexts.insert_str(0, &format!("        <context ref=\"fence-{id}\"/>\n"));
    }
    LANGUAGE
        .replace("HEADING_STYLES", &heading_styles)
        .replace("FENCES", &fences)
        .replace("HEADINGS", &headings)
        .replace("CONTEXTS", &contexts)
}

pub fn scheme(dark: bool) -> String {
    let (id, name, parent, heading, dim, code_bg) = if dark {
        ("downmark-dark", "downmark dark", "Adwaita-dark", "#78aeed", "#77767b", "#2e2e32")
    } else {
        ("downmark", "downmark", "Adwaita", "#1c71d8", "#9a9996", "#f0eff6")
    };
    let mut styles = String::new();
    for (level, scale) in [(1, "1.5"), (2, "1.3"), (3, "1.15"), (4, "1"), (5, "1"), (6, "1")] {
        styles += &format!("  <style name=\"downmark:h{level}\" foreground=\"{heading}\" bold=\"true\" scale=\"{scale}\"/>\n");
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<style-scheme id="{id}" name="{name}" version="1.0" parent-scheme="{parent}">
{styles}  <style name="markdown:header" foreground="{heading}" bold="true"/>
  <style name="downmark:heading-marker" foreground="{dim}"/>
  <style name="downmark:fence" foreground="{dim}"/>
  <style name="downmark:table" foreground="{dim}"/>
  <style name="markdown:code-span" background="{code_bg}"/>
  <style name="markdown:blockquote-marker" foreground="{dim}" bold="true"/>
</style-scheme>
"#
    )
}

pub fn install() -> std::io::Result<()> {
    let dir = glib::user_cache_dir().join("downmark").join("sourceview");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("downmark.lang"), language())?;
    std::fs::write(dir.join("downmark.xml"), scheme(false))?;
    std::fs::write(dir.join("downmark-dark.xml"), scheme(true))?;
    let path = dir.to_string_lossy();
    sourceview5::LanguageManager::default().append_search_path(&path);
    sourceview5::StyleSchemeManager::default().append_search_path(&path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_builds_on_stock_markdown() {
        let lang = language();
        assert!(lang.contains(r#"<language id="downmark""#));
        assert!(lang.contains(r#"<context ref="markdown:markdown"/>"#));
    }

    #[test]
    fn fenced_code_embeds_the_named_language() {
        let lang = language();
        assert!(lang.contains(r#"<context ref="rust:rust"/>"#));
        assert!(lang.contains(r#"<context ref="python3:python3"/>"#));
        assert!(lang.contains("(?i:javascript|js|jsx|mjs)"));
    }

    #[test]
    fn every_heading_level_has_its_own_style() {
        let lang = language();
        for level in 1..=6 {
            assert!(lang.contains(&format!(r#"<style id="h{level}""#)));
            assert!(lang.contains(&format!("^(#{{{level}}})[ \\t]")));
        }
    }

    #[test]
    fn gfm_extensions_are_highlighted() {
        let lang = language();
        for id in ["strikethrough", "task", "table", "footnote"] {
            assert!(lang.contains(&format!(r#"style-ref="{id}""#)), "{id}");
        }
    }

    #[test]
    fn schemes_extend_adwaita_per_variant() {
        assert!(scheme(false).contains(r#"id="downmark" name="downmark" version="1.0" parent-scheme="Adwaita">"#));
        assert!(scheme(true).contains(r#"id="downmark-dark" name="downmark dark" version="1.0" parent-scheme="Adwaita-dark">"#));
        assert!(scheme(true).contains(r#"<style name="downmark:h1""#));
    }
}
