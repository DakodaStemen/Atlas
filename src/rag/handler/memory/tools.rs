//! Implementation functions for memory tools; called from handler mod.rs tool methods.

use super::super::{
    internal_error_sanitized, AgenticHandler, IngestionProvider, VectorStoreProvider,
};
use super::{
    commit_to_memory_retry_attempts, commit_to_memory_retry_base_ms, humanize_text,
    is_retryable_io_error, ApprovePatternParams, CommitToMemoryParams, LogTrainingRowParams,
    RefreshFileIndexParams,
};
use crate::rag::cli_helpers::run_refresh_file_index;
use crate::rag::ingest::ingest_directory_parallel;
use crate::tools::slack;
use crate::tools::web::reject_private_or_reserved_host;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tracing::warn;

pub async fn commit_to_memory_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: CommitToMemoryParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let project_root = handler
        .store
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let docs_dir = project_root.join("docs");
    let lessons_path = docs_dir.join("lessons_learned.md");

    let mut title = params.title.trim().to_string();
    let mut lesson = params.lesson.trim().to_string();
    let category = params.category.trim();
    if title.is_empty() || lesson.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "commit_to_memory requires non-empty title and lesson.",
        )]));
    }

    if let Ok(url_str) = std::env::var("HUMANIZER_URL") {
        let url_str = url_str.trim().to_string();
        let ssrf_ok = url_str.starts_with("https://")
            && url::Url::parse(&url_str)
                .map(|parsed| reject_private_or_reserved_host(&parsed).is_ok())
                .unwrap_or(false);
        if ssrf_ok {
            let lesson_in = lesson.clone();
            let title_in = title.clone();
            let (l, t) = tokio::task::spawn_blocking(move || {
                let l = humanize_text(&lesson_in, &url_str).unwrap_or(lesson_in);
                let t = humanize_text(&title_in, &url_str).unwrap_or(title_in);
                (l, t)
            })
            .await
            .map_err(|e| McpError::internal_error(format!("humanizer spawn: {}", e), None))?;
            lesson = l;
            title = t;
        } else {
            tracing::warn!(
                "HUMANIZER_URL must be an https:// URL pointing to a public host; humanizer skipped."
            );
        }
    }

    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let entry = format!(
        "\n## {}: {}\n\n**Category:** {}\n\n{}\n\n",
        date,
        title.replace('\n', " "),
        if category.is_empty() {
            "general"
        } else {
            category
        },
        lesson
    );
    let retry_attempts = commit_to_memory_retry_attempts();
    let base_ms = commit_to_memory_retry_base_ms();
    let mut last_err = None::<String>;
    for attempt in 0..retry_attempts {
        if attempt > 0 {
            let ms = base_ms * (1 << attempt);
            std::thread::sleep(Duration::from_millis(ms));
        }
        if !docs_dir.exists() {
            match std::fs::create_dir_all(&docs_dir) {
                Ok(()) => {}
                Err(e) => {
                    if is_retryable_io_error(&e) {
                        last_err = Some(e.to_string());
                        continue;
                    }
                    return Err(internal_error_sanitized(
                        "commit_to_memory (create_dir)",
                        &e,
                    ));
                }
            }
        }
        let open_result = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&lessons_path);
        let mut f = match open_result {
            Ok(file) => file,
            Err(e) => {
                if is_retryable_io_error(&e) {
                    last_err = Some(e.to_string());
                    continue;
                }
                return Err(internal_error_sanitized("commit_to_memory (open)", &e));
            }
        };
        if let Err(e) = f.write_all(entry.as_bytes()) {
            if is_retryable_io_error(&e) {
                last_err = Some(e.to_string());
                continue;
            }
            return Err(internal_error_sanitized("commit_to_memory (write)", &e));
        }
        if let Err(e) = f.flush() {
            if is_retryable_io_error(&e) {
                last_err = Some(e.to_string());
                continue;
            }
            return Err(internal_error_sanitized("commit_to_memory (flush)", &e));
        }
        last_err = None;
        break;
    }
    if let Some(e) = last_err {
        warn!(
            "commit_to_memory failed after {} retries (server): {}",
            retry_attempts, e
        );
        return Err(McpError::internal_error(
            format!("commit_to_memory failed after {} retries.", retry_attempts),
            None,
        ));
    }

    if let Some(ref manifest_path) = handler.ingest_manifest_path {
        let db = handler.store.db.clone();
        let db_checkpoint = handler.store.db.clone();
        let embedder = handler.store.embedder.clone();
        let allowed_roots = handler.store.allowed_roots.clone();
        let docs = docs_dir.clone();
        let manifest = manifest_path.clone();
        tokio::spawn(async move {
            if let Err(e) =
                ingest_directory_parallel(docs.as_path(), db, embedder, allowed_roots, manifest)
                    .await
            {
                tracing::warn!(
                    "commit_to_memory background re-index failed — lessons_learned.md \
                     will not be searchable until next ingest: {}",
                    e
                );
            }
            db_checkpoint.wal_checkpoint_passive_retry();
        });
    }

    if let Ok(url) = std::env::var("SLACK_WEBHOOK_URL_EVOLUTION") {
        let url = url.trim().to_string();
        if !url.is_empty() {
            let msg = format!(
                "New lesson: {} (category: {})",
                title,
                if category.is_empty() {
                    "general"
                } else {
                    category
                }
            );
            tokio::task::spawn_blocking(move || {
                if let Err(e) = slack::notify_slack(&url, &msg) {
                    tracing::warn!("Slack evolution webhook failed: {}", e);
                }
            });
        }
    }

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Appended lesson to {}. Background re-index of docs/ started.",
        lessons_path.display()
    ))]))
}

pub async fn log_training_row_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: LogTrainingRowParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let guard = match &handler.dataset_collector {
        Some(g) => g.clone(),
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "Training log not configured (dataset_collector unavailable).",
            )]));
        }
    };
    let query = params.query;
    let mut context = params.context;
    let mut response = params.response;
    let domain = params.domain.unwrap_or_else(|| "ouroboros".to_string());
    if let Ok(url) = std::env::var("HUMANIZER_URL") {
        let context_in = context.clone();
        let response_in = response.clone();
        let (c, r) = tokio::task::spawn_blocking(move || {
            let c = humanize_text(&context_in, &url).unwrap_or(context_in);
            let r = humanize_text(&response_in, &url).unwrap_or(response_in);
            (c, r)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("humanizer spawn: {}", e), None))?;
        context = c;
        response = r;
    }
    let query_for_slack = query.clone();
    let domain_for_slack = domain.clone();
    tokio::task::spawn_blocking(move || match guard.lock() {
        Ok(c) => {
            let _ = c.record_interaction(&query, &context, &response, &domain);
        }
        Err(e) => tracing::warn!("dataset_collector lock poisoned: {}", e),
    })
    .await
    .map_err(|e| McpError::internal_error(format!("log_training_row spawn: {}", e), None))?;

    if let Ok(url) = std::env::var("SLACK_WEBHOOK_URL_LOG") {
        let url = url.trim().to_string();
        if !url.is_empty() {
            let msg = format!("Logged: {} [{}]", query_for_slack, domain_for_slack);
            tokio::task::spawn_blocking(move || {
                if let Err(e) = slack::notify_slack(&url, &msg) {
                    tracing::warn!("Slack log webhook failed: {}", e);
                }
            });
        }
    }

    Ok(CallToolResult::success(vec![Content::text(
        "Training row logged (or skipped if low-value).",
    )]))
}

pub async fn approve_pattern_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ApprovePatternParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let guard = match &handler.dataset_collector {
        Some(g) => g.clone(),
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "Training log not configured (dataset_collector unavailable).",
            )]));
        }
    };
    let name = params.name;
    let code = params.code;
    let language = params.language;

    let name_success = name.clone();
    let store = handler.store.clone();

    // Wrap both dataset_collector write and store.db write in a single
    // spawn_blocking so they are atomic from the caller's perspective.
    let name_inner = name.clone();
    let code_inner = code.clone();
    let lang_inner = language.clone();
    tokio::task::spawn_blocking(move || {
        // Step 1: write to dataset_collector
        let collector_ok = match guard.lock() {
            Ok(c) => match c.approve_pattern(&name_inner, &code_inner, lang_inner.as_deref()) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!("approve_pattern dataset_collector error: {}", e);
                    false
                }
            },
            Err(e) => {
                tracing::warn!("dataset_collector lock poisoned: {}", e);
                false
            }
        };

        // Step 2: embed and write to store.db
        let db_ok = match store.embedder.embed(&code_inner) {
            Ok(emb) => {
                let id = format!("{}::{}", name_inner, chrono::Utc::now().timestamp_millis());
                match store.db.insert_golden_pattern(
                    &id,
                    &name_inner,
                    &code_inner,
                    lang_inner.as_deref().unwrap_or("text"),
                    &emb,
                ) {
                    Ok(()) => true,
                    Err(e) => {
                        tracing::warn!("approve_pattern insert_golden_pattern error: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                tracing::warn!("approve_pattern embedding error: {}", e);
                false
            }
        };

        if !collector_ok || !db_ok {
            tracing::warn!(
                "approve_pattern partial failure: collector_ok={}, db_ok={}",
                collector_ok,
                db_ok
            );
        }
    })
    .await
    .map_err(|e| McpError::internal_error(format!("approve_pattern spawn: {}", e), None))?;

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Pattern '{}' approved and added to Golden Set.",
        name_success
    ))]))
}

pub async fn auto_approve_pattern_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ApprovePatternParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    const MAX_CODE_LEN: usize = 8000;
    const MIN_PATTERN_LEN: usize = 100;
    const MAX_PATTERN_LEN: usize = 4000;
    const SIMILARITY_DISTANCE_THRESHOLD: f64 = 1.0;

    let name = params.name.trim().to_string();
    let code = params.code.trim();
    let language = params.language.clone();

    if code.len() > MAX_CODE_LEN {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "Not auto-approved: code too long (max {} chars).",
            MAX_CODE_LEN
        ))]));
    }

    let store = handler.store.clone();
    let store_embed = handler.store.clone();
    let code_owned = code.to_string();
    let embedding = tokio::task::spawn_blocking({
        let code = code_owned.clone();
        move || store_embed.embedder.embed(&code).ok()
    })
    .await
    .map_err(|e| McpError::internal_error(format!("auto_approve spawn: {}", e), None))?;

    let embedding = match embedding {
        Some(emb) => emb,
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "Not auto-approved: embedding failed (embedder unavailable or error).",
            )]));
        }
    };

    let similar = store
        .db
        .search_golden_patterns_knn(&embedding, 5)
        .ok()
        .unwrap_or_default();
    let min_distance = similar.first().map(|(_, d)| *d).unwrap_or(f64::INFINITY);
    let is_similar_to_approved = min_distance < SIMILARITY_DISTANCE_THRESHOLD;
    let is_new_reasonable_pattern =
        similar.is_empty() && code.len() >= MIN_PATTERN_LEN && code.len() <= MAX_PATTERN_LEN;

    if !is_similar_to_approved && !is_new_reasonable_pattern {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "Not auto-approved: no similar preferred pattern (min distance {:.3}) and code outside pattern size ({}-{} chars).",
            min_distance, MIN_PATTERN_LEN, MAX_PATTERN_LEN
        ))]));
    }

    let guard = match &handler.dataset_collector {
        Some(g) => g.clone(),
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "Training log not configured (dataset_collector unavailable).",
            )]));
        }
    };
    let name_w = name.clone();
    let code_w = code.to_string();
    let lang_w = language.clone();
    tokio::task::spawn_blocking(move || match guard.lock() {
        Ok(c) => {
            if let Err(e) = c.approve_pattern(&name_w, &code_w, lang_w.as_deref()) {
                tracing::warn!("auto_approve_pattern approve_pattern error: {}", e);
            }
        }
        Err(e) => tracing::warn!("dataset_collector lock poisoned: {}", e),
    })
    .await
    .map_err(|e| McpError::internal_error(format!("auto_approve_pattern spawn: {}", e), None))?;

    let store2 = handler.store.clone();
    let name_emb = name.clone();
    let code_emb = code.to_string();
    let lang_emb = language.clone();
    let _ = tokio::task::spawn_blocking(move || {
        let id = format!("{}::{}", name_emb, chrono::Utc::now().timestamp_millis());
        let _ = store2.db.insert_golden_pattern(
            &id,
            &name_emb,
            &code_emb,
            lang_emb.as_deref().unwrap_or("text"),
            &embedding,
        );
    })
    .await;

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Pattern '{}' auto-approved and added to Golden Set (pattern recognition + preferences).",
        name
    ))]))
}

pub async fn refresh_file_index_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: RefreshFileIndexParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let paths_input: Vec<String> = params
        .paths
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if paths_input.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "Refreshed index for 0 files.",
        )]));
    }

    let paths: Vec<PathBuf> = paths_input.iter().map(PathBuf::from).collect();
    let db = handler.store.db.clone();
    let embedder = handler.store.embedder.clone();
    let allowed_roots = handler.store.allowed_roots.clone();
    let manifest_path = handler.ingest_manifest_path.clone();

    let count = tokio::task::spawn_blocking(move || {
        run_refresh_file_index(
            db.as_ref(),
            embedder.as_ref(),
            &allowed_roots,
            manifest_path.as_deref(),
            &paths,
        )
    })
    .await
    .map_err(|e| internal_error_sanitized("refresh_file_index (spawn)", &e))?
    .map_err(|e| internal_error_sanitized("refresh_file_index", &e))?;

    handler.store.db.wal_checkpoint_passive_retry();
    // Invalidate semantic cache after re-indexing to avoid stale cached responses.
    if let Err(e) = handler.store.db.semantic_cache_prune(0, 0) {
        tracing::warn!("semantic_cache invalidation after refresh: {}", e);
    }
    Ok(CallToolResult::success(vec![Content::text(format!(
        "Refreshed index for {} files. Semantic cache cleared.",
        count
    ))]))
}
