//! save_rule_to_memory and propose_vault_rule implementations.

use super::super::{
    internal_error_sanitized, AgenticHandler, IngestionProvider, VectorStoreProvider,
};
use super::SaveRuleToMemoryParams;
use crate::rag::ingest::{ingest_single_file, load_manifest, save_manifest};
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use std::io::Write;
use std::path::PathBuf;

pub async fn save_rule_to_memory_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: SaveRuleToMemoryParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let rule = params.rule.trim();
    if rule.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "save_rule_to_memory requires non-empty rule.",
        )]));
    }

    let project_root = handler
        .store
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let docs_dir = project_root.join("docs");
    let rules_path = docs_dir.join("agent_rules.md");

    if !docs_dir.exists() {
        std::fs::create_dir_all(&docs_dir)
            .map_err(|e| internal_error_sanitized("save_rule_to_memory (create_dir)", &e))?;
    }
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let entry = format!("\n\n## Rule ({})\n\n{}\n", timestamp, rule);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rules_path)
        .map_err(|e| internal_error_sanitized("save_rule_to_memory (open)", &e))?;
    f.write_all(entry.as_bytes())
        .map_err(|e| internal_error_sanitized("save_rule_to_memory (write)", &e))?;
    f.flush()
        .map_err(|e| internal_error_sanitized("save_rule_to_memory (flush)", &e))?;

    let db = handler.store.db.clone();
    let embedder = handler.store.embedder.clone();
    let allowed_roots = handler.store.allowed_roots.clone();
    let manifest_path = handler.ingest_manifest_path.clone();
    let path_to_refresh = rules_path.clone();

    let _refreshed = tokio::task::spawn_blocking(move || {
        let mut manifest = match &manifest_path {
            Some(p) => load_manifest(p.as_path()),
            None => std::collections::HashMap::new(),
        };
        let path = path_to_refresh.as_path();
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let under = allowed_roots.iter().any(|r| canonical.starts_with(r));
        if !under {
            return Ok(0u32);
        }
        match ingest_single_file(
            &canonical,
            db.as_ref(),
            embedder.as_ref(),
            &allowed_roots,
            &mut manifest,
        ) {
            Ok(Some(_)) => {
                if let Some(ref p) = manifest_path {
                    if let Err(e) = save_manifest(p.as_path(), &manifest) {
                        tracing::warn!("save_rule_to_memory: failed to save manifest: {}", e);
                    }
                }
                Ok(1u32)
            }
            Ok(None) => Ok(0u32),
            Err(e) => Err(e),
        }
    })
    .await
    .map_err(|e| McpError::internal_error(format!("save_rule_to_memory spawn: {}", e), None))?
    .map_err(|e| McpError::internal_error(format!("save_rule_to_memory ingest: {}", e), None))?;
    handler.store.db.wal_checkpoint_passive_retry();

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Appended rule to {}. RAG index refreshed for that file.",
        rules_path.display()
    ))]))
}

pub async fn propose_vault_rule_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: SaveRuleToMemoryParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let rule = params.rule.trim();
    if rule.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "propose_vault_rule requires non-empty rule.",
        )]));
    }
    let vault_dir = match &handler.vault_dir {
        Some(d) => d.clone(),
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "propose_vault_rule requires VAULT_DIR or HOLLOW_VAULT to be set.",
            )]));
        }
    };
    let allow = std::env::var("ALLOW_VAULT_RULE_PROPOSAL")
        .map(|v| matches!(v.to_lowercase().trim(), "true" | "1" | "yes"))
        .unwrap_or(false);
    if !allow {
        return Ok(CallToolResult::success(vec![Content::text(
            "propose_vault_rule is guarded. Set ALLOW_VAULT_RULE_PROPOSAL=true to enable.",
        )]));
    }
    let rules_dir = vault_dir.join("00_Meta").join("Rules");
    let rules_path = rules_dir.join("agent_rules.md");
    if !rules_dir.exists() {
        std::fs::create_dir_all(&rules_dir)
            .map_err(|e| internal_error_sanitized("propose_vault_rule (create_dir)", &e))?;
    }
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let entry = format!("\n\n## Rule ({})\n\n{}\n", timestamp, rule);
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rules_path)
        .map_err(|e| internal_error_sanitized("propose_vault_rule (open)", &e))?;
    f.write_all(entry.as_bytes())
        .map_err(|e| internal_error_sanitized("propose_vault_rule (write)", &e))?;
    f.flush()
        .map_err(|e| internal_error_sanitized("propose_vault_rule (flush)", &e))?;

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Appended rule to {} (Vault). Run Synapse to refresh .cursorrules if needed.",
        rules_path.display()
    ))]))
}
