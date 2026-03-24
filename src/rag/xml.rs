//! Shared XML escaping for RAG output (handler symbol/context XML, store source_file blocks).
//! Callers: handler (symbol_xml, context blocks), store (source_file blocks). Use **escape_attr** for attribute values (e.g. `<tag name="...">`); use **escape_text** for element body content (e.g. `<tag>...</tag>`).
//! Apostrophe (') in attribute values is left unescaped by design; we use double-quote-delimited attributes only.

/// Applies a sequence of (char, replacement) to the string in order.
/// Used by `escape_attr` and `escape_text` to build XML-safe output.
fn apply_replacements(s: &str, replacements: &[(char, &'static str)]) -> String {
    let mut out = s.to_string();
    for (c, r) in replacements {
        out = out.replace(*c, r);
    }
    out
}

/// Escape for XML attribute values: & " < > → &amp; &quot; &lt; &gt; (safe for <symbol name="...">). Uses double-quote only; apostrophe is not escaped.
pub fn escape_attr(s: &str) -> String {
    apply_replacements(
        s,
        &[
            ('&', "&amp;"),
            ('"', "&quot;"),
            ('<', "&lt;"),
            ('>', "&gt;"),
        ],
    )
}

/// Escape for XML text content: & < > → &amp; &lt; &gt; (safe inside element body). Empty input returns empty string.
pub fn escape_text(s: &str) -> String {
    apply_replacements(s, &[('&', "&amp;"), ('<', "&lt;"), ('>', "&gt;")])
}

/// Unit tests for escape_attr and escape_text.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// escape_attr escapes & " < >.
    fn escape_attr_escapes_amp_quote_lt_gt() {
        assert_eq!(escape_attr("a&b"), "a&amp;b");
        assert_eq!(escape_attr(r#"a"b"#), "a&quot;b");
        assert_eq!(escape_attr("a<b>"), "a&lt;b&gt;");
        assert_eq!(escape_attr("&\"<>"), "&amp;&quot;&lt;&gt;");
    }

    #[test]
    /// Safe chars and apostrophe unchanged.
    fn escape_attr_preserves_safe_chars() {
        assert_eq!(escape_attr("hello"), "hello");
        assert_eq!(escape_attr("a'b"), "a'b");
    }

    #[test]
    /// escape_text escapes & < >.
    fn escape_text_escapes_amp_lt_gt() {
        assert_eq!(escape_text("a&b"), "a&amp;b");
        assert_eq!(escape_text("a<b>"), "a&lt;b&gt;");
        assert_eq!(escape_text("&<>"), "&amp;&lt;&gt;");
    }

    #[test]
    /// Double quote not escaped in text.
    fn escape_text_does_not_escape_double_quote() {
        assert_eq!(escape_text(r#"a"b"#), r#"a"b"#);
    }

    #[test]
    /// Empty input returns empty string.
    fn escape_attr_empty_string_returns_empty() {
        assert_eq!(escape_attr(""), "");
    }

    #[test]
    /// escape_text with empty input returns empty string.
    fn escape_text_empty_string_returns_empty() {
        assert_eq!(escape_text(""), "");
    }

    #[test]
    /// escape_attr with Unicode and XML chars escapes only XML chars.
    fn escape_attr_unicode_escapes_xml_chars_only() {
        assert_eq!(escape_attr("café < 1"), "café &lt; 1");
        assert_eq!(escape_attr("α & β"), "α &amp; β");
    }

    #[test]
    /// escape_attr with newline escapes < and > only; newline preserved.
    fn escape_attr_with_newline() {
        assert_eq!(escape_attr("a\n<b>\n"), "a\n&lt;b&gt;\n");
    }

    #[test]
    /// escape_text with newline preserves newline; escapes < and >.
    fn escape_text_with_newline_preserved() {
        assert_eq!(escape_text("a\n<b>\n"), "a\n&lt;b&gt;\n");
    }
}
