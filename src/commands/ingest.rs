use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rag_mcp::config::{load_web_sources, web_sources_path};
use rag_mcp::rag::{ingest_directory_parallel, ingest_from_jsonl, RagDb};
use rag_mcp::tools::web::fetch_url_as_markdown_clean;

/// Min delay between web fetches in ingest-web to avoid rate limits.
const WEB_RATE_DELAY_SECS: u64 = 2;
/// Max retries per URL in ingest-web on fetch failure.
const WEB_FETCH_RETRIES: u32 = 3;

/// Run one-shot ingest on a directory (used by janitor-cycle and Ingest CLI). Adds path to allowed_roots if not already under any.
pub(crate) fn run_ingest(
    config: &rag_mcp::config::Config,
    path: &Path,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let rag_db = Arc::new(RagDb::open(&config.db_path)?);
    let embedder = Arc::new(super::load_nomic_embedder(&config.nomic_path));
    if !embedder.is_available() {
        eprintln!(
            "ingest: ORT_DYLIB_PATH not set or embedder unavailable. Chunks will be indexed for FTS only (no vectors). Set ORT_DYLIB_PATH for hybrid search."
        );
    }
    let mut allowed_roots = config.allowed_roots.clone();
    let ingest_root = path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|| path.to_path_buf())
        }
    });
    let under_any = allowed_roots.iter().any(|r| ingest_root.starts_with(r));
    if !under_any {
        allowed_roots.push(ingest_root.clone());
        tracing::info!(ingest_root = %ingest_root.display(), "ingest: added path to allowed roots");
    }
    let manifest_path = config.data_dir.join("rag_manifest.json");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let count = rt.block_on(ingest_directory_parallel(
        path,
        rag_db,
        embedder,
        allowed_roots,
        manifest_path,
    ))?;
    Ok(count)
}

/// Fetch URLs from web_sources.json, write web.jsonl, ingest into RAG; optional prune by age.
pub(crate) fn run_ingest_web(
    config: &rag_mcp::config::Config,
    prune_after_days: Option<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sources_path = web_sources_path(&config.data_dir);
    let raw_entries = load_web_sources(&sources_path);
    // One line per URL in web.jsonl: dedupe by URL (keep last occurrence).
    let mut by_url: HashMap<String, rag_mcp::config::WebSourceEntry> = HashMap::new();
    for entry in raw_entries {
        by_url.insert(entry.url.trim().to_string(), entry);
    }
    let entries: Vec<_> = by_url.into_values().collect();
    if entries.is_empty() {
        eprintln!(
            "ingest-web: no URLs in {} (or file missing). Add a JSON array of {{ \"url\": \"https://...\", \"domain\": \"optional\" }}.",
            sources_path.display()
        );
        std::fs::create_dir_all(&config.data_dir)?;
        let web_jsonl = config.data_dir.join("web.jsonl");
        std::fs::write(&web_jsonl, b"")?;
        let rag_db = Arc::new(RagDb::open(&config.db_path)?);
        let embedder = Arc::new(super::load_nomic_embedder(&config.nomic_path));
        let _ = ingest_from_jsonl(&web_jsonl, &rag_db, &embedder)?;
        if let Some(days) = prune_after_days {
            let n = rag_db.prune_web_chunks_older_than(days)?;
            eprintln!(
                "ingest-web: pruned {} web chunks older than {} days.",
                n, days
            );
        }
        return Ok(());
    }

    std::fs::create_dir_all(&config.data_dir)?;
    let web_jsonl = config.data_dir.join("web.jsonl");
    let mut file = std::fs::File::create(&web_jsonl)?;
    let mut last_domain_fetch: HashMap<String, Instant> = HashMap::new();
    let mut failed_first_pass: Vec<(String, String, Option<String>)> = Vec::new();

    for entry in &entries {
        let url = entry.url.trim();
        let domain = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .unwrap_or_else(|| "unknown".to_string());
        // Rate limit: wait if we fetched this domain recently
        if let Some(&last) = last_domain_fetch.get(&domain) {
            let elapsed = last.elapsed();
            if elapsed < Duration::from_secs(WEB_RATE_DELAY_SECS) {
                std::thread::sleep(Duration::from_secs(WEB_RATE_DELAY_SECS) - elapsed);
            }
        }
        let mut last_err = None;
        for attempt in 0..WEB_FETCH_RETRIES {
            match fetch_url_as_markdown_clean(url) {
                Ok(markdown) => {
                    let source = url.to_string();
                    let line = serde_json::json!({
                        "path": source,
                        "text": markdown,
                        "domain": entry.domain.as_deref().unwrap_or("external_docs")
                    });
                    writeln!(file, "{}", line)?;
                    last_domain_fetch.insert(domain.clone(), Instant::now());
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt + 1 < WEB_FETCH_RETRIES {
                        std::thread::sleep(Duration::from_secs(1 << attempt));
                    }
                }
            }
        }
        if last_err.is_some() && !last_domain_fetch.contains_key(&domain) {
            failed_first_pass.push((url.to_string(), domain, entry.domain.clone()));
            tracing::warn!(
                "ingest-web: skip {} after {} retries: {:?}",
                url,
                WEB_FETCH_RETRIES,
                last_err
            );
        }
    }

    // Second pass: retry failed URLs once more
    let mut skipped_count: u32 = 0;
    for (url, domain, domain_opt) in &failed_first_pass {
        if let Some(&last) = last_domain_fetch.get(domain) {
            let elapsed = last.elapsed();
            if elapsed < Duration::from_secs(WEB_RATE_DELAY_SECS) {
                std::thread::sleep(Duration::from_secs(WEB_RATE_DELAY_SECS) - elapsed);
            }
        }
        let mut last_err = None;
        for attempt in 0..WEB_FETCH_RETRIES {
            match fetch_url_as_markdown_clean(url) {
                Ok(markdown) => {
                    let line = serde_json::json!({
                        "path": url,
                        "text": markdown,
                        "domain": domain_opt.as_deref().unwrap_or("external_docs")
                    });
                    writeln!(file, "{}", line)?;
                    last_domain_fetch.insert(domain.clone(), Instant::now());
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt + 1 < WEB_FETCH_RETRIES {
                        std::thread::sleep(Duration::from_secs(1 << attempt));
                    }
                }
            }
        }
        if last_err.is_some() && !last_domain_fetch.contains_key(domain) {
            skipped_count += 1;
            tracing::warn!(
                "ingest-web: skip {} after second-pass retries: {:?}",
                url,
                last_err
            );
        }
    }

    let rag_db = Arc::new(RagDb::open(&config.db_path)?);
    let embedder = Arc::new(super::load_nomic_embedder(&config.nomic_path));
    let count = ingest_from_jsonl(&web_jsonl, &rag_db, &embedder)?;
    eprintln!("ingest-web: {} sources written to RAG.", count);

    if let Some(days) = prune_after_days {
        let n = rag_db.prune_web_chunks_older_than(days)?;
        eprintln!(
            "ingest-web: pruned {} web chunks older than {} days.",
            n, days
        );
    }
    if skipped_count > 0 {
        eprintln!("ingest-web: {} URLs skipped after retries.", skipped_count);
        return Err(anyhow::anyhow!("{} URLs skipped", skipped_count).into());
    }
    Ok(())
}

/// Ingest from JSONL: each line {"path": "source-id", "text": "..."}. Merge e.g. NotebookLM export into RAG.
pub(crate) fn run_ingest_from_jsonl(
    config: &rag_mcp::config::Config,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rag_db = Arc::new(rag_mcp::rag::RagDb::open(&config.db_path)?);
    let embedder = Arc::new(super::load_nomic_embedder(&config.nomic_path));
    if !embedder.is_available() {
        eprintln!("ingest-from-jsonl: ORT_DYLIB_PATH not set or embedder unavailable. Chunks will be FTS-only (no vectors). Set ORT_DYLIB_PATH for hybrid search.");
    }
    let count = rag_mcp::rag::ingest_from_jsonl(path, &rag_db, &embedder)?;
    eprintln!("Ingest-from-JSONL complete. {} sources indexed.", count);
    Ok(())
}
