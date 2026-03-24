//! RAG and code navigation tools: query_knowledge, query_master_research, get_related_code, resolve_symbol, get_doc_outline, get_section, get_relevant_tools.
//! Params and implementation functions; handler mod.rs delegates to *_impl. Symbol tools live in rag_symbols.

mod rag_symbols;
pub use rag_symbols::{
    get_related_code_impl, resolve_symbol_impl, symbol_xml, GetRelatedCodeParams,
    ResolveSymbolParams,
};

use super::{
    format_rag_meta, format_response::apply_response_format, truncate_rag_response, AgenticHandler,
    IngestionProvider, VectorStoreProvider, EMPTY_RAG_CONTEXT, MAX_EXTRA,
};
use crate::rag::chunking::SECTION_CHUNK_TYPE;
use crate::rag::domain_classifier::classify_source;
use crate::rag::store::format_sandbox_response;
use rmcp::model::{CallToolResult, Content, Tool};
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

// ---------- Params (re-exported from handler mod for tool_router) ----------

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// QueryKnowledgeParams.
pub struct QueryKnowledgeParams {
    /// Natural language or keyword query over the codebase.
    pub query: String,
    /// When true, filter to fewer, highly relevant chunks (slower). You generate the final response; use when you need deep context.
    #[serde(default)]
    pub reasoning: bool,
    /// When true, return RAG context for the IDE to synthesize the answer.
    #[serde(default)]
    pub execute: bool,
    /// When true, return only chunk id, source, and name per hit (no full text). Use for exploratory browse; then get_section(section_id) or full query_knowledge for content.
    #[serde(default)]
    pub outline_only: bool,
    /// When true (and not outline_only), return section IDs and instruction to call get_section(id) for content. Token-efficient for document-heavy queries.
    #[serde(default)]
    pub section_first: bool,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// QueryMasterResearchParams. Searches only the synthesized Godly RAG master document.
pub struct QueryMasterResearchParams {
    /// Natural language or keyword query over the master research document.
    pub query: String,
    /// Max number of chunks to return (default 5).
    #[serde(default = "query_master_research_default_top_k")]
    pub top_k: u32,
}

pub fn query_master_research_default_top_k() -> u32 {
    5
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GetDocOutlineParams. Token-efficient: returns section ids and titles only (no full content).
pub struct GetDocOutlineParams {
    /// Source path of the document (e.g. docs/setup/RAG_OPERATIONS.md). Must be under ALLOWED_ROOTS.
    pub source: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GetSectionParams. Fetch full content of one section by chunk id (from get_doc_outline).
pub struct GetSectionParams {
    /// Chunk/section id (e.g. path#0 from get_doc_outline).
    pub section_id: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// Params for get_relevant_tools (Tool-RAG: similarity over tool names/descriptions).
pub struct GetRelevantToolsParams {
    /// Natural-language task or user message; tools are ranked by semantic similarity to this query.
    pub query: String,
    /// Max number of tool names to return (default 15). Use to limit context when building prompts.
    #[serde(default)]
    pub top_k: Option<u32>,
    /// When true, return array of { name, description } per tool (gateway mode). Default false = array of names only.
    #[serde(default)]
    pub include_descriptions: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// Params for invoke_tool (gateway mode: run a tool by name with given arguments).
pub struct InvokeToolParams {
    /// Name of the tool to run (from get_relevant_tools).
    pub name: String,
    /// Arguments object for the tool (must match the tool's schema). Pass as a plain
    /// JSON object — do NOT stringify it. Example: {"query": "foo"} not "{\"query\":\"foo\"}".
    #[serde(default)]
    #[schemars(schema_with = "invoke_tool_arguments_schema")]
    pub arguments: Option<serde_json::Value>,
}

fn invoke_tool_arguments_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": ["object", "null"],
        "description": "Arguments for the tool. Pass as a plain JSON object — do NOT stringify it.",
        "additionalProperties": true
    })
}

// ---------- Implementation functions ----------

/// Core RAG logic. execute=true returns context for the IDE to synthesize. Used by the #[tool] query_knowledge and ServerHandler::call_tool.
pub async fn query_knowledge_core<I, S>(
    handler: &AgenticHandler<I, S>,
    p: QueryKnowledgeParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let query = p.query.trim();
    if query.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            EMPTY_RAG_CONTEXT,
        )]));
    }

    // Semantic cache: optional hit for similar queries (SEMANTIC_CACHE_ENABLED, !outline_only).
    let semantic_cache_enabled = std::env::var("SEMANTIC_CACHE_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if semantic_cache_enabled && !p.outline_only && handler.store.embedder.is_available() {
        let store = handler.store.clone();
        let query_owned = query.to_string();
        let embed_result =
            tokio::task::spawn_blocking(move || store.embedder.embed_query(&query_owned).ok())
                .await
                .ok()
                .and_then(|o| o);
        if let Some(ref query_emb) = embed_result {
            let threshold: f32 = std::env::var("SEMANTIC_CACHE_SIMILARITY_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.90);
            if let Ok(Some(response_text)) = handler.store.db.semantic_cache_knn(query_emb, threshold) {
                crate::metrics::CACHE_HITS.inc();
                let suffix = "\n[semantic_cache_hit]";
                return Ok(CallToolResult::success(vec![Content::text(
                    apply_response_format(format!("{}{}", response_text, suffix)),
                )]));
            }
            // Embedding was computed but no cache hit — record a miss.
            crate::metrics::CACHE_MISSES.inc();
        }
    }

    // section_first: return outline + instruction to call get_section(id) (no full rerank/MMR).
    if p.section_first && !p.outline_only {
        let limit = 10;
        let mut rows = handler
            .store
            .hybrid_search(query, limit, None)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        rows.truncate(limit);
        rows.retain(|r| handler.store.path_under_allowed(&r.source));
        if rows.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                EMPTY_RAG_CONTEXT,
            )]));
        }
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        let section_ids_line = ids.join(", ");
        let outline_lines: Vec<String> = rows
            .iter()
            .map(|r| {
                let title = if r.name.is_empty() {
                    r.summary.lines().next().unwrap_or("").trim()
                } else {
                    &r.name
                };
                format!("{} | {} | {}", r.id, r.source, title)
            })
            .collect();
        let outline_text = outline_lines.join("\n");
        let instruction = format!(
            "\n\nCall get_section(section_id) for these ids: {}",
            section_ids_line
        );
        let tokens_estimated = handler
            .store
            .embedder
            .count_tokens(&outline_text)
            .unwrap_or_else(|| outline_text.chars().count() / 4);
        let meta = format_rag_meta("chunks_returned", rows.len(), tokens_estimated);
        return Ok(CallToolResult::success(vec![Content::text(
            apply_response_format(format!("{}{}{}", outline_text, instruction, meta)),
        )]));
    }

    // outline_only: skip cross-encoder rerank and MMR — use lightweight hybrid search for browse
    if p.outline_only {
        let limit = (handler.store.rerank_top_k * 2).max(10);
        let mut rows = handler
            .store
            .hybrid_search(query, limit, None)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        rows.truncate(handler.store.rerank_top_k);
        rows.retain(|r| handler.store.path_under_allowed(&r.source));
        if rows.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                EMPTY_RAG_CONTEXT,
            )]));
        }
        let outline_lines: Vec<String> = rows
            .iter()
            .map(|r| {
                let title = if r.name.is_empty() {
                    r.summary.lines().next().unwrap_or("").trim()
                } else {
                    &r.name
                };
                format!("{} | {} | {}", r.id, r.source, title)
            })
            .collect();
        let outline_text = outline_lines.join("\n");
        let tokens_estimated = handler
            .store
            .embedder
            .count_tokens(&outline_text)
            .unwrap_or_else(|| outline_text.chars().count() / 4);
        let meta = format_rag_meta("chunks_returned", rows.len(), tokens_estimated);
        return Ok(CallToolResult::success(vec![Content::text(
            apply_response_format(format!("{}{}", outline_text, meta)),
        )]));
    }

    let step_start = Instant::now();
    let mut rows = tracing::info_span!("hierarchical_search", query = %query).in_scope(|| {
        handler
            .store
            .hierarchical_search(query, handler.store.rerank_candidates, MAX_EXTRA)
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    })?;
    tracing::info!(
        target: "mcp_timing",
        request_id = %super::current_request_id(),
        step = "hierarchical_search",
        elapsed_ms = %step_start.elapsed().as_millis(),
        "query_knowledge step"
    );

    let step_start = Instant::now();
    rows = handler
        .store
        .rerank_results(query, rows, handler.store.rerank_top_k);
    tracing::info!(
        target: "mcp_timing",
        request_id = %super::current_request_id(),
        step = "rerank_results",
        elapsed_ms = %step_start.elapsed().as_millis(),
        "query_knowledge step"
    );

    let step_start = Instant::now();
    rows = handler.store.mmr_rerank(rows, handler.store.rerank_top_k);
    tracing::info!(
        target: "mcp_timing",
        request_id = %super::current_request_id(),
        step = "mmr_rerank",
        elapsed_ms = %step_start.elapsed().as_millis(),
        "query_knowledge step"
    );

    if p.reasoning || p.execute {
        rows.truncate(5);
        let step_start = Instant::now();
        rows = handler.store.expand_with_details(rows, 3);
        tracing::info!(
            target: "mcp_timing",
            request_id = %super::current_request_id(),
            step = "expand_with_details",
            elapsed_ms = %step_start.elapsed().as_millis(),
            "query_knowledge step"
        );
    } else {
        rows.truncate(handler.store.rerank_top_k);
    }

    let text = truncate_rag_response(
        &handler.store,
        &format_sandbox_response(&rows, &handler.store.allowed_roots),
    );
    let tokens_estimated = handler
        .store
        .embedder
        .count_tokens(&text)
        .unwrap_or_else(|| text.chars().count() / 4);
    let meta_suffix = if rows.is_empty() {
        String::new()
    } else {
        format_rag_meta("chunks_returned", rows.len(), tokens_estimated)
    };

    if rows.is_empty() && (text == EMPTY_RAG_CONTEXT || text.is_empty()) {
        return Ok(CallToolResult::success(vec![Content::text(
            EMPTY_RAG_CONTEXT,
        )]));
    }

    if p.execute && !text.is_empty() {
        let response_text = format!(
            "Use the context below to answer the user's question.\n\n{}{}",
            text, meta_suffix
        );
        if semantic_cache_enabled && handler.store.embedder.is_available() {
            let store = handler.store.clone();
            let query_owned = query.to_string();
            let response_owned = response_text.clone();
            tokio::task::spawn_blocking(move || {
                if let Ok(emb) = store.embedder.embed_query(&query_owned) {
                    let _ = store
                        .db
                        .semantic_cache_insert(&query_owned, &response_owned, &emb);
                }
            });
        }
        return Ok(CallToolResult::success(vec![Content::text(
            apply_response_format(response_text),
        )]));
    }
    {
        if !p.reasoning && !rows.is_empty() {
            if let Some(ref guard) = handler.dataset_collector {
                let guard = Arc::clone(guard);
                let query = query.to_string();
                let text = text.clone();
                let domain = rows
                    .first()
                    .map(|r| classify_source(&r.source))
                    .unwrap_or("general")
                    .to_string();
                tokio::task::spawn_blocking(move || match guard.lock() {
                    Ok(c) => {
                        if let Err(e) = c.record_interaction(&query, &text, "", &domain) {
                            tracing::warn!("dataset_collector record: {}", e);
                        }
                    }
                    Err(e) => tracing::warn!("dataset_collector lock poisoned: {}", e),
                });
            }
        }
        let output = format!("{}{}", text, meta_suffix);
        if semantic_cache_enabled
            && !output.is_empty()
            && output != EMPTY_RAG_CONTEXT
            && handler.store.embedder.is_available()
        {
            let store = handler.store.clone();
            let query_owned = query.to_string();
            let output_owned = output.clone();
            tokio::task::spawn_blocking(move || {
                if let Ok(emb) = store.embedder.embed_query(&query_owned) {
                    let _ = store
                        .db
                        .semantic_cache_insert(&query_owned, &output_owned, &emb);
                }
            });
        }
        Ok(CallToolResult::success(vec![Content::text(
            apply_response_format(output),
        )]))
    }
}

pub async fn query_master_research_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    p: QueryMasterResearchParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let query = p.query.trim();
    if query.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            EMPTY_RAG_CONTEXT,
        )]));
    }
    let master_source = std::env::var("MASTER_RESEARCH_SOURCE")
        .unwrap_or_else(|_| "research/master_research.md".to_string());
    let top_k = p.top_k.clamp(1, 20) as usize;
    let hybrid_limit = (top_k * 3).min(30);
    let filter = [master_source.clone()];
    let mut rows = handler
        .store
        .hybrid_search(query, hybrid_limit, Some(&filter))
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    rows.retain(|r| r.source == master_source);
    if rows.is_empty() {
        let by_source = handler
            .store
            .get_chunks_by_source(&master_source)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        if by_source.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                EMPTY_RAG_CONTEXT,
            )]));
        }
        rows = by_source;
        rows.truncate(top_k);
    } else {
        rows = handler
            .store
            .rerank_results(query, rows, handler.store.rerank_candidates.min(15));
        rows = handler.store.mmr_rerank(rows, top_k);
    }
    let text = truncate_rag_response(
        &handler.store,
        &format_sandbox_response(&rows, &handler.store.allowed_roots),
    );
    let tokens_estimated = handler
        .store
        .embedder
        .count_tokens(&text)
        .unwrap_or_else(|| text.chars().count() / 4);
    let meta = format_rag_meta("chunks_returned", rows.len(), tokens_estimated);
    Ok(CallToolResult::success(vec![Content::text(
        apply_response_format(format!("{}{}", text, meta)),
    )]))
}

pub async fn get_doc_outline_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    p: GetDocOutlineParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let source = p.source.trim();
    if source.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No source path provided.",
        )]));
    }
    if !handler.store.path_under_allowed(source) {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "Access denied: {} is not under ALLOWED_ROOTS.",
            source
        ))]));
    }
    let rows = handler
        .store
        .db
        .get_chunks_by_source(source)
        .map_err(|e| McpError::internal_error(format!("get_doc_outline db: {}", e), None))?;
    let mut sections: Vec<(String, String)> = rows
        .into_iter()
        .filter(|r| r.type_ == SECTION_CHUNK_TYPE)
        .map(|r| (r.id.clone(), r.name.clone()))
        .collect();
    sections.sort_by(|a, b| a.0.cmp(&b.0));
    let out = if sections.is_empty() {
        format!(
            "No sections found for '{}'. (Document may use generic chunking or not be indexed.)",
            source
        )
    } else {
        sections
            .iter()
            .map(|(id, name)| format!("{} | {}", id, name))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let tokens_estimated = handler
        .store
        .embedder
        .count_tokens(&out)
        .unwrap_or_else(|| out.chars().count() / 4);
    let meta = format_rag_meta("sections_returned", sections.len(), tokens_estimated);
    Ok(CallToolResult::success(vec![Content::text(format!(
        "{}{}",
        out, meta
    ))]))
}

pub async fn get_section_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    p: GetSectionParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let section_id = p.section_id.trim();
    if section_id.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No section_id provided.",
        )]));
    }
    let rows = handler
        .store
        .db
        .get_chunks_by_ids(&[section_id.to_string()])
        .map_err(|e| McpError::internal_error(format!("get_section db: {}", e), None))?;
    let row = match rows.into_iter().next() {
        Some(r) => r,
        None => {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No chunk found for section_id '{}'.",
                section_id
            ))]));
        }
    };
    if !handler.store.path_under_allowed(&row.source) {
        return Ok(CallToolResult::success(vec![Content::text(
            "Access denied: chunk source is not under ALLOWED_ROOTS.",
        )]));
    }
    let text = truncate_rag_response(&handler.store, &row.text);
    let tokens_estimated = handler
        .store
        .embedder
        .count_tokens(&text)
        .unwrap_or_else(|| text.chars().count() / 4);
    let meta = format_rag_meta("chunks_returned", 1, tokens_estimated);
    Ok(CallToolResult::success(vec![Content::text(
        apply_response_format(format!("{}{}", text, meta)),
    )]))
}

pub async fn get_relevant_tools_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    p: GetRelevantToolsParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let query = p.query.trim().to_string();
    let default_top_k = std::env::var("GET_RELEVANT_TOOLS_TOP_K")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let top_k = p.top_k.unwrap_or(default_top_k).clamp(1, 50) as usize;
    let tools: Vec<Tool> = handler.tool_router.list_all();
    if tools.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text("[]")]));
    }
    let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    let include_descriptions = p.include_descriptions.unwrap_or(false);
    if !handler.store.embedder.is_available() {
        if include_descriptions {
            let items: Vec<serde_json::Value> = tools
                .iter()
                .take(top_k)
                .map(|t| {
                    serde_json::json!({
                        "name": t.name.as_ref(),
                        "description": t.description.as_deref().unwrap_or("")
                    })
                })
                .collect();
            let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }
        let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
        return Ok(CallToolResult::success(vec![Content::text(json)]));
    }
    let store = handler.store.clone();
    let query_owned = query.clone();
    let tool_texts: Vec<String> = tools
        .iter()
        .map(|t| {
            let d = t.description.as_deref().unwrap_or("");
            format!("{}: {}", t.name.as_ref(), d)
        })
        .collect();
    let result = tokio::task::spawn_blocking(move || {
        let q = store.embedder.embed_query(&query_owned).ok()?;
        let mut embs: Vec<Vec<f32>> = Vec::with_capacity(tool_texts.len());
        for text in &tool_texts {
            embs.push(store.embedder.embed(text).ok()?);
        }
        Some((q, embs))
    })
    .await
    .map_err(|e| McpError::internal_error(format!("get_relevant_tools spawn: {}", e), None))?;
    let (query_emb, tool_embs): (Vec<f32>, Vec<Vec<f32>>) = match result {
        Some((q, e)) if e.len() == names.len() => (q, e),
        _ => {
            if include_descriptions {
                let items: Vec<serde_json::Value> = tools
                    .iter()
                    .take(top_k)
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name.as_ref(),
                            "description": t.description.as_deref().unwrap_or("")
                        })
                    })
                    .collect();
                let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }
            let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }
    };
    let mut scored: Vec<(usize, f32)> = tool_embs
        .iter()
        .enumerate()
        .map(|(i, emb)| {
            let sim = query_emb
                .iter()
                .zip(emb.iter())
                .map(|(a, b)| a * b)
                .sum::<f32>();
            (i, sim)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_indices: Vec<usize> = scored.into_iter().take(top_k).map(|(i, _)| i).collect();
    let json = if include_descriptions {
        let items: Vec<serde_json::Value> = top_indices
            .iter()
            .map(|&i| {
                let t = &tools[i];
                serde_json::json!({
                    "name": t.name.as_ref(),
                    "description": t.description.as_deref().unwrap_or("")
                })
            })
            .collect();
        serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
    } else {
        let top_names: Vec<String> = top_indices.iter().map(|&i| names[i].clone()).collect();
        serde_json::to_string(&top_names).unwrap_or_else(|_| "[]".to_string())
    };
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// Params for get_codebase_outline (compressed structural view: file path + symbols).
pub struct GetCodebaseOutlineParams {
    /// Workspace root to scan (default: first allowed root).
    #[serde(default)]
    pub workspace_path: Option<String>,
    /// Max symbols to return (default 2000).
    #[serde(default)]
    pub max_items: Option<u32>,
}

const CODEBASE_OUTLINE_EXTENSIONS: &[&str] = &["rs", "py", "ts", "tsx", "js", "jsx"];
const CODEBASE_OUTLINE_MAX_FILES: usize = 500;

/// Compressed structural outline: for each file under workspace, list path and symbol names (from AST). Token-efficient mental map.
pub async fn get_codebase_outline_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    p: GetCodebaseOutlineParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let roots = &handler.store.allowed_roots;
    if roots.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_codebase_outline: no ALLOWED_ROOTS configured.",
        )]));
    }
    let workspace_path = p
        .workspace_path
        .as_deref()
        .map(std::path::Path::new)
        .and_then(|pth| {
            let canonical = pth.canonicalize().ok()?;
            roots
                .iter()
                .find(|r| canonical.starts_with(r.as_path()))
                .map(|_| canonical)
        })
        .unwrap_or_else(|| roots[0].clone());
    let max_items = p.max_items.unwrap_or(2000).min(10_000) as usize;
    let mut lines: Vec<String> = Vec::new();
    let mut total = 0usize;
    let mut files_scanned = 0usize;
    for entry in walkdir::WalkDir::new(&workspace_path)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if total >= max_items || files_scanned >= CODEBASE_OUTLINE_MAX_FILES {
            break;
        }
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !CODEBASE_OUTLINE_EXTENSIONS.contains(&ext) {
            continue;
        }
        files_scanned += 1;
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let extraction = crate::rag::symbols::extract_symbols(&content, ext);
        let rel = path
            .strip_prefix(&workspace_path)
            .unwrap_or(path)
            .display()
            .to_string();
        for name in &extraction.defines {
            if total >= max_items {
                break;
            }
            lines.push(format!("{}: {}", rel, name));
            total += 1;
        }
    }
    let text = if lines.is_empty() {
        "No symbols found (empty workspace or unsupported extensions).".to_string()
    } else {
        format!(
            "{}\n\n[Total {} symbols from {} files]",
            lines.join("\n"),
            total,
            files_scanned
        )
    };
    Ok(CallToolResult::success(vec![Content::text(
        apply_response_format(text),
    )]))
}
