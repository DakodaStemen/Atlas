//! RAG MCP server: single binary, stdio transport. CLI: serve (default) | ingest <path> | ingest-web | bin-to-jsonl.
//! **Command enum:** Serve (MCP on STDIO), Ingest, IngestWeb, StressQuery, Audit, BinToJsonl, VerifyRetrieval, Query, CountChunks, ReviewLessons, Background, TrimTraining, etc.

use clap::{Parser, Subcommand};
use rmcp::transport::async_rw::TransportAdapterAsyncRW;
use rag_mcp::rag::{
    cli_helpers::{run_refresh_file_index, run_verify_integrity},
    verify_ui_integrity_check, AgenticHandler, DatasetCollector, ManagedLoop, RagDb, RagStore,
};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
mod commands;
mod constitution;

/// Embedded tools registry (name + description) for list-tools CLI. Source: docs/tools_registry.json.
const TOOLS_REGISTRY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/tools_registry.json"
));

/// CLI: serve (default) | ingest | ingest-web | stress-query | audit | bin-to-jsonl | verify-retrieval | query | count-chunks | review-lessons | background | compile-constitution | build-web-sources.
#[derive(Parser)]
#[command(name = "rag-mcp")]
#[command(about = "Monolith MCP: RAG + web tools, single binary")]
#[command(version)]
/// Top-level CLI: optional subcommand (default Serve).
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

/// Subcommands: Serve (MCP server), Ingest, StressQuery, Audit, IngestFromJsonl, BinToJsonl, TrimTraining, VerifyRetrieval, IngestWeb, Query, CountChunks, ReviewLessons, Background.
#[derive(Subcommand)]
/// Command.
enum Command {
    /// Run the MCP server on STDIO (default). Training: query_knowledge with execute=true appends to training.jsonl; use for Ouroboros.
    Serve,
    /// Recursively index a directory into the SQLite RAG DB (BLAKE3 manifest, parallel ingest).
    Ingest {
        /// Root directory to ingest.
        path: PathBuf,
    },
    /// Stress-test RAG layer: run N hierarchical_search queries (no MCP server, no training.jsonl). Use Serve for training data capture.
    StressQuery {
        /// Number of queries to run (default 30).
        #[arg(long, default_value = "30")]
        count: u32,
    },
    /// Audit environment: ORT, Nomic, Reranker, Qwen paths. Run before first ingest.
    Audit,
    /// Ingest from JSONL: each line {"path": "source-id", "text": "..."}. Merge e.g. NotebookLM export into RAG.
    IngestFromJsonl {
        /// Path to the JSONL file.
        path: PathBuf,
    },
    /// Clean and deduplicate training.jsonl for LLM pipelines (e.g. Unsloth). Output to file or stdout.
    BinToJsonl {
        /// Write cleaned JSONL to this path. If omitted, write to stdout (e.g. for piping).
        #[arg(long)]
        output: Option<PathBuf>,
        /// Drop rows that would be skipped at write time (run N, ok, no code change, etc.).
        #[arg(long)]
        strip_low_value: bool,
    },
    /// Keep only the last N lines of training.jsonl (default 100_000). Use to cap size without manual editing.
    TrimTraining {
        /// Number of most recent lines to keep. Default 100_000.
        #[arg(long)]
        keep_last: Option<u64>,
    },
    /// Verify RAG retrieval: run fixed queries and assert non-empty chunks. Run after ingest and before training/web ingest.
    VerifyRetrieval,
    /// Fetch URLs from data/web_sources.json (readability + html2md), write data/web.jsonl, then ingest into RAG. Optional prune of old web chunks.
    IngestWeb {
        /// After ingest, prune web chunks older than this many days (default: no prune).
        #[arg(long)]
        prune_after_days: Option<u32>,
    },
    /// Run one query and print formatted context (for Phase 2 verification: rag-mcp query "ownership rules rust").
    Query {
        /// Query string.
        query: String,
    },
    /// Print number of chunks in the RAG DB (workspace_chunks).
    CountChunks,
    /// Memory Review (Janitor): read docs/lessons_learned.md, produce docs/lessons_audit_YYYY-MM-DD.md. If OPENAI_API_KEY or ANTHROPIC_API_KEY is set, call API to classify each lesson as Valid or Anti-Pattern; otherwise write a template for manual review. Run weekly to prevent memory poisoning.
    ReviewLessons,
    /// Run ingest, ingest-web (with prune), and trim-training on a loop. Set and forget: keeps RAG and training fresh with zero maintenance. Use JANITOR_INTERVAL_HOURS (default 24), JANITOR_WEB_PRUNE_DAYS (default 30), JANITOR_TRIM_KEEP_LAST (default 100000). Run in background (e.g. Start-Process -WindowStyle Hidden or nohup).
    Background {
        /// Hours between full cycles (ingest + ingest-web + trim). Default from JANITOR_INTERVAL_HOURS or 24.
        #[arg(long)]
        interval_hours: Option<u64>,
        /// Prune web chunks older than this many days when running ingest-web. Default from JANITOR_WEB_PRUNE_DAYS or 30.
        #[arg(long)]
        web_prune_days: Option<u32>,
        /// Keep last N lines of training.jsonl when trimming. Default from JANITOR_TRIM_KEEP_LAST or 100000.
        #[arg(long)]
        trim_keep_last: Option<u64>,
    },
    /// Chaos engineering: path traversal, AST overload, RRF keyword, secrets. Writes docs/reports/CHAOS_TEST_RESULTS.md.
    Chaos,
    /// Remove RAG DB chunks and manifest entries for workspace paths that no longer exist on disk. Fast sync without re-embedding.
    PruneOrphans,
    /// Re-ingest the given file paths into the RAG index (CLI for task runner). Paths must be under ALLOWED_ROOTS.
    RefreshFileIndex {
        /// File paths to refresh (repeat for multiple).
        #[arg(short, long, num_args = 1..)]
        path: Vec<PathBuf>,
    },
    /// Run cargo check, test, and clippy; print JSON pass/fail (CLI for task runner).
    VerifyIntegrity {
        /// Workspace path (directory containing Cargo.toml). Default: first allowed root.
        #[arg(long)]
        workspace_path: Option<PathBuf>,
    },
    /// Lint UI snippet or file against DESIGN_AXIOMS (shadows, stacking). Print JSON { pass, violations }; exit 1 if violations. For CI/build gate.
    VerifyUiIntegrity {
        /// Path to TSX/JSX or HTML file to lint. If omitted, read snippet from stdin.
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Complexity report: list .rs files under path with line counts; flag files over 500 lines (complexity cap). Use before refactors. Path defaults to first ALLOWED_ROOT or current dir.
    Complexity {
        /// Directory to scan (default: first allowed root or current dir).
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Compile hub rules (AGENTIC_OPERATOR_RULE, lessons_learned, RED_LINES, GSD_AND_MONOLITH) into a single constitution file. Replaces scripts/compile_synapse_context.ps1.
    CompileConstitution {
        /// Output path for CONSTITUTION.md. Default: repo root / CONSTITUTION.md.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Repo root (where docs/ lives). Default: inferred from cwd or monolith parent.
        #[arg(long)]
        repo_root: Option<PathBuf>,
    },
    /// Generate data/web_sources.json from docs/setup/doc_manifest.json. Replaces scripts/build_web_sources.ps1.
    BuildWebSources {
        /// Run ingest-web after writing web_sources.json.
        #[arg(long)]
        run_ingest: bool,
        /// When run_ingest: prune web chunks older than this many days.
        #[arg(long)]
        prune_after_days: Option<u32>,
    },
    /// One-shot janitor: ingest (repo root), ingest-web (with prune), trim-training. Replaces scripts/run_janitor_cycle.ps1.
    JanitorCycle {
        /// Web chunks older than this many days are pruned. Default from JANITOR_WEB_PRUNE_DAYS or 30.
        #[arg(long)]
        web_prune_days: Option<u32>,
        /// Keep last N lines of training.jsonl. Default from JANITOR_TRIM_KEEP_LAST or 100000.
        #[arg(long)]
        trim_keep_last: Option<u64>,
    },
    /// Remove manifest entries for files that no longer exist. Updates only rag_manifest.json.
    PruneManifestStale,
    /// Full data clean: training clean, prune-orphans, prune-manifest-stale, ingest-web, trim-training, optional review-lessons. Replaces scripts/run_data_clean.ps1.
    DataClean {
        #[arg(long)]
        web_prune_days: Option<u32>,
        #[arg(long)]
        trim_keep_last: Option<u64>,
        #[arg(long)]
        run_review_lessons: bool,
        #[arg(long)]
        skip_training_clean: bool,
        #[arg(long)]
        intensive: bool,
    },
    /// Print MCP tool names and descriptions (JSON array). Same registry as get_relevant_tools.
    ListTools,
    /// Merge two Alpaca-format JSONL files (instruction, input, output) into one. Replaces scripts/training/merge_unsloth_jsonl.py.
    MergeUnslothJsonl {
        #[arg(short, long)]
        bootstrap: Option<PathBuf>,
        #[arg(short, long)]
        unsloth: Option<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Sample N random rows from a JSONL file (e.g. training.jsonl). Replaces scripts/training/sample_and_audit.py sampling.
    SampleTraining {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long, default_value = "5")]
        n: usize,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Init ONNX Runtime only when ORT_DYLIB_PATH is set (requires ONNX Runtime >= 1.23).
/// Auto-discovery is skipped so an old onnxruntime.dll in the project dir does not cause a version mismatch panic.
#[cfg(feature = "onnx")]
/// Init ONNX Runtime only when ORT_DYLIB_PATH is set; requires ONNX Runtime >= 1.23.
fn try_init_ort(_data_dir: &str) {
    if let Ok(path) = std::env::var("ORT_DYLIB_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            if let Ok(env) = ort::init_from(&p) {
                env.commit();
            } else {
                tracing::warn!("ort init from ORT_DYLIB_PATH failed");
            }
        }
    }
}

#[cfg(not(feature = "onnx"))]
fn try_init_ort(_data_dir: &str) {}

#[cfg(feature = "otel")]
fn init_tracing_with_otel() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::resource::Resource;
    use opentelemetry_sdk::trace::Config;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Registry;

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());
    let resource = Resource::new([KeyValue::new("service.name", "rag-mcp")]);
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&endpoint);
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(Config::default().with_resource(resource))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .map_err(|e| format!("OTel pipeline install: {}", e))?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("rag_mcp=info".parse()?)
        .add_directive("ort=warn".parse()?)
        .add_directive("info".parse()?);
    let fmt = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    Registry::default()
        .with(filter)
        .with(fmt)
        .with(telemetry)
        .init();
    Ok(())
}

#[cfg(not(feature = "otel"))]
fn init_tracing_with_otel() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load .env file if present (non-fatal if missing).
    let _ = dotenvy::dotenv();

    // All server logs go to stderr so JSON-RPC on stdout is not broken.
    // Per-step and overall tool timing: set RUST_LOG=mcp_timing=info to log each step and tool call duration.
    if cfg!(feature = "otel") {
        init_tracing_with_otel()?;
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("rag_mcp=info".parse()?)
                    .add_directive("ort=warn".parse()?)
                    .add_directive("info".parse()?),
            )
            .with_writer(std::io::stderr)
            .init();
    }

    let config = rag_mcp::config::Config::new();

    let data_dir = config.data_dir.clone(); // Clone for local use if needed, or use config ref

    try_init_ort(&data_dir.to_string_lossy());
    std::fs::create_dir_all(&data_dir)?;

    let command = Cli::parse().command.unwrap_or(Command::Serve);
    match command {
        Command::Ingest { path } => {
            let count = commands::run_ingest(&config, path.as_path())?;
            eprintln!("Ingest complete. {} chunks indexed.", count);
            return Ok(());
        }
        Command::StressQuery { count } => {
            commands::run_stress_query(&config, count)?;
            return Ok(());
        }
        Command::Audit => {
            commands::run_audit(&config);
            return Ok(());
        }
        Command::IngestFromJsonl { path } => {
            commands::run_ingest_from_jsonl(&config, path.as_path())?;
            return Ok(());
        }
        Command::BinToJsonl {
            output,
            strip_low_value,
        } => {
            commands::run_bin_to_jsonl(&config, output.as_deref(), strip_low_value)?;
            return Ok(());
        }
        Command::TrimTraining { keep_last } => {
            commands::run_trim_training(&config, keep_last)?;
            return Ok(());
        }
        Command::VerifyRetrieval => {
            commands::run_verify_retrieval(&config)?;
            return Ok(());
        }
        Command::IngestWeb { prune_after_days } => {
            commands::run_ingest_web(&config, prune_after_days)?;
            return Ok(());
        }
        Command::Query { query } => {
            commands::run_query(&config, &query)?;
            return Ok(());
        }
        Command::CountChunks => {
            commands::run_count_chunks(&config)?;
            return Ok(());
        }
        Command::ReviewLessons => {
            commands::run_review_lessons(&config)?;
            return Ok(());
        }
        Command::Background {
            interval_hours,
            web_prune_days,
            trim_keep_last,
        } => {
            commands::run_background_janitor(
                &config,
                interval_hours,
                web_prune_days,
                trim_keep_last,
            )?;
            return Ok(());
        }
        Command::Chaos => {
            commands::run_chaos(&config)?;
            return Ok(());
        }
        Command::PruneOrphans => {
            commands::run_prune_orphans(&config)?;
            return Ok(());
        }
        Command::RefreshFileIndex { path: paths } => {
            if !config.db_path.exists() {
                eprintln!(
                    "refresh-file-index: RAG DB not found at {}. Run ingest first.",
                    config.db_path.display()
                );
                std::process::exit(1);
            }
            let rag_db = RagDb::open(&config.db_path)?;
            let embedder = commands::load_nomic_embedder(&config.nomic_path);
            let manifest_path = config.data_dir.join("rag_manifest.json");
            let count = run_refresh_file_index(
                &rag_db,
                &embedder,
                &config.allowed_roots,
                Some(manifest_path.as_path()),
                paths.as_slice(),
            )?;
            println!("Refreshed index for {} files.", count);
            rag_db.wal_checkpoint_passive_retry();
            return Ok(());
        }
        Command::VerifyIntegrity { workspace_path } => {
            let mut project_root: PathBuf = if let Some(ref p) = workspace_path {
                p.clone()
            } else {
                config.allowed_roots.first().cloned().unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                })
            };
            if !project_root.join("Cargo.toml").exists() {
                let monolith = project_root.join("monolith");
                if monolith.join("Cargo.toml").exists() {
                    project_root = monolith;
                }
            }
            let json = run_verify_integrity(&project_root);
            println!("{}", json);
            let summary: String = serde_json::from_str(&json)
                .ok()
                .and_then(|v: serde_json::Value| {
                    v.get("summary").and_then(|s| s.as_str()).map(String::from)
                })
                .unwrap_or_else(|| "fail".to_string());
            if summary != "pass" {
                std::process::exit(1);
            }
            return Ok(());
        }
        Command::VerifyUiIntegrity { file } => {
            let snippet = match file {
                Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| {
                    eprintln!(
                        "verify-ui-integrity: failed to read {}: {}",
                        path.display(),
                        e
                    );
                    std::process::exit(1);
                }),
                None => {
                    let mut s = String::new();
                    if std::io::stdin().lock().read_to_string(&mut s).is_err() || s.is_empty() {
                        eprintln!(
                            "verify-ui-integrity: provide --file <path> or pipe snippet on stdin."
                        );
                        std::process::exit(1);
                    }
                    s
                }
            };
            let (pass, violations) = verify_ui_integrity_check(&snippet);
            let json = serde_json::json!({ "pass": pass, "violations": violations });
            println!(
                "{}",
                serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string())
            );
            if !pass {
                std::process::exit(1);
            }
            return Ok(());
        }
        Command::Complexity { path } => {
            commands::run_complexity(path.as_deref(), &config)?;
            return Ok(());
        }
        Command::CompileConstitution { output, repo_root } => {
            let root = constitution::resolve_repo_root(repo_root);
            let out_path = output.unwrap_or_else(|| root.join("CONSTITUTION.md"));
            constitution::run_compile_constitution(&root, &out_path)?;
            return Ok(());
        }
        Command::BuildWebSources {
            run_ingest,
            prune_after_days,
        } => {
            commands::run_build_web_sources(&config, run_ingest, prune_after_days)?;
            return Ok(());
        }
        Command::JanitorCycle {
            web_prune_days,
            trim_keep_last,
        } => {
            commands::run_janitor_cycle(&config, web_prune_days, trim_keep_last)?;
            return Ok(());
        }
        Command::PruneManifestStale => {
            commands::run_prune_manifest_stale(&config)?;
            return Ok(());
        }
        Command::ListTools => {
            println!("{}", TOOLS_REGISTRY_JSON);
            return Ok(());
        }
        Command::MergeUnslothJsonl {
            bootstrap,
            unsloth,
            output,
        } => {
            let data_dir = &config.data_dir;
            let bootstrap_path =
                bootstrap.unwrap_or_else(|| data_dir.join("bootstrap_training.jsonl"));
            let unsloth_path = unsloth.unwrap_or_else(|| data_dir.join("unsloth_training.jsonl"));
            commands::run_merge_unsloth_jsonl(&bootstrap_path, &unsloth_path, &output)?;
            return Ok(());
        }
        Command::SampleTraining { path, n, output } => {
            commands::run_sample_training(path.as_path(), n, output.as_deref())?;
            return Ok(());
        }
        Command::DataClean {
            web_prune_days,
            trim_keep_last,
            run_review_lessons: do_review_lessons,
            skip_training_clean,
            intensive,
        } => {
            commands::data::run_data_clean(
                &config,
                web_prune_days,
                trim_keep_last,
                do_review_lessons,
                skip_training_clean,
                intensive,
            )?;
            return Ok(());
        }
        Command::Serve => {}
    }

    // Optional: start janitor with server so it runs when Cursor starts the MCP server (set JANITOR_WITH_SERVER=true).
    if std::env::var("JANITOR_WITH_SERVER")
        .map(|v| matches!(v.to_lowercase().trim(), "true" | "1" | "yes"))
        .unwrap_or(false)
    {
        if let Ok(exe) = std::env::current_exe() {
            if std::process::Command::new(exe)
                .arg("background")
                .spawn()
                .is_ok()
            {
                tracing::info!("Janitor started with server (JANITOR_WITH_SERVER).");
            }
        }
    }

    let rag_db = Arc::new(RagDb::open(&config.db_path)?);
    if let Err(e) = rag_db.vacuum() {
        tracing::warn!("RAG DB vacuum on startup failed: {} (continuing)", e);
    }
    match rag_db.prune_orphaned_chunk_vectors() {
        Ok(0) => {}
        Ok(n) => tracing::info!("Pruned {} orphaned chunk_vectors rows on startup.", n),
        Err(e) => tracing::warn!(
            "prune_orphaned_chunk_vectors on startup failed: {} (continuing)",
            e
        ),
    }
    if std::env::var("SEMANTIC_CACHE_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        let max_rows: usize = std::env::var("SEMANTIC_CACHE_MAX_ROWS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500);
        let ttl_secs: i64 = std::env::var("SEMANTIC_CACHE_TTL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(86400);
        if let Err(e) = rag_db.semantic_cache_prune(max_rows, ttl_secs) {
            tracing::warn!("semantic_cache_prune on startup failed: {} (continuing)", e);
        }
    }
    if let Err(e) = rag_db.check_embedding_dimension() {
        eprintln!(
            "RAG DB has inconsistent embedding dimension: {}. \
             Delete data/rag.db, set ORT_DYLIB_PATH, and re-run ingest so chunks get 768-d vectors.",
            e
        );
        std::process::exit(1);
    }

    // Load Nomic
    let embedder = Arc::new(commands::load_nomic_embedder(&config.nomic_path));
    if !embedder.is_available() && rag_db.has_any_chunk_vectors().unwrap_or(false) {
        tracing::warn!(
            "RAG DB contains vector data but ORT_DYLIB_PATH is not set; semantic search will be FTS-only. \
             Set ORT_DYLIB_PATH and restart for full hybrid search."
        );
    }

    // Load Reranker
    let reranker = Arc::new(commands::load_reranker(&config.reranker_path));
    if !reranker.is_available() {
        tracing::warn!(
            "Reranker unavailable at startup: query_knowledge will use RRF ranking only (lower precision). \
             Set ORT_DYLIB_PATH and ensure data/models/reranker.onnx + reranker-tokenizer.json exist for two-stage reranking."
        );
    }

    let allowed_roots = config.allowed_roots.clone();
    let store = Arc::new(RagStore::new(
        rag_db,
        embedder.clone(),
        Some(reranker),
        allowed_roots,
    ));

    let (training_path, golden_set_dir) = if let Some(ref d) = config.golden_set_dir {
        (d.join("training.jsonl"), Some(d.clone()))
    } else {
        (
            DatasetCollector::default_path(&data_dir.to_string_lossy()),
            None,
        )
    };
    let dataset_collector = Arc::new(std::sync::Mutex::new(DatasetCollector::new(
        training_path,
        golden_set_dir,
    )));
    let ingest_manifest_path = Some(data_dir.join("rag_manifest.json"));

    let handler = Arc::new(AgenticHandler::new_with_collector(
        store,
        Some(dataset_collector),
        ingest_manifest_path,
        config.tool_selection_guide_path.clone(),
        config.design_tokens_dir.clone(),
        config.global_rules_dir.clone(),
        config.vault_dir.clone(),
    ));

    // Wrap with ManagedLoop to enable autonomous control loop (iteration tracking, stagnation detection).
    let managed_loop = Arc::new(ManagedLoop::new(handler));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        use rmcp::ServiceExt;
        use tokio::sync::watch;
        let gpu_warned = Arc::new(AtomicBool::new(false));
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let gpu_handle = tokio::spawn(async move {
            let interval_secs = std::env::var("GPU_STATS_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60u64);
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let out = tokio::task::spawn_blocking(|| {
                            std::process::Command::new("nvidia-smi")
                                .args([
                                    "--query-gpu=utilization.gpu,memory.used,memory.total",
                                    "--format=csv,noheader",
                                ])
                                .output()
                        })
                        .await;
                        let failed = match &out {
                            Ok(Ok(output)) if output.status.success() => {
                                let line = String::from_utf8_lossy(&output.stdout);
                                let line = line.lines().next().unwrap_or("").trim();
                                tracing::info!(target: "nvidia-smi", "[nvidia-smi] {}", line);
                                false
                            }
                            _ => true,
                        };
                        if failed && !gpu_warned.swap(true, Ordering::Relaxed) {
                            let msg = match &out {
                                Ok(Err(e)) => format!("io: {}", e),
                                Ok(Ok(_)) => "non-zero exit or no output".to_string(),
                                Err(e) => format!("spawn: {}", e),
                            };
                            tracing::warn!("nvidia-smi failed ({}); GPU heartbeat disabled", msg);
                        }
                    }
                    _ = cancel_rx.changed() => break,
                }
            }
        });

        // Blocking stdin reader: dedicated thread with std::io::stdin().lock().read() for Windows
        // named-pipe reliability (Cursor, Windsurf, Claude Desktop). Async token-reading can cause
        // the server to time out on the initialize handshake; blocking I/O for the message listener fixes this.
        let (stdin_tx, stdin_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1024);
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut handle = stdin.lock();
            let mut buf = [0u8; 4096];
            use std::io::Read;
            loop {
                match handle.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let data = buf[..n].to_vec();
                        if stdin_tx.blocking_send(data).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("stdin read error: {}", e);
                        break;
                    }
                }
            }
        });

        /// ChannelReader.
        struct ChannelReader {
            rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
            buffer: Vec<u8>,
        }
        impl tokio::io::AsyncRead for ChannelReader {
            fn poll_read(
                mut self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                if !self.buffer.is_empty() {
                    let amt = std::cmp::min(self.buffer.len(), buf.remaining());
                    let data = self.buffer.drain(..amt).collect::<Vec<_>>();
                    buf.put_slice(&data);
                    return std::task::Poll::Ready(Ok(()));
                }
                match self.rx.poll_recv(cx) {
                    std::task::Poll::Ready(Some(data)) => {
                        let amt = std::cmp::min(data.len(), buf.remaining());
                        buf.put_slice(&data[..amt]);
                        if amt < data.len() {
                            self.buffer.extend_from_slice(&data[amt..]);
                        }
                        std::task::Poll::Ready(Ok(()))
                    }
                    std::task::Poll::Ready(None) => std::task::Poll::Ready(Ok(())), // EOF
                    std::task::Poll::Pending => std::task::Poll::Pending,
                }
            }
        }

        let reader = ChannelReader {
            rx: stdin_rx,
            buffer: Vec::new(),
        };
        let writer = tokio::io::stdout();

        let service = managed_loop
            .serve::<(ChannelReader, tokio::io::Stdout), std::io::Error, TransportAdapterAsyncRW>((reader, writer))
            .await
            .map_err(|e| e.to_string())?;
        eprintln!("Monolith MCP Server (Managed Loop) running on custom STDIO channel");
        tokio::select! {
            result = service.waiting() => {
                result.map_err(|e| e.to_string())?;
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received SIGINT/SIGTERM — shutting down gracefully");
                eprintln!("Received shutdown signal, cleaning up...");
            }
        }
        let _ = cancel_tx.send(true);
        drop(cancel_tx);
        let _ = gpu_handle.await;
        Ok::<(), String>(())
    })?;
    Ok(())
}
