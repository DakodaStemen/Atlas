//! Optional response formatting for token savings. When RESPONSE_FORMAT=compact,
//! wrap content in a single-line JSON with short keys to reduce tokens.

/// If RESPONSE_FORMAT=compact, return content as single-line JSON {"t": content}; otherwise return as-is.
pub fn apply_response_format(text: String) -> String {
    let format = std::env::var("RESPONSE_FORMAT").unwrap_or_else(|_| "json".to_string());
    if format.eq_ignore_ascii_case("compact") {
        serde_json::json!({ "t": text }).to_string()
    } else {
        text
    }
}
