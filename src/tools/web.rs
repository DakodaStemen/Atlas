//! Safe web fetch: HTTPS only, SSRF blocklist (no private/reserved IPs or localhost), HTML -> Markdown, sanitized output. No JS execution. chunk_text: split by words with overlap for long content.
//! **Public:** [`fetch_url_as_markdown`]: full page HTML → Markdown. [`fetch_url_as_markdown_clean`]: readability extraction for ingest. [`chunk_text`]: word-based overlapping chunks for RAG.
//! fetch_url_as_markdown: full page HTML → Markdown (raw .md URLs pass through). fetch_url_as_markdown_clean: readability extraction (nav/ad removal) for ingest. 10s timeout (HTTP_TIMEOUT_SECS).

use crate::rag::sanitize_shell_output;
use anyhow::anyhow;
use std::io::Cursor;
use std::sync::OnceLock;

#[cfg(test)]
type TestFetcher = Option<Box<dyn Fn(&str) -> Result<String, anyhow::Error> + Send>>;

#[cfg(test)]
std::thread_local! {
    static TEST_FETCH: std::cell::RefCell<TestFetcher> = std::cell::RefCell::new(None);
}

/// Test-only: set a fetcher that `fetch_url_as_markdown` will use when present (bypasses network and SSRF).
/// Thread-local so parallel tests do not interfere. Call with `None` to clear after the test.
#[cfg(test)]
pub fn set_test_fetcher(f: TestFetcher) {
    TEST_FETCH.with(|cell| *cell.borrow_mut() = f);
}

static HTTP_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
/// Timeout in seconds for HTTP requests (default 10).
const HTTP_TIMEOUT_SECS: u64 = 10;
/// User-Agent string for outbound requests.
const HTTP_USER_AGENT: &str = "AgenticMonolith/1.0 (research)";

/// Rejects URLs whose host is a private/reserved IP or localhost (SSRF prevention).
/// Blocked: 127.0.0.0/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, ::1, fc00::/7 (ULA), and hostname "localhost".
/// `pub(crate)` so handler.rs can validate HUMANIZER_URL with the same blocklist (C-2).
pub(crate) fn reject_private_or_reserved_host(url: &url::Url) -> Result<(), anyhow::Error> {
    let host = match url.host() {
        Some(h) => h,
        None => return Err(anyhow!("URL has no host")),
    };
    match host {
        url::Host::Domain(d) => {
            if d.eq_ignore_ascii_case("localhost") {
                return Err(anyhow!("Private/localhost host is not allowed (SSRF)"));
            }
            Ok(())
        }
        url::Host::Ipv4(ip) => {
            let octets = ip.octets();
            if octets[0] == 0  // 0.0.0.0/8
                || octets[0] == 127  // loopback
                || octets[0] == 10   // private
                || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)  // private
                || (octets[0] == 192 && octets[1] == 168)  // private
                || (octets[0] == 169 && octets[1] == 254)  // link-local / cloud metadata (169.254.169.254)
                || (octets[0] == 100 && octets[1] >= 64 && octets[1] <= 127)
            // carrier-grade NAT RFC 6598
            {
                return Err(anyhow!(
                    "Private or reserved IPv4 address is not allowed (SSRF)"
                ));
            }
            Ok(())
        }
        url::Host::Ipv6(ip) => {
            let segs = ip.segments();
            if ip.is_loopback()
                || (segs[0] & 0xfe00 == 0xfc00)  // ULA fc00::/7
                || (segs[0] & 0xffc0 == 0xfe80)
            // link-local fe80::/10
            {
                return Err(anyhow!(
                    "Private or reserved IPv6 address is not allowed (SSRF)"
                ));
            }
            Ok(())
        }
    }
}

/// Returns a shared blocking HTTP client: rustls, 10s timeout, no cookies/cache.
/// Uses OnceLock singleton for connection pooling and TLS session reuse.
fn blocking_client() -> reqwest::blocking::Client {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::blocking::Client::builder()
                .use_rustls_tls()
                .redirect(reqwest::redirect::Policy::none())
                .user_agent(HTTP_USER_AGENT)
                .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
                .build()
                .expect("reqwest blocking client")
        })
        .clone()
}

/// Validate URL (HTTPS-only, SSRF blocklist) and return (trimmed_url, parsed, client).
fn validated_fetch_setup(
    url: &str,
) -> Result<(&str, url::Url, reqwest::blocking::Client), anyhow::Error> {
    let url = url.trim();
    if !url.starts_with("https://") {
        return Err(anyhow!("Only https:// URLs are allowed"));
    }
    let parsed = url::Url::parse(url).map_err(|e| anyhow!("Invalid URL: {}", e))?;
    reject_private_or_reserved_host(&parsed)?;
    Ok((url, parsed, blocking_client()))
}

/// True if URL is likely raw Markdown (e.g. raw.githubusercontent.com or path ends with .md).
/// For these we skip readability/html2md and pass the body through.
fn is_raw_markdown_url(url: &str) -> bool {
    url.contains("raw.githubusercontent.com") || url.trim().to_lowercase().ends_with(".md")
}

/// Fetches `url` (HTTPS only), converts HTML to Markdown, and sanitizes the result.
/// Raw .md URLs (e.g. raw.githubusercontent.com) are returned as-is (no html2md conversion).
/// Rejects private/reserved hosts (SSRF blocklist).
pub fn fetch_url_as_markdown(url: &str) -> Result<String, anyhow::Error> {
    #[cfg(test)]
    {
        if let Some(r) = TEST_FETCH.with(|cell| cell.borrow().as_ref().map(|f| f(url))) {
            return r;
        }
    }
    let (url, _parsed, client) = validated_fetch_setup(url)?;
    let body = client.get(url).send()?.error_for_status()?.text()?;
    if is_raw_markdown_url(url) {
        return Ok(sanitize_shell_output(&body));
    }
    let markdown = html2md::parse_html(&body);
    Ok(sanitize_shell_output(&markdown))
}

/// Fetches `url` (HTTPS only), extracts main content with readability, converts to Markdown, and sanitizes.
/// Use for ingest-web to reduce nav/footer/ad noise in embeddings.
/// Raw .md URLs bypass readability and are returned as-is (sanitized).
/// Rejects private/reserved hosts (SSRF blocklist).
pub fn fetch_url_as_markdown_clean(url: &str) -> Result<String, anyhow::Error> {
    let (url, parsed, client) = validated_fetch_setup(url)?;
    let body = client.get(url).send()?.error_for_status()?.text()?;
    if is_raw_markdown_url(url) {
        return Ok(sanitize_shell_output(&body));
    }
    let mut cursor = Cursor::new(body.as_bytes());
    let product = readability::extractor::extract(&mut cursor, &parsed)
        .map_err(|e| anyhow!("Readability extract: {}", e))?;
    let markdown = html2md::parse_html(&product.content);
    Ok(sanitize_shell_output(&markdown))
}

/// Split text into overlapping chunks for RAG ingestion (word-based).
/// Each chunk has up to `chunk_size` words; consecutive chunks share `overlap` words.
/// Returns `(chunk_text, chunk_index)` pairs. Empty or whitespace-only input returns empty vec.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<(String, usize)> {
    if chunk_size == 0 {
        return vec![];
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let mut chunk_index = 0;

    while start < words.len() {
        let end = (start + chunk_size).min(words.len());
        let chunk_words = &words[start..end];
        let chunk_text = chunk_words.join(" ");

        if !chunk_text.trim().is_empty() {
            chunks.push((chunk_text, chunk_index));
            chunk_index += 1;
        }

        // Move start forward, but overlap by `overlap` words
        if end >= words.len() {
            break;
        }
        start += chunk_size.saturating_sub(overlap).max(1);
        if start >= words.len() {
            break;
        }
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::{chunk_text, fetch_url_as_markdown, reject_private_or_reserved_host};

    #[test]
    /// ssrf_rejects_private_ipv4.
    fn ssrf_rejects_private_ipv4() {
        let url = url::Url::parse("https://192.168.1.1/path").unwrap();
        assert!(reject_private_or_reserved_host(&url).is_err());
        let url2 = url::Url::parse("https://10.0.0.1/").unwrap();
        assert!(reject_private_or_reserved_host(&url2).is_err());
        let url3 = url::Url::parse("https://127.0.0.1/").unwrap();
        assert!(reject_private_or_reserved_host(&url3).is_err());
    }

    #[test]
    /// ssrf_rejects_localhost_domain.
    fn ssrf_rejects_localhost_domain() {
        let url = url::Url::parse("https://localhost/").unwrap();
        assert!(reject_private_or_reserved_host(&url).is_err());
    }

    #[test]
    /// fetch_rejects_private_url.
    fn fetch_rejects_private_url() {
        let res = fetch_url_as_markdown("https://192.168.1.1/");
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("SSRF") || err_msg.contains("not allowed"));
    }

    #[test]
    /// chunk_text_empty_returns_empty.
    fn chunk_text_empty_returns_empty() {
        assert!(chunk_text("", 100, 10).is_empty());
        assert!(chunk_text("   \n\t  ", 100, 10).is_empty());
    }

    #[test]
    /// chunk_text_single_chunk_when_words_under_size.
    fn chunk_text_single_chunk_when_words_under_size() {
        let text = "one two three";
        let chunks = chunk_text(text, 10, 2);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, "one two three");
    }

    #[test]
    /// chunk_text_overlapping_chunks_share_words.
    fn chunk_text_overlapping_chunks_share_words() {
        let words: String = (0..20)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let chunks = chunk_text(&words, 10, 3);
        assert!(chunks.len() >= 2, "should have at least 2 chunks");
        let first: Vec<&str> = chunks[0].0.split_whitespace().collect();
        let second: Vec<&str> = chunks[1].0.split_whitespace().collect();
        let overlap_count = first.iter().filter(|w| second.contains(w)).count();
        assert!(
            overlap_count >= 1,
            "consecutive chunks should overlap by at least 1 word"
        );
    }
}
