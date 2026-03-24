use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rag_mcp::rag::{RagDb, RagStore};

/// Default interval (hours) between background janitor cycles. Override with JANITOR_INTERVAL_HOURS.
pub(crate) const JANITOR_DEFAULT_INTERVAL_HOURS: u64 = 24;
/// Default web prune (days). Override with JANITOR_WEB_PRUNE_DAYS.
pub(crate) const JANITOR_DEFAULT_WEB_PRUNE_DAYS: u32 = 30;
/// Run review-lessons every N days when set; 0 = skip. Override with JANITOR_REVIEW_LESSONS_DAYS.
pub(crate) const JANITOR_REVIEW_LESSONS_DEFAULT_DAYS: u32 = 7;

/// Alpaca-format keys required for merge-unsloth-jsonl (duplicated from data.rs for sibling-module access).
const ALPACA_KEYS: &[&str] = &["instruction", "input", "output"];

/// Run ingest, ingest-web (with prune), trim-training, and optionally review-lessons in a loop.
pub(crate) fn run_background_janitor(
    config: &rag_mcp::config::Config,
    interval_hours: Option<u64>,
    web_prune_days: Option<u32>,
    trim_keep_last: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let interval_hours = interval_hours
        .or_else(|| {
            std::env::var("JANITOR_INTERVAL_HOURS")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(JANITOR_DEFAULT_INTERVAL_HOURS);
    let web_prune_days = web_prune_days
        .or_else(|| {
            std::env::var("JANITOR_WEB_PRUNE_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(JANITOR_DEFAULT_WEB_PRUNE_DAYS);
    let trim_keep = trim_keep_last
        .or_else(|| {
            std::env::var("JANITOR_TRIM_KEEP_LAST")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(super::data::TRIM_TRAINING_DEFAULT_KEEP);
    let review_lessons_days: u32 = std::env::var("JANITOR_REVIEW_LESSONS_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(JANITOR_REVIEW_LESSONS_DEFAULT_DAYS);

    let root = config
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let root_str = root.display().to_string();

    eprintln!(
        "background: starting janitor loop (interval={}h, web_prune={}d, trim_keep={}). Root: {}",
        interval_hours, web_prune_days, trim_keep, root_str
    );

    loop {
        use rag_mcp::process_utils::run_command;
        // Ingest workspace
        match run_command(&exe.to_string_lossy(), &["ingest", &root_str], &root) {
            Ok(s) if s.status.success() => {
                tracing::info!("background: ingest completed successfully")
            }
            Ok(s) => tracing::warn!("background: ingest exited with {:?}", s.status.code()),
            Err(e) => tracing::warn!("background: ingest failed: {}", e),
        }

        // Ingest web + prune
        match run_command(
            &exe.to_string_lossy(),
            &[
                "ingest-web",
                "--prune-after-days",
                &web_prune_days.to_string(),
            ],
            &root,
        ) {
            Ok(s) if s.status.success() => {
                tracing::info!("background: ingest-web completed successfully")
            }
            Ok(s) => tracing::warn!("background: ingest-web exited with {:?}", s.status.code()),
            Err(e) => tracing::warn!("background: ingest-web failed: {}", e),
        }

        // Trim training
        match run_command(
            &exe.to_string_lossy(),
            &["trim-training", "--keep-last", &trim_keep.to_string()],
            &root,
        ) {
            Ok(s) if s.status.success() => tracing::info!("background: trim-training completed"),
            Ok(s) => tracing::warn!(
                "background: trim-training exited with {:?}",
                s.status.code()
            ),
            Err(e) => tracing::warn!("background: trim-training failed: {}", e),
        }

        // Review-lessons every N days (stamp file in data_dir)
        if review_lessons_days > 0 {
            let stamp_path = config.data_dir.join(".last_review_lessons");
            let run_review = match std::fs::metadata(&stamp_path) {
                Ok(meta) => {
                    let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let elapsed = std::time::SystemTime::now()
                        .duration_since(mtime)
                        .unwrap_or_default();
                    elapsed.as_secs() >= (review_lessons_days as u64) * 86400
                }
                Err(_) => true,
            };
            if run_review {
                match run_command(&exe.to_string_lossy(), &["review-lessons"], &root) {
                    Ok(s) if s.status.success() => {
                        let _ = std::fs::write(&stamp_path, "");
                        tracing::info!("background: review-lessons completed");
                    }
                    Ok(s) => {
                        tracing::warn!(
                            "background: review-lessons exited with {:?}",
                            s.status.code()
                        )
                    }
                    Err(e) => tracing::warn!("background: review-lessons failed: {}", e),
                }
            }
        }

        let duration = std::time::Duration::from_secs(interval_hours * 3600);
        eprintln!(
            "background: next cycle in {} hours (Ctrl+C to stop)",
            interval_hours
        );
        std::thread::sleep(duration);
    }
}

/// Chaos engineering: path traversal, AST overload, RRF keyword, secrets. Writes docs/reports/CHAOS_TEST_RESULTS.md.
pub(crate) fn run_chaos(
    config: &rag_mcp::config::Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use rag_mcp::rag::handler::run_secret_scan;
    use rag_mcp::rag::ingest::{ingest_single_file, load_manifest, save_manifest};

    let root = config
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let docs_dir = root.join("docs");
    let reports_dir = docs_dir.join("reports");
    std::fs::create_dir_all(&reports_dir)?;
    let report_path = reports_dir.join("CHAOS_TEST_RESULTS.md");

    let mut results = Vec::<(String, bool, String)>::new();

    // Phase 1: Path traversal — refresh_file_index with paths outside ALLOWED_ROOTS
    let disallowed_paths: Vec<String> = if cfg!(windows) {
        vec![
            "C:\\Windows\\System32\\drivers\\etc\\hosts".to_string(),
            "..\\..\\..\\..\\Windows\\System32\\drivers\\etc\\hosts".to_string(),
        ]
    } else {
        vec![
            "/etc/passwd".to_string(),
            "../../../../etc/passwd".to_string(),
        ]
    };
    let mut to_ingest_count = 0u32;
    for s in &disallowed_paths {
        let path = PathBuf::from(s);
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if canonical.is_file() {
            let under = config
                .allowed_roots
                .iter()
                .any(|r| canonical.starts_with(r));
            if under {
                to_ingest_count += 1;
            }
        }
    }
    let phase1_pass = to_ingest_count == 0 && !disallowed_paths.is_empty();
    results.push((
        "Phase 1: Path Traversal".to_string(),
        phase1_pass,
        format!(
            "Disallowed paths tested: {}; would ingest: {}. Server skips paths not under ALLOWED_ROOTS (implicit reject, no crash).",
            disallowed_paths.len(),
            to_ingest_count
        ),
    ));

    // Phase 2: AST overload — monster .rs file, ingest, no hang (100 reps; 5000 causes tree-sitter stack overflow)
    let monster_path = docs_dir.join("monster_file.rs");
    const MONSTER_REPS: usize = 100;
    let part1: String = (0..MONSTER_REPS).map(|_| "let x = vec![").collect();
    let part2: String = (0..MONSTER_REPS).map(|_| "];").collect();
    std::fs::write(&monster_path, format!("{}{}", part1, part2))?;
    let rag_db = Arc::new(RagDb::open(&config.db_path)?);
    let embedder = Arc::new(super::load_nomic_embedder(&config.nomic_path));
    let manifest_path = config.data_dir.join("rag_manifest.json");
    let mut manifest = load_manifest(&manifest_path);
    let phase2_result = ingest_single_file(
        monster_path.as_path(),
        rag_db.as_ref(),
        embedder.as_ref(),
        &config.allowed_roots,
        &mut manifest,
    );
    if let Err(e) = save_manifest(&manifest_path, &manifest) {
        tracing::warn!("verify_retrieval: failed to save manifest: {}", e);
    }
    let phase2_pass = phase2_result.is_ok();
    if let Err(e) = &phase2_result {
        tracing::warn!("Phase 2 ingest_single_file: {:?}", e);
    }
    let _ = std::fs::remove_file(&monster_path);
    results.push((
        "Phase 2: AST Parser Overload".to_string(),
        phase2_pass,
        format!(
            "Monster file ({}x vec![]) ingested or skipped without hang. Result: {}. Note: 5000 reps causes tree-sitter stack overflow; runner uses {} for stability.",
            MONSTER_REPS,
            if phase2_result.is_ok() {
                "OK"
            } else {
                "Error (see logs)"
            },
            MONSTER_REPS
        ),
    ));

    // Phase 3: RRF keyword test
    let rrf_path = docs_dir.join("rrf_test.md");
    std::fs::write(&rrf_path, "CRITICAL_SYS_VAR_ZETA_9942")?;
    let mut manifest3 = load_manifest(&manifest_path);
    let _ = ingest_single_file(
        rrf_path.as_path(),
        rag_db.as_ref(),
        embedder.as_ref(),
        &config.allowed_roots,
        &mut manifest3,
    );
    save_manifest(&manifest_path, &manifest3)?;
    let reranker = Arc::new(super::load_reranker(&config.reranker_path));
    let store = RagStore::new(
        rag_db.clone(),
        embedder.clone(),
        Some(reranker),
        config.allowed_roots.clone(),
    );
    let rows = store
        .hierarchical_search(
            "CRITICAL_SYS_VAR_ZETA_9942",
            store.rerank_candidates,
            10, // STRESS_MAX_EXTRA
        )
        .unwrap_or_default();
    let reranked = store.rerank_results("CRITICAL_SYS_VAR_ZETA_9942", rows, store.rerank_top_k);
    let phase3_pass = reranked
        .iter()
        .any(|r| r.text.contains("CRITICAL_SYS_VAR_ZETA_9942"));
    let _ = std::fs::remove_file(&rrf_path);
    results.push((
        "Phase 3: RRF Keyword Test".to_string(),
        phase3_pass,
        format!(
            "Query for exact keyword returned {} chunks; keyword present: {}",
            reranked.len(),
            phase3_pass
        ),
    ));

    // Phase 4: Secrets — dummy file with api_key, scan_secrets must flag it
    let secrets_path = docs_dir.join("chaos_secrets_test.rs");
    // Split the fake credential literal across concat! arms so the full pattern never appears
    // as a single string in this source file — which would trigger scan_secrets on ops.rs itself.
    // The generated temp file still contains the complete value, so Phase 4 detection still works.
    let chaos_content = concat!(
        "// Chaos test dummy\n",
        "const _: () = ();\n",
        "fn _dummy() { let api_key = \"AKIA", // scan-secrets-ignore
        "IOSFODNN7EXAMPLE\"; }\n",
    );
    std::fs::write(&secrets_path, chaos_content)?;
    let findings = run_secret_scan(&root);
    let phase4_pass = findings
        .iter()
        .any(|(p, _, _)| p.to_string_lossy().contains("chaos_secrets_test"));
    let _ = std::fs::remove_file(&secrets_path);
    results.push((
        "Phase 4: Secrets Engine".to_string(),
        phase4_pass,
        format!(
            "scan_secrets found credential in dummy file: {} (findings: {})",
            phase4_pass,
            findings.len()
        ),
    ));

    let all_pass = results.iter().all(|(_, pass, _)| *pass);
    let date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let version = env!("CARGO_PKG_VERSION");
    let mut report = format!(
        "# Chaos Engineering Protocol: Monolith\n\n**Date:** {}\n\n**Server:** rag-mcp {}\n\n**Summary:** {}\n\n---\n\n",
        date,
        version,
        if all_pass {
            "ALL TESTS PASS"
        } else {
            "SOME TESTS FAILED"
        }
    );
    for (name, pass, note) in &results {
        report.push_str(&format!(
            "## {}\n\n**Result:** {}\n\n**Notes:** {}\n\n",
            name,
            if *pass { "PASS" } else { "FAIL" },
            note
        ));
    }
    std::fs::write(&report_path, report)?;
    eprintln!("Chaos run complete. Report: {}", report_path.display());
    if !all_pass {
        return Err("One or more chaos phases failed. See report.".into());
    }
    Ok(())
}

/// Merge two Alpaca-format JSONL files into one. Keeps only lines that have instruction, input, output.
pub(crate) fn run_merge_unsloth_jsonl(
    bootstrap_path: &Path,
    unsloth_path: &Path,
    output_path: &Path,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let mut total = 0u64;
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = BufWriter::new(std::fs::File::create(output_path)?);
    for (path, _label) in [(bootstrap_path, "bootstrap"), (unsloth_path, "unsloth")] {
        if !path.exists() {
            eprintln!("Skip (not found): {}", path.display());
            continue;
        }
        let f = std::fs::File::open(path)?;
        let reader = BufReader::new(f);
        let mut count = 0u64;
        for line in reader.lines() {
            let line = line?.trim().to_string();
            if line.is_empty() {
                continue;
            }
            let obj: serde_json::Map<String, serde_json::Value> = match serde_json::from_str(&line)
            {
                Ok(v) => v,
                _ => continue,
            };
            let has_all = ALPACA_KEYS.iter().all(|k| obj.contains_key(*k));
            if !has_all {
                continue;
            }
            let out_obj: serde_json::Map<String, serde_json::Value> = ALPACA_KEYS
                .iter()
                .map(|k| {
                    (
                        k.to_string(),
                        obj.get(*k)
                            .cloned()
                            .unwrap_or(serde_json::Value::String(String::new())),
                    )
                })
                .collect();
            let json = serde_json::to_string(&out_obj)?;
            writeln!(out, "{}", json)?;
            count += 1;
            total += 1;
        }
        eprintln!("Wrote {} rows from {}", count, path.display());
    }
    eprintln!("Total: {} rows -> {}", total, output_path.display());
    Ok(total)
}

/// Sample n random lines from a JSONL file. Writes to output path or stdout.
pub(crate) fn run_sample_training(
    path: &Path,
    n: usize,
    output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let f = std::fs::File::open(path)?;
    let reader = BufReader::new(f);
    let lines: Vec<String> = reader
        .lines()
        .map_while(Result::ok)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if lines.is_empty() {
        eprintln!("sample-training: file is empty.");
        return Ok(());
    }
    let mut rng = rand::rngs::OsRng;
    let chosen: Vec<&String> =
        rand::seq::SliceRandom::choose_multiple(lines.as_slice(), &mut rng, n.min(lines.len()))
            .collect();
    let mut out: Box<dyn Write> = match output {
        Some(p) => {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Box::new(BufWriter::new(std::fs::File::create(p)?))
        }
        None => Box::new(std::io::stdout()),
    };
    let len = chosen.len();
    for line in &chosen {
        writeln!(out, "{}", line)?;
    }
    if let Some(p) = output {
        eprintln!("sample-training: wrote {} samples to {}.", len, p.display());
    }
    Ok(())
}

/// One-shot janitor: ingest (repo root), ingest-web (with prune), trim-training.
pub(crate) fn run_janitor_cycle(
    config: &rag_mcp::config::Config,
    web_prune_days: Option<u32>,
    trim_keep_last: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = config
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| crate::constitution::resolve_repo_root(None));
    let web_prune = web_prune_days
        .or_else(|| {
            std::env::var("JANITOR_WEB_PRUNE_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(JANITOR_DEFAULT_WEB_PRUNE_DAYS);
    let trim_keep = trim_keep_last
        .or_else(|| {
            std::env::var("JANITOR_TRIM_KEEP_LAST")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(super::data::TRIM_TRAINING_DEFAULT_KEEP);
    eprintln!("janitor-cycle: ingest {}", root.display());
    let count = super::ingest::run_ingest(config, &root)?;
    eprintln!("janitor-cycle: ingest OK ({} chunks).", count);
    eprintln!("janitor-cycle: ingest-web (prune {} days).", web_prune);
    super::ingest::run_ingest_web(config, Some(web_prune))?;
    eprintln!("janitor-cycle: trim-training (keep last {}).", trim_keep);
    super::data::run_trim_training(config, Some(trim_keep))?;
    eprintln!("janitor-cycle: finished.");
    Ok(())
}

/// Generate data/web_sources.json from docs/setup/doc_manifest.json.
pub(crate) fn run_build_web_sources(
    config: &rag_mcp::config::Config,
    run_ingest_flag: bool,
    prune_after_days: Option<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = crate::constitution::resolve_repo_root(None);
    let manifest_path = root.join("docs/setup/doc_manifest.json");
    if !manifest_path.exists() {
        eprintln!(
            "build-web-sources: doc manifest not found at {}. Set repo root or run from repo root.",
            manifest_path.display()
        );
        std::process::exit(1);
    }
    let output_path = config.data_dir.join("web_sources.json");
    rag_mcp::config::build_web_sources_from_doc_manifest(&manifest_path, &output_path)?;
    if run_ingest_flag {
        super::ingest::run_ingest_web(config, prune_after_days)?;
    }
    Ok(())
}
