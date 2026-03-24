//! Symbol and related-code tools: get_related_code, resolve_symbol, symbol_xml for graph://symbol/{name}.

use super::super::{truncate_rag_response, AgenticHandler, IngestionProvider, VectorStoreProvider};
use crate::rag::db::ChunkRow;
use crate::rag::domain_classifier::classify_source;
use crate::rag::store::{format_related_code_response, RagStore};
use crate::rag::xml::{escape_attr, escape_text};
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// get_related_code: max callee symbols to include in <callee_context> (depth 1).
const MAX_CALLEES_IN_RELATED: usize = 3;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GetRelatedCodeParams.
pub struct GetRelatedCodeParams {
    /// Symbol name (e.g. class or function name) to find definition and references.
    pub symbol_name: String,
    /// Cap on reference chunks returned (saves tokens). Omit to use GET_RELATED_CODE_MAX_REFERENCES env or 25.
    #[serde(default)]
    pub max_references: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ResolveSymbolParams.
pub struct ResolveSymbolParams {
    /// Symbol name for jump-to-definition. Returns defining chunk(s) from symbol_index.
    pub symbol_name: String,
}

fn chunk_context_snippet(text: &str, max_len: usize) -> String {
    let first = text.trim().lines().next().unwrap_or("").trim();
    if first.len() <= max_len {
        first.to_string()
    } else {
        format!(
            "{}...",
            first
                .get(..max_len.saturating_sub(3))
                .unwrap_or(first)
                .trim_end()
        )
    }
}

pub(crate) fn defines_symbol(row: &ChunkRow, symbol_name: &str) -> bool {
    serde_json::from_str::<Vec<String>>(&row.defines)
        .map(|v| v.contains(&symbol_name.to_string()))
        .unwrap_or(false)
}

fn ref_rel(row: &ChunkRow, symbol_name: &str) -> &'static str {
    let imports: Vec<String> = serde_json::from_str(&row.imports).unwrap_or_default();
    if imports.contains(&symbol_name.to_string()) {
        "dependency"
    } else {
        "consumer"
    }
}

fn build_definition_section(store: &RagStore, defining: &[ChunkRow]) -> String {
    let mut out = String::from("<definition>");
    if defining.is_empty() {
        out.push_str("No defining chunk in index.");
    } else {
        for row in defining {
            if store.path_under_allowed(&row.source) {
                out.push_str(&format!(
                    r#"<source path="{}" rel="implementation">{}</source>"#,
                    escape_attr(&row.source),
                    escape_text(row.text.trim())
                ));
            }
        }
        if !defining.iter().any(|r| store.path_under_allowed(&r.source)) {
            out.push_str("No definition in allowed paths.");
        }
    }
    out.push_str("</definition>");
    out
}

fn build_references_section(
    store: &RagStore,
    symbol_name: &str,
    referencing: &[ChunkRow],
) -> String {
    let mut out = String::from("<references>");
    let mut seen = std::collections::HashSet::new();
    for row in referencing {
        if !store.path_under_allowed(&row.source) {
            continue;
        }
        let rel = ref_rel(row, symbol_name);
        let context = escape_attr(&chunk_context_snippet(&row.text, 80));
        let defines: Vec<String> = serde_json::from_str(&row.defines).unwrap_or_default();
        for sym in &defines {
            if sym.is_empty() || seen.contains(sym) {
                continue;
            }
            seen.insert(sym.clone());
            let base = std::path::Path::new(&row.source)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            out.push_str(&format!(
                r#"<ref rel="{}" path="graph://symbol/{}" context="{}">{} (in {})</ref>"#,
                rel,
                escape_attr(sym),
                context,
                escape_text(sym),
                escape_text(base)
            ));
        }
    }
    out.push_str("</references>");
    out
}

/// Build symbol XML for graph://symbol/{name}. Used by read_resource in handler mod.rs.
pub fn symbol_xml(store: &RagStore, symbol_name: &str) -> String {
    let symbol_name = symbol_name.trim();
    if symbol_name.is_empty() {
        return r#"<symbol name=""><definition>Invalid symbol name.</definition><references/></symbol>"#
            .to_string();
    }
    let rows = match store.get_related_code(symbol_name, None) {
        Ok(r) => r,
        Err(_) => {
            return format!(
                r#"<symbol name="{}"><definition>Not found in index.</definition><references/></symbol>"#,
                escape_attr(symbol_name)
            );
        }
    };
    if rows.is_empty() {
        return format!(
            r#"<symbol name="{}"><definition>Not found in index.</definition><references/></symbol>"#,
            escape_attr(symbol_name)
        );
    }
    let (defining, referencing): (Vec<_>, _) = rows
        .into_iter()
        .partition(|r| defines_symbol(r, symbol_name));
    let mut out = format!(r#"<symbol name="{}">"#, escape_attr(symbol_name));
    out.push_str(&build_definition_section(store, &defining));
    out.push_str(&build_references_section(store, symbol_name, &referencing));
    out.push_str("</symbol>");
    out
}

pub async fn get_related_code_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    p: GetRelatedCodeParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let name = p.symbol_name.trim();
    if name.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No defining or importing chunks found for symbol ''.",
        )]));
    }
    let max_ref = p.max_references.map(|n| n as usize);
    let rows = handler
        .store
        .get_related_code(name, max_ref)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    if rows.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "No defining or importing chunks found for symbol '{}'.",
            name
        ))]));
    }
    let raw = format_related_code_response(&handler.store, &rows, true, MAX_CALLEES_IN_RELATED);
    let text = truncate_rag_response(&handler.store, &raw);
    if !raw.is_empty() {
        if let Some(ref guard) = handler.dataset_collector {
            let guard = Arc::clone(guard);
            let query = format!("get_related_code symbol_name={}", name);
            let text_log = text.clone();
            let domain = rows
                .first()
                .map(|r| classify_source(&r.source))
                .unwrap_or("general")
                .to_string();
            tokio::task::spawn_blocking(move || match guard.lock() {
                Ok(c) => {
                    if let Err(e) = c.record_interaction(&query, &text_log, "", &domain) {
                        tracing::warn!("dataset_collector record: {}", e);
                    }
                }
                Err(e) => tracing::warn!("dataset_collector lock poisoned: {}", e),
            });
        }
    }
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn resolve_symbol_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    p: ResolveSymbolParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let name = p.symbol_name.trim();
    if name.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No symbol name provided.",
        )]));
    }
    let rows = handler
        .store
        .get_related_code(name, Some(0))
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let defining: Vec<ChunkRow> = rows
        .into_iter()
        .filter(|r| defines_symbol(r, name))
        .collect();
    if defining.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "No defining chunk found for symbol '{}'.",
            name
        ))]));
    }
    let raw = format_related_code_response(&handler.store, &defining, false, 0);
    let text = truncate_rag_response(&handler.store, &raw);
    if !raw.is_empty() {
        if let Some(ref guard) = handler.dataset_collector {
            let guard = Arc::clone(guard);
            let query = format!("resolve_symbol symbol_name={}", name);
            let text_log = text.clone();
            let domain = defining
                .first()
                .map(|r| classify_source(&r.source))
                .unwrap_or("general")
                .to_string();
            tokio::task::spawn_blocking(move || match guard.lock() {
                Ok(c) => {
                    if let Err(e) = c.record_interaction(&query, &text_log, "", &domain) {
                        tracing::warn!("dataset_collector record: {}", e);
                    }
                }
                Err(e) => tracing::warn!("dataset_collector lock poisoned: {}", e),
            });
        }
    }
    Ok(CallToolResult::success(vec![Content::text(text)]))
}
