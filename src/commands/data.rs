use std::collections::HashSet;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use rag_mcp::rag::{
    ingest::{load_manifest, save_manifest},
    is_low_value_training_row, sanitize_shell_output, DatasetCollector, RagDb,
};

/// One line of training.jsonl: query, context, response, domain, ts (matches DatasetCollector output).
#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct TrainingLine {
    pub query: String,
    pub context: String,
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub ts: u64,
}

/// Default number of lines to keep when trimming training.jsonl.
pub(crate) const TRIM_TRAINING_DEFAULT_KEEP: u64 = 100_000;

/// Read data_dir/training.jsonl, deduplicate by (query, context), sanitize, write to output path or stdout.
/// When strip_low_value is true, drops rows that would be skipped at write time (run N, ok, no code change, etc.).
pub(crate) fn run_bin_to_jsonl(
    config: &rag_mcp::config::Config,
    output_path: Option<&Path>,
    strip_low_value: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input_path = config
        .golden_set_dir
        .as_ref()
        .map(|d| d.join("training.jsonl"))
        .unwrap_or_else(|| DatasetCollector::default_path(&config.data_dir.to_string_lossy()));
    if !input_path.exists() {
        eprintln!(
            "bin-to-jsonl: input file not found: {}",
            input_path.display()
        );
        std::process::exit(1);
    }
    let f = std::fs::File::open(&input_path)?;
    let reader = BufReader::new(f);
    let mut seen: HashSet<u64> = HashSet::new();
    let mut out: Box<dyn Write> = match output_path {
        Some(p) => {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Box::new(BufWriter::new(std::fs::File::create(p)?))
        }
        None => Box::new(std::io::stdout()),
    };
    let mut skipped_low_value = 0u64;
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let row: TrainingLine = match serde_json::from_str(line) {
            Ok(r) => r,
            _ => continue,
        };
        let query = row.query.trim().to_string();
        let context = row.context.trim().to_string();
        if query.is_empty() && context.is_empty() {
            continue;
        }
        if row.response.trim().is_empty() {
            continue;
        }
        if strip_low_value && is_low_value_training_row(&query, &row.response) {
            skipped_low_value += 1;
            continue;
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (&query, &context).hash(&mut hasher);
        let key = hasher.finish();
        if !seen.insert(key) {
            continue;
        }
        let out_line = TrainingLine {
            query: sanitize_shell_output(&query),
            context: sanitize_shell_output(&context),
            response: sanitize_shell_output(&row.response),
            domain: row.domain,
            ts: row.ts,
        };
        let json = serde_json::to_string(&out_line)?;
        writeln!(out, "{}", json)?;
    }
    if strip_low_value && skipped_low_value > 0 {
        eprintln!(
            "bin-to-jsonl: stripped {} low-value rows",
            skipped_low_value
        );
    }
    if let Some(p) = output_path {
        eprintln!(
            "bin-to-jsonl: wrote {} lines to {}",
            seen.len(),
            p.display()
        );
    }
    Ok(())
}

/// Keep only the last N lines of training.jsonl. Writes to a temp file then replaces the original.
/// Memory: reads the entire file into memory (bounded by `keep_last`, default 100k lines); acceptable for typical trim-training usage.
pub(crate) fn run_trim_training(
    config: &rag_mcp::config::Config,
    keep_last: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let n = keep_last.unwrap_or(TRIM_TRAINING_DEFAULT_KEEP);
    let input_path = config
        .golden_set_dir
        .as_ref()
        .map(|d| d.join("training.jsonl"))
        .unwrap_or_else(|| DatasetCollector::default_path(&config.data_dir.to_string_lossy()));
    if !input_path.exists() {
        eprintln!("trim-training: {} not found.", input_path.display());
        std::process::exit(1);
    }
    let f = std::fs::File::open(&input_path)?;
    let reader = BufReader::new(f);
    let lines: Vec<String> = reader.lines().collect::<Result<Vec<_>, _>>()?;
    let total = lines.len();
    if total <= n as usize {
        eprintln!("trim-training: {} lines (<= {}), no trim needed.", total, n);
        return Ok(());
    }
    let to_keep = lines
        .into_iter()
        .skip(total - n as usize)
        .collect::<Vec<_>>();
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp_path = parent.to_path_buf();
    temp_path.push(".training.jsonl.trim.tmp");
    {
        let mut out = BufWriter::new(std::fs::File::create(&temp_path)?);
        for line in &to_keep {
            writeln!(out, "{}", line.trim_end())?;
        }
        out.flush()?;
    }
    std::fs::rename(&temp_path, &input_path)?;
    eprintln!(
        "trim-training: kept last {} of {} lines in {}",
        to_keep.len(),
        total,
        input_path.display()
    );
    Ok(())
}

/// Remove RAG DB chunks and manifest entries for workspace paths that no longer exist on disk.
pub(crate) fn run_prune_orphans(
    config: &rag_mcp::config::Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manifest_path = config.data_dir.join("rag_manifest.json");
    if !manifest_path.exists() {
        eprintln!(
            "prune-orphans: manifest not found at {}. Nothing to prune.",
            manifest_path.display()
        );
        return Ok(());
    }
    let mut manifest = load_manifest(&manifest_path);
    let to_remove: Vec<String> = manifest
        .keys()
        .filter(|k| {
            let path_part = k.split("::").next().unwrap_or("");
            let check_path = path_part.strip_prefix("\\\\?\\").unwrap_or(path_part);
            !Path::new(check_path).exists()
        })
        .cloned()
        .collect();
    if to_remove.is_empty() {
        eprintln!(
            "prune-orphans: no stale entries (all {} paths exist).",
            manifest.len()
        );
        return Ok(());
    }
    let rag_db = RagDb::open(&config.db_path)?;
    for k in &to_remove {
        if let Some(path) = k.split("::").next() {
            let _ = rag_db.delete_chunks_by_source(path);
            let _ = rag_db.delete_symbol_index_by_chunk_prefix(path);
            let _ = rag_db.delete_reference_index_by_chunk_prefix(path);
        }
        manifest.remove(k);
    }
    save_manifest(&manifest_path, &manifest)?;
    // Also clean up vector entries whose workspace_chunks rows were removed above or by
    // external tooling (e.g. Python-based bulk deletes that bypass delete_chunks_by_source).
    if let Ok(n) = rag_db.prune_orphaned_chunk_vectors() {
        if n > 0 {
            eprintln!("prune-orphans: removed {} orphaned chunk_vectors rows.", n);
        }
    }
    rag_db.wal_checkpoint_passive_retry();
    eprintln!(
        "prune-orphans: removed {} stale entries; {} remaining.",
        to_remove.len(),
        manifest.len()
    );
    Ok(())
}

/// Prune manifest entries for files that no longer exist on disk. Updates only rag_manifest.json (does not touch DB).
pub(crate) fn run_prune_manifest_stale(
    config: &rag_mcp::config::Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manifest_path = config.data_dir.join("rag_manifest.json");
    if !manifest_path.exists() {
        eprintln!(
            "prune-manifest-stale: manifest not found at {}.",
            manifest_path.display()
        );
        return Ok(());
    }
    let manifest = load_manifest(&manifest_path);
    if manifest.is_empty() {
        eprintln!("prune-manifest-stale: no files in manifest (or config_hash mismatch).");
        return Ok(());
    }
    let mut kept: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut removed = 0usize;
    for (key, hash_val) in &manifest {
        let path_part = key.split("::").next().unwrap_or("");
        let check_path = path_part.strip_prefix("\\\\?\\").unwrap_or(path_part);
        if Path::new(check_path).exists() {
            kept.insert(key.clone(), hash_val.clone());
        } else {
            removed += 1;
        }
    }
    save_manifest(&manifest_path, &kept)?;
    eprintln!(
        "prune-manifest-stale: pruned {} stale entries; {} remaining.",
        removed,
        kept.len()
    );
    Ok(())
}

/// Full data clean: training clean, prune-orphans, prune-manifest-stale, ingest-web, trim-training, optional review-lessons.
pub(crate) fn run_data_clean(
    config: &rag_mcp::config::Config,
    web_prune_days: Option<u32>,
    trim_keep_last: Option<u64>,
    run_review_lessons_flag: bool,
    skip_training_clean: bool,
    intensive: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use super::ops::JANITOR_DEFAULT_WEB_PRUNE_DAYS;

    let (web_prune, trim_keep, review) = if intensive {
        (
            web_prune_days.unwrap_or(7),
            trim_keep_last.unwrap_or(50_000),
            true,
        )
    } else {
        (
            web_prune_days.unwrap_or(JANITOR_DEFAULT_WEB_PRUNE_DAYS),
            trim_keep_last.unwrap_or(TRIM_TRAINING_DEFAULT_KEEP),
            run_review_lessons_flag,
        )
    };
    if !skip_training_clean {
        let training_path = config
            .golden_set_dir
            .as_ref()
            .map(|d| d.join("training.jsonl"))
            .unwrap_or_else(|| DatasetCollector::default_path(&config.data_dir.to_string_lossy()));
        if training_path.exists() {
            let parent = training_path.parent().unwrap_or_else(|| Path::new("."));
            let cleaned = parent.join("training_cleaned.jsonl");
            let _ = run_bin_to_jsonl(config, Some(cleaned.as_path()), true);
            if cleaned.exists() {
                let backup = parent.join("training.jsonl.bak");
                let _ = std::fs::copy(&training_path, &backup);
                let _ = std::fs::rename(&cleaned, &training_path);
                eprintln!("data-clean: Step 1 OK (training cleaned).");
            }
        }
    }
    eprintln!("data-clean: Step 2 prune-orphans.");
    let _ = run_prune_orphans(config);
    eprintln!("data-clean: Step 3 prune-manifest-stale.");
    let _ = run_prune_manifest_stale(config);
    eprintln!("data-clean: Step 4 ingest-web (prune {} days).", web_prune);
    let _ = super::ingest::run_ingest_web(config, Some(web_prune));
    eprintln!(
        "data-clean: Step 4b trim-training (keep last {}).",
        trim_keep
    );
    let _ = run_trim_training(config, Some(trim_keep));
    if review {
        eprintln!("data-clean: Step 5 review-lessons.");
        let _ = super::audit::run_review_lessons(config);
    }
    eprintln!("data-clean: finished.");
    Ok(())
}
