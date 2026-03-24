//! Server-side web search: Tavily and Serper. Used by the search_web MCP tool.
//! Requires TAVILY_API_KEY or SERPER_API_KEY to be set.
//! Public: `tavily_search`, `serper_search` (env keys); `search_web` builds the MCP request.

use anyhow::Result;
use serde::Deserialize;
use std::sync::OnceLock;

const SEARCH_TIMEOUT_SECS: u64 = 15;

static SEARCH_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

fn search_client() -> reqwest::blocking::Client {
    SEARCH_CLIENT
        .get_or_init(|| {
            reqwest::blocking::Client::builder()
                .use_rustls_tls()
                .timeout(std::time::Duration::from_secs(SEARCH_TIMEOUT_SECS))
                .build()
                .expect("search http client")
        })
        .clone()
}

/// One search result: URL and optional title.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub url: String,
    pub title: Option<String>,
}

/// Call Tavily Search API. Returns up to `limit` results. Requires TAVILY_API_KEY.
pub fn tavily_search(topic: &str, limit: u32) -> Result<Vec<SearchResult>> {
    let key =
        std::env::var("TAVILY_API_KEY").map_err(|_| anyhow::anyhow!("TAVILY_API_KEY not set"))?;
    let key = key.trim();
    if key.is_empty() {
        return Err(anyhow::anyhow!("TAVILY_API_KEY is empty"));
    }
    let limit = limit.clamp(1, 20);
    let body = serde_json::json!({
        "api_key": key,
        "query": topic,
        "max_results": limit,
    });
    let client = search_client();
    let res = client
        .post("https://api.tavily.com/search")
        .json(&body)
        .send()?;
    let status = res.status();
    let text = res.text()?;
    if !status.is_success() {
        return Err(anyhow::anyhow!("Tavily API error ({}): {}", status, text));
    }
    let parsed: TavilyResponse = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Tavily response parse error: {}", e))?;
    Ok(parsed
        .results
        .unwrap_or_default()
        .into_iter()
        .map(|r| SearchResult {
            url: r.url,
            title: Some(r.title).filter(|s| !s.is_empty()),
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct TavilyResponse {
    results: Option<Vec<TavilyResult>>,
}

#[derive(Debug, Deserialize)]
struct TavilyResult {
    url: String,
    title: String,
}

/// Call Serper (Google Search) API. Returns up to `limit` results. Requires SERPER_API_KEY.
pub fn serper_search(topic: &str, limit: u32) -> Result<Vec<SearchResult>> {
    let key =
        std::env::var("SERPER_API_KEY").map_err(|_| anyhow::anyhow!("SERPER_API_KEY not set"))?;
    let key = key.trim();
    if key.is_empty() {
        return Err(anyhow::anyhow!("SERPER_API_KEY is empty"));
    }
    let num = limit.clamp(1, 10) as i32;
    let body = serde_json::json!({
        "q": topic,
        "num": num,
    });
    let client = search_client();
    let res = client
        .post("https://google.serper.dev/search")
        .header("X-API-KEY", key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()?;
    let status = res.status();
    let text = res.text()?;
    if !status.is_success() {
        return Err(anyhow::anyhow!("Serper API error ({}): {}", status, text));
    }
    let parsed: SerperResponse = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Serper response parse error: {}", e))?;
    Ok(parsed
        .organic
        .unwrap_or_default()
        .into_iter()
        .map(|r| SearchResult {
            url: r.link,
            title: Some(r.title).filter(|s| !s.is_empty()),
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct SerperResponse {
    organic: Option<Vec<SerperOrganic>>,
}

#[derive(Debug, Deserialize)]
struct SerperOrganic {
    link: String,
    title: String,
}

/// Run server-side search: Tavily if TAVILY_API_KEY set, else Serper if SERPER_API_KEY set.
/// Returns Ok(results) or Err with message suitable for user (no key leakage).
pub fn search_web(topic: &str, limit: u32) -> Result<Vec<SearchResult>, String> {
    let topic = topic.trim();
    if topic.is_empty() {
        return Err("topic is required and must be non-empty.".to_string());
    }
    if std::env::var("TAVILY_API_KEY")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return tavily_search(topic, limit).map_err(|e| {
            if e.to_string().contains("not set") || e.to_string().contains("is empty") {
                "TAVILY_API_KEY is not set or empty.".to_string()
            } else {
                format!("Search failed: {}", e)
            }
        });
    }
    if std::env::var("SERPER_API_KEY")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return serper_search(topic, limit).map_err(|e| {
            if e.to_string().contains("not set") || e.to_string().contains("is empty") {
                "SERPER_API_KEY is not set or empty.".to_string()
            } else {
                format!("Search failed: {}", e)
            }
        });
    }
    Err("Set TAVILY_API_KEY or SERPER_API_KEY to use server-side search.".to_string())
}
