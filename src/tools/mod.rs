//! External API tools: web fetch (fetch_url_*), chunk_text for URL content, search (Tavily/Serper), slack (webhook).
//! Re-exports: `search` (tavily_search, serper_search, search_web), `web` (fetch_url_*, chunk_text).

pub mod search;
pub mod slack;
pub mod web;
