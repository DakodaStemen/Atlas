use std::sync::Arc;

use rag_mcp::rag::store::format_sandbox_response;
use rag_mcp::rag::{RagDb, RagStore};

/// StressQuery: max extra chunk IDs to add in graph walk during hierarchical search; mirrors handler MAX_EXTRA.
const STRESS_MAX_EXTRA: usize = 10;

/// Run one query and print formatted context to stdout (for Phase 2: rag-mcp query "...").
pub(crate) fn run_query(
    config: &rag_mcp::config::Config,
    query: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let theme = rag_mcp::ui::theme::load_theme(
        config
            .allowed_roots
            .first()
            .map(std::path::PathBuf::as_path),
    );
    let err_wrap = |s: &str| {
        format!(
            "{}{}",
            rag_mcp::ui::theme::ansi_for_role(&theme, "error"),
            s
        ) + rag_mcp::ui::theme::ANSI_RESET
    };
    if !config.db_path.exists() {
        eprintln!(
            "{}",
            err_wrap(&format!(
                "query: RAG DB not found at {}. Run ingest first.",
                config.db_path.display()
            ))
        );
        std::process::exit(1);
    }
    let rag_db = Arc::new(RagDb::open(&config.db_path)?);
    if let Err(e) = rag_db.check_embedding_dimension() {
        eprintln!(
            "{}",
            err_wrap(&format!(
                "query: {} Delete data/rag.db, set ORT_DYLIB_PATH, and re-run ingest.",
                e
            ))
        );
        std::process::exit(1);
    }
    let embedder = Arc::new(super::load_nomic_embedder(&config.nomic_path));
    let reranker = Arc::new(super::load_reranker(&config.reranker_path));
    let store = RagStore::new(
        rag_db,
        embedder.clone(),
        Some(reranker),
        config.allowed_roots.clone(),
    );
    let rows = store.hierarchical_search(query, store.rerank_candidates, STRESS_MAX_EXTRA)?;
    let reranked = store.rerank_results(query, rows, store.rerank_top_k);
    let out = format_sandbox_response(&reranked, &config.allowed_roots);
    println!("{}", out);
    Ok(())
}

/// Run verify-retrieval: same path as query_knowledge (hierarchical_search -> rerank). Exits 1 if no chunks returned.
pub(crate) fn run_verify_retrieval(
    config: &rag_mcp::config::Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !config.db_path.exists() {
        eprintln!(
            "verify-retrieval: RAG DB not found at {}. Run ingest first.",
            config.db_path.display()
        );
        std::process::exit(1);
    }
    let rag_db = Arc::new(RagDb::open(&config.db_path)?);
    rag_db.check_embedding_dimension().map_err(|e| {
        eprintln!(
            "verify-retrieval: {} Delete data/rag.db, set ORT_DYLIB_PATH, and re-run ingest.",
            e
        );
        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
    })?;
    let embedder = Arc::new(super::load_nomic_embedder(&config.nomic_path));
    let reranker = Arc::new(super::load_reranker(&config.reranker_path));
    let store = RagStore::new(
        rag_db,
        embedder.clone(),
        Some(reranker),
        config.allowed_roots.clone(),
    );

    if std::env::var("ORT_DYLIB_PATH").is_ok() && !embedder.is_available() {
        eprintln!("verify-retrieval: ORT_DYLIB_PATH is set but embedder is not available; search will be FTS-only.");
    }

    let queries = ["DatasetCollector", "Where is RAG embedding used?"];
    let mut any_ok = false;
    for q in &queries {
        let rows = store.hierarchical_search(q, store.rerank_candidates, STRESS_MAX_EXTRA)?;
        let reranked = store.rerank_results(q, rows, store.rerank_top_k);
        if !reranked.is_empty() {
            any_ok = true;
            eprintln!(
                "verify-retrieval: query \"{}\" returned {} chunks.",
                q,
                reranked.len()
            );
            break;
        }
    }
    if !any_ok {
        eprintln!("verify-retrieval: FAILED. No chunks returned for any test query.");
        eprintln!("  Run ingest with embeddings (set ORT_DYLIB_PATH, then rag-mcp ingest <path>).");
        eprintln!("  Ensure ALLOWED_ROOTS includes your repo so files are indexed.");
        std::process::exit(1);
    }
    eprintln!("verify-retrieval: OK");
    Ok(())
}

/// Run stress-query: N hierarchical_search queries, report avg/p95 latency.
pub(crate) fn run_stress_query(
    config: &rag_mcp::config::Config,
    count: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rag_db = Arc::new(RagDb::open(&config.db_path)?);
    rag_db.check_embedding_dimension().map_err(|e| {
        eprintln!(
            "stress-query: {} Delete data/rag.db, set ORT_DYLIB_PATH, and re-run ingest.",
            e
        );
        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
    })?;
    let embedder = Arc::new(super::load_nomic_embedder(&config.nomic_path));
    let reranker = Arc::new(super::load_reranker(&config.reranker_path));
    let allowed_roots = config.allowed_roots.clone();
    let store = RagStore::new(rag_db, embedder, Some(reranker), allowed_roots);
    let n = count as usize;
    let query = "Where is RAG embedding used?";
    let mut latencies_ms: Vec<u64> = Vec::with_capacity(n);
    for i in 0..n {
        let start = std::time::Instant::now();
        if let Err(e) = store.hierarchical_search(query, store.rerank_candidates, STRESS_MAX_EXTRA)
        {
            eprintln!("Query stress failed at query {}: {}", i + 1, e);
            return Err(e.into());
        }
        latencies_ms.push(start.elapsed().as_millis() as u64);
    }
    latencies_ms.sort();
    let avg_ms = latencies_ms.iter().sum::<u64>() / n as u64;
    let p95_idx = (n as f64 * 0.95).floor() as usize;
    let p95_idx = p95_idx.min(n.saturating_sub(1));
    let p95_ms = latencies_ms[p95_idx];
    eprintln!(
        "stress-query: {} queries, avg {} ms, p95 {} ms.",
        n, avg_ms, p95_ms
    );
    Ok(())
}

/// Run count-chunks: print number of chunks in the RAG DB.
pub(crate) fn run_count_chunks(
    config: &rag_mcp::config::Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !config.db_path.exists() {
        eprintln!(
            "RAG DB not found at {}. Run ingest first.",
            config.db_path.display()
        );
        std::process::exit(1);
    }
    let rag_db = match RagDb::open(&config.db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open RAG DB: {}", e);
            std::process::exit(1);
        }
    };
    let n = match rag_db.count_chunks() {
        Ok(count) => count,
        Err(e) => {
            eprintln!("Failed to count chunks: {}", e);
            std::process::exit(1);
        }
    };
    println!("{}", n);
    Ok(())
}
