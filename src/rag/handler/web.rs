//! Web fetch, search, and ingest tools: fetch_web_markdown, search_web, ingest_web_context, research_and_verify.

use super::{
    truncate_for_budget, AgenticHandler, IngestionProvider, VectorStoreProvider, MAX_EXTRA,
    VERIFICATION_AGENT_MAX_CHARS, WEB_FALLBACK_MAX_URLS,
};
use crate::rag::db::{ChunkRow, RagDb};
use crate::rag::embedding::RagEmbedder;
use crate::rag::store::format_sandbox_response;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

// ---------- Params ----------

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// FetchWebMarkdownParams.
pub struct FetchWebMarkdownParams {
    /// URL to fetch. Must be https:// only; no JS execution.
    pub url: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// SearchWebParams.
pub struct SearchWebParams {
    pub topic: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// IngestWebContextParams.
pub struct IngestWebContextParams {
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub snippets: Vec<WebSnippetItem>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// WebSnippetItem.
pub struct WebSnippetItem {
    pub url: String,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ResearchAndVerifyParams.
pub struct ResearchAndVerifyParams {
    pub topic: String,
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub force_fetch: Option<bool>,
}

// ---------- Helpers ----------

fn read_research_verify_max_age_days() -> u64 {
    std::env::var("RESEARCH_VERIFY_MAX_AGE_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
}

/// Default max characters for fetch_web_markdown response. Set FETCH_WEB_MAX_CHARS=0 to disable.
const DEFAULT_FETCH_WEB_MAX_CHARS: usize = 32_000;

fn read_fetch_web_max_chars() -> usize {
    std::env::var("FETCH_WEB_MAX_CHARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_FETCH_WEB_MAX_CHARS)
}

/// Parse Verification Agent response: returns (should_ingest, reason).
pub fn parse_verification_agent_response(response: &str) -> (bool, String) {
    let r = response.trim();
    let ingest_false = r.to_uppercase().contains("[INGEST=FALSE]");
    let ingest_true = r.to_uppercase().contains("[INGEST=TRUE]");
    let should_ingest = if !ingest_false && !ingest_true {
        true
    } else {
        ingest_true && !ingest_false
    };
    let reason = r
        .lines()
        .find(|l| !l.trim().is_empty() && !l.trim().to_uppercase().starts_with("[INGEST="))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| "No reason given.".to_string());
    (should_ingest, reason)
}

const OFFICIAL_DOC_PATTERNS: &[&str] = &[
    "doc.rust-lang.org",
    "docs.python.org",
    "react.dev",
    "reactjs.org",
    "developer.mozilla.org",
    "docs.microsoft.com",
];

fn url_contains_any(url_lower: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| url_lower.contains(p))
}

/// Classify source quality based on URL domain.
pub fn classify_web_source_type(url: &str) -> &'static str {
    let url_lower = url.to_lowercase();
    if url_contains_any(&url_lower, &["stackoverflow.com", "stackexchange.com"]) {
        return "stackoverflow";
    }
    if url_contains_any(&url_lower, OFFICIAL_DOC_PATTERNS) {
        return "official";
    }
    if url_contains_any(&url_lower, &["github.com", "gitlab.com", "bitbucket.org"]) {
        return "repository";
    }
    if url_contains_any(&url_lower, &["medium.com", "dev.to", "blog.", "/blog/"]) {
        return "blog";
    }
    "external"
}

/// A web item prepared for ingestion into the RAG database.
pub struct WebIngestItem {
    pub url: String,
    pub summary: String,
    pub detail_chunks: Vec<String>,
    pub source_type: &'static str,
}

/// Ingest web items into RAG.
pub fn ingest_web_items_to_rag(db: &RagDb, embedder: &RagEmbedder, items: &[WebIngestItem]) -> u32 {
    let mut count = 0u32;
    let last_updated = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    );
    for item in items {
        if let Err(e) = db.delete_chunks_by_source(&item.url) {
            warn!(
                "ingest_web_items_to_rag: delete_chunks_by_source {}: {}",
                item.url, e
            );
        }
        if let Err(e) = db.delete_summary_by_source(&item.url) {
            warn!(
                "ingest_web_items_to_rag: delete_summary_by_source {}: {}",
                item.url, e
            );
        }
        let summary_emb = embedder.embed(&item.summary).ok();
        if let Err(e) = db.upsert_summary(&item.url, &item.summary, summary_emb.as_deref()) {
            warn!(
                "ingest_web_items_to_rag: upsert_summary {}: {}",
                item.url, e
            );
        }
        for (idx, chunk_text) in item.detail_chunks.iter().enumerate() {
            let chunk_id = format!("{}#detail{}", item.url, idx);
            let chunk_emb = embedder.embed(chunk_text).ok();
            if let Err(e) = db.upsert_chunk(
                &chunk_id,
                chunk_text,
                &item.url,
                &item.summary,
                "[]",
                "[]",
                "text",
                "detail",
                "[]",
                chunk_emb.as_deref(),
                last_updated,
                "detail",
                item.source_type,
            ) {
                warn!("ingest_web_items_to_rag: upsert_chunk {}: {}", chunk_id, e);
            } else {
                count += 1;
            }
        }
    }
    count
}

// ---------- Impls ----------

pub async fn fetch_web_markdown_impl<I, S>(
    _handler: &AgenticHandler<I, S>,
    params: FetchWebMarkdownParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let url = params.url.trim();
    if url.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "fetch_web_markdown requires a non-empty url.",
        )]));
    }
    if !url.starts_with("https://") {
        return Ok(CallToolResult::success(vec![Content::text(
            "Only https:// URLs are allowed.",
        )]));
    }
    let url_owned = url.to_string();
    let result =
        tokio::task::spawn_blocking(move || crate::tools::web::fetch_url_as_markdown(&url_owned))
            .await
            .map_err(|e| McpError::internal_error(format!("spawn_blocking: {}", e), None))?;
    match result {
        Ok(text) => {
            let truncated = truncate_for_budget(&text, read_fetch_web_max_chars());
            Ok(CallToolResult::success(vec![Content::text(truncated)]))
        }
        Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
            "fetch_web_markdown failed: {}",
            e
        ))])),
    }
}

pub async fn search_web_impl<I, S>(
    _handler: &AgenticHandler<I, S>,
    params: SearchWebParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let topic = params.topic.trim().to_string();
    let limit = params.limit.unwrap_or(5).clamp(1, 20);
    let result =
        tokio::task::spawn_blocking(move || crate::tools::search::search_web(&topic, limit))
            .await
            .map_err(|e| McpError::internal_error(format!("search_web spawn: {}", e), None))?;
    match result {
        Ok(results) => {
            let lines: Vec<String> = results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let t = r.title.as_deref().unwrap_or("").trim();
                    if t.is_empty() {
                        format!("{}. {}", i + 1, r.url)
                    } else {
                        format!(
                            "{}. {} - {}",
                            i + 1,
                            r.title.as_deref().unwrap_or(""),
                            r.url
                        )
                    }
                })
                .collect();
            let text = if lines.is_empty() {
                "No results returned.".to_string()
            } else {
                format!(
                    "{}\n\nUse these URLs with research_and_verify(topic, urls) to compare and ingest.",
                    lines.join("\n")
                )
            };
            Ok(CallToolResult::success(vec![Content::text(text)]))
        }
        Err(msg) => Ok(CallToolResult::success(vec![Content::text(msg)])),
    }
}

pub async fn ingest_web_context_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: IngestWebContextParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let snippets: Vec<(String, String)> = params
        .snippets
        .into_iter()
        .filter_map(|s| {
            let url = s.url.trim().to_string();
            let content = s.content.trim().to_string();
            if url.is_empty() || !url.starts_with("https://") || content.len() < 10 {
                return None;
            }
            Some((url, content))
        })
        .take(WEB_FALLBACK_MAX_URLS * 2)
        .collect();
    if snippets.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "ingest_web_context requires at least one snippet with url (https://) and content (min 10 chars).",
        )]));
    }
    let items: Vec<WebIngestItem> = tokio::task::spawn_blocking(|| {
        snippets
            .into_iter()
            .map(|(url, summary)| {
                let source_type = classify_web_source_type(&url);
                let detail_chunks = match crate::tools::web::fetch_url_as_markdown(&url) {
                    Ok(full_md) => crate::tools::web::chunk_text(&full_md, 500, 50)
                        .into_iter()
                        .map(|(text, _)| text)
                        .collect(),
                    Err(_) => vec![],
                };
                WebIngestItem {
                    url,
                    summary,
                    detail_chunks,
                    source_type,
                }
            })
            .collect()
    })
    .await
    .map_err(|e| McpError::internal_error(format!("ingest_web_context spawn: {}", e), None))?;
    let num_urls = items.len();
    let db = Arc::clone(&handler.store.db);
    let embedder = Arc::clone(&handler.store.embedder);
    let count =
        tokio::task::spawn_blocking(move || ingest_web_items_to_rag(&db, &embedder, &items))
            .await
            .map_err(|e| {
                McpError::internal_error(format!("ingest_web_context join: {}", e), None)
            })?;
    Ok(CallToolResult::success(vec![Content::text(format!(
        "Ingested {} chunks from {} URL(s) into RAG.",
        count, num_urls
    ))]))
}

pub async fn research_and_verify_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ResearchAndVerifyParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let topic = params.topic.trim().to_string();
    let urls: Vec<String> = params
        .urls
        .into_iter()
        .filter_map(|u| {
            let s = u.trim().to_string();
            if s.starts_with("https://") && !s.is_empty() {
                Some(s)
            } else {
                None
            }
        })
        .take(WEB_FALLBACK_MAX_URLS * 2)
        .collect();
    if topic.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "research_and_verify requires a non-empty topic.",
        )]));
    }
    if urls.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "research_and_verify requires at least one https:// URL in urls.",
        )]));
    }
    let force_fetch = params.force_fetch;
    let store = Arc::clone(&handler.store);
    let topic_clone = topic.clone();
    let rows: Vec<ChunkRow> = tokio::task::spawn_blocking(move || {
        store
            .hierarchical_search(&topic_clone, store.rerank_candidates, MAX_EXTRA)
            .unwrap_or_default()
    })
    .await
    .map_err(|e| McpError::internal_error(format!("research_and_verify lookup: {}", e), None))?;
    let rag_had_content = !rows.is_empty();
    if rag_had_content && force_fetch != Some(true) {
        let max_last_updated = rows.iter().filter_map(|r| r.last_updated).max();
        if let Some(max_ts) = max_last_updated {
            let max_age_days = read_research_verify_max_age_days();
            let now_secs = chrono::Utc::now().timestamp().max(0) as u64;
            let cutoff = now_secs.saturating_sub(max_age_days * 86400);
            if max_ts >= cutoff {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "RAG already has content for this topic and it was verified within the last {} days; no fetch or ingest performed. Use different keywords or set force_fetch to refresh.",
                    max_age_days
                ))]));
            }
        }
    }
    let _local_rag_text: String = if rows.is_empty() {
        String::new()
    } else {
        let formatted = format_sandbox_response(&rows, &handler.store.allowed_roots);
        truncate_for_budget(&formatted, VERIFICATION_AGENT_MAX_CHARS)
    };
    let items: Vec<WebIngestItem> = tokio::task::spawn_blocking(move || {
        urls.into_iter()
            .filter_map(|url| {
                let md = crate::tools::web::fetch_url_as_markdown(&url).ok()?;
                let summary = md.lines().next().unwrap_or("").trim().to_string();
                let summary = if summary.len() > 500 {
                    format!("{}...", &summary[..497])
                } else {
                    summary
                };
                let detail_chunks: Vec<String> = crate::tools::web::chunk_text(&md, 500, 50)
                    .into_iter()
                    .map(|(text, _)| text)
                    .collect();
                Some(WebIngestItem {
                    url: url.clone(),
                    summary: if summary.is_empty() {
                        url.clone()
                    } else {
                        summary
                    },
                    detail_chunks,
                    source_type: classify_web_source_type(&url),
                })
            })
            .collect()
    })
    .await
    .map_err(|e| McpError::internal_error(format!("research_and_verify fetch: {}", e), None))?;
    let num_urls = items.len();
    if items.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "research_and_verify: could not fetch any URL; no chunks ingested.",
        )]));
    }
    let _new_web_content: String = {
        let mut s = String::new();
        for item in &items {
            s.push_str(&item.summary);
            s.push('\n');
            if let Some(first) = item.detail_chunks.first() {
                s.push_str(first);
                s.push('\n');
            }
        }
        truncate_for_budget(&s, VERIFICATION_AGENT_MAX_CHARS)
    };
    let db = Arc::clone(&handler.store.db);
    let embedder = Arc::clone(&handler.store.embedder);
    let count =
        tokio::task::spawn_blocking(move || ingest_web_items_to_rag(&db, &embedder, &items))
            .await
            .map_err(|e| {
                McpError::internal_error(format!("research_and_verify ingest: {}", e), None)
            })?;
    let status = if rag_had_content {
        "RAG already had content for this topic"
    } else {
        "RAG had no prior content for this topic"
    };
    Ok(CallToolResult::success(vec![Content::text(format!(
        "{}; ingested {} chunks from {} URL(s).",
        status, count, num_urls
    ))]))
}
