//! Ingest pipeline: BLAKE3 manifest, tree-sitter chunking, symbol index. Parity with ingest_workspace.py.
//! Parallel ingest via JoinSet + spawn_blocking for AST + embed.
//! Eligible extensions: EXTENSIONS. Skip dirs: SKIP_DIRS. Write retry: WRITE_PREPARED_RETRY_ATTEMPTS; MIN_FREE_DISK_MB required before run.
//!
//! Manifest is path + config based (key = path::config_hash, value = file_content_hash).
//! Renamed/moved files are re-embedded; content-addressed dedup is not implemented (see docs/RAG_OPERATIONS.md).

use crate::rag::chunking::{chunk_file, Chunk};
use crate::rag::db::RagDb;
use crate::rag::embedding::RagEmbedder;
use crate::rag::path_filter;
use crate::rag::symbols::{self, SymbolExtraction};
use blake3::Hasher;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// One row for prepare_one_file chunk batch (owned defines/calls JSON).
type ChunkRowForBatch<'a> = (
    &'a str,
    &'a str,
    &'a str,
    String,
    &'a str,
    &'a str,
    &'a str,
    String,
    Option<&'a [f32]>,
);
/// Refs slice for db.upsert_chunks_batch (all &str + Option<&[f32]>).
type BatchChunkRefs<'a> = Vec<(
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    Option<&'a [f32]>,
)>;

/// File extensions eligible for ingest (code and docs). Only files with these extensions are considered; used in `should_ingest` and directory walks.
const EXTENSIONS: &[&str] = &[
    "py", "md", "txt", "json", "js", "ts", "jsx", "tsx", "rs", "ps1", "scala",
];
/// Directory names to skip when walking the tree (deps, build, cache).
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "chroma_db",
    "lancedb",
    "__pycache__",
    ".gemini",
    "build",
    "dist",
    "out",
    ".next",
    ".cache",
    ".doc",
    // "doc" removed so that "docs" (e.g. docs/lessons_learned.md) is ingestible
    "target",
    // Python virtualenv directories — never index installed packages
    ".venv",
    "venv",
    "env",
    "site-packages",
    ".tox",
    ".nox",
    // Other common noise
    "onnxruntime-win-x64-1.23.0",
    "ort_gpu",
    "cudnn9",
    "ZZOTHER",
    // Reference-only; RAG uses SQLite (see HUB.md, PIPELINE_VERIFICATION_CHECKLIST)
    "qdrant_storage",
];
/// Filenames to skip (e.g. tokenizer assets).
const SKIP_FILENAMES: &[&str] = &["tokenizer.json"];
/// Max file size to ingest (2 MiB); larger files are skipped. Resource safeguard to avoid OOM on large minified/binary files.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024; // 2 MiB
/// Alias for discoverability (Prism-style naming).
pub const MAX_FILE_SIZE_BYTES: u64 = MAX_FILE_BYTES;
/// Prefix for stub summaries when first-line summary is used (e.g. parse failure). Used in prepare_one_file.
const STUB_SUMMARY_PREFIX: &str = "Stub summary:";
/// Bumped when manifest/DB schema or chunk format changes; used in config_hash.
const SCHEMA_VERSION: &str = "1";
/// Minimum free disk space (MB) required before starting ingest to avoid mid-run disk full.
/// Minimum free disk space (MB) required on the volume containing the DB before ingest runs; checked at start of ingest_directory and ingest_directory_parallel.
const MIN_FREE_DISK_MB: u64 = 100;
/// Max chunks per file; beyond this we truncate with a warning. Resource safeguard to avoid OOM/hangs (Prism parity).
const MAX_CHUNKS_PER_FILE: usize = 2000;
/// Outer retry attempts for write_prepared_to_db on SQLITE_BUSY (complements db.rs per-op retry).
#[allow(dead_code)]
const WRITE_PREPARED_RETRY_ATTEMPTS: u32 = 3;
/// Backoff (ms) between retries in write_prepared_to_db on SQLITE_BUSY.
#[allow(dead_code)]
const WRITE_PREPARED_BACKOFF_MS: u64 = 150;
/// Max concurrent file-prepare tasks (spawn_blocking). Uses 2 * available_parallelism; caps parallelism for cache/throughput.
fn prepare_concurrency_limit() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get().saturating_mul(2))
        .unwrap_or(16)
        .max(1)
}

#[derive(Error, Debug)]
/// Errors from ingest: Io (file read), Db (RAG DB), Embed (embedder).
pub enum IngestError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("db: {0}")]
    Db(#[from] crate::rag::db::RagDbError),
    #[error("embed: {0}")]
    Embed(#[from] crate::rag::embedding::EmbedError),
}

/// Hash of schema version and pipeline tag; used as manifest key suffix so format changes invalidate cache.
fn config_hash() -> String {
    let mut h = Hasher::new();
    h.update(SCHEMA_VERSION.as_bytes());
    h.update(b"|rag-mcp|1");
    h.finalize().to_hex()[..16].to_string()
}
/// BLAKE3 hex hash of content; used for manifest and change detection.
fn file_content_hash(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}
/// True if path (canonicalized) is under any allowed root. Delegates to path_filter for consistency with store.
fn path_under_allowed(path: &Path, allowed: &[PathBuf]) -> bool {
    path_filter::path_under_allowed(path, allowed, false)
}

/// Check if any ancestor directory component exactly matches a SKIP_DIRS entry.
/// Build manifest key for path (canonical path display + config_hash). Used to skip unchanged files.
fn build_manifest_key(path: &Path) -> String {
    format!(
        "{}::{}",
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .display(),
        config_hash()
    )
}

/// True if any ancestor directory name is in SKIP_DIRS (e.g. target, node_modules).
fn path_has_skip_dir(path: &Path) -> bool {
    path.ancestors().any(|ancestor| {
        ancestor
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| SKIP_DIRS.contains(&n))
            .unwrap_or(false)
    })
}
/// True if path is a file, under allowed, not in skip dir, and has allowed extension/size.
fn should_ingest(path: &Path, allowed: &[PathBuf]) -> bool {
    if !path.is_file() {
        return false;
    }
    match path.metadata() {
        Ok(meta) if meta.len() > MAX_FILE_BYTES => {
            tracing::warn!(
                path = %path.display(),
                size_bytes = meta.len(),
                max_bytes = MAX_FILE_BYTES,
                "skipping file: exceeds MAX_FILE_BYTES limit"
            );
            return false;
        }
        Ok(_) => {}
        Err(_) => return false,
    }
    if path_has_skip_dir(path) {
        return false;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !EXTENSIONS.iter().any(|e| *e == ext) {
        return false;
    }
    if SKIP_FILENAMES.contains(&path.file_name().and_then(|n| n.to_str()).unwrap_or("")) {
        return false;
    }
    path_under_allowed(path, allowed)
}

/// Pre-flight disk space check. Fails if the volume containing `path` has less than `min_free_mb` MB free.
pub fn check_disk_space(path: &Path, min_free_mb: u64) -> Result<(), IngestError> {
    use sysinfo::Disks;
    let path_buf = path.canonicalize().unwrap_or_else(|_| {
        path.parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf())
    });
    let disks = Disks::new_with_refreshed_list();
    let min_free_bytes = min_free_mb.saturating_mul(1024 * 1024);
    let matching = disks
        .list()
        .iter()
        .filter(|d| path_buf.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len());
    match matching {
        Some(disk) => {
            if disk.available_space() < min_free_bytes {
                return Err(IngestError::Io(std::io::Error::new(
                    std::io::ErrorKind::StorageFull,
                    format!(
                        "insufficient disk space: {} MB free on {:?} (need at least {} MB for ingest)",
                        disk.available_space() / (1024 * 1024),
                        disk.mount_point(),
                        min_free_mb
                    ),
                )));
            }
            Ok(())
        }
        None => {
            tracing::warn!(
                "disk space check: path {:?} not on a known volume, proceeding",
                path
            );
            Ok(())
        }
    }
}

/// Truncate at the last valid UTF-8 char boundary at or before `max_bytes` to avoid panics on multi-byte characters.
fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    let end = max_bytes.min(s.len());
    let mut i = end;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    &s[..i]
}

pub(crate) fn stub_summary(content: &str) -> String {
    let first = content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("(no summary)");
    let s = first.trim();
    if s.len() > 120 {
        format!("{}...", &truncate_at_char_boundary(s, 117))
    } else {
        s.to_string()
    }
}

/// Chunk file content and build stable chunk IDs (source#0, source#1, ...). Used by ingest_single_file and prepare_one_file; single place for chunking + ID convention.
pub(crate) fn chunk_content_to_chunks_and_ids(
    content: &str,
    source: &str,
) -> (Vec<Chunk>, Vec<String>) {
    let chunks = chunk_file(content, source);
    let ids = (0..chunks.len())
        .map(|i| format!("{}#{}", source, i))
        .collect();
    (chunks, ids)
}

/// Ingest one file: delete old chunks, chunk, embed, upsert chunks and summary, update symbol index.
pub fn ingest_single_file(
    path: &Path,
    db: &RagDb,
    embedder: &RagEmbedder,
    allowed_roots: &[PathBuf],
    manifest: &mut HashMap<String, String>,
) -> Result<Option<u32>, IngestError> {
    if !should_ingest(path, allowed_roots) {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(IngestError::Io)?;
    let content = content.trim();
    if content.is_empty() {
        return Ok(None);
    }
    let file_hash = file_content_hash(content.as_bytes());
    let manifest_key = build_manifest_key(path);
    if manifest.get(&manifest_key) == Some(&file_hash) {
        return Ok(None);
    }
    let last_updated = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let syms = symbols::extract_symbols(content, &ext);
    let source = path.to_string_lossy().to_string();
    let (chunks, ids) = chunk_content_to_chunks_and_ids(content, &source);
    db.execute_write_batch(|conn| {
        RagDb::delete_chunks_by_source_conn(conn, &source)?;
        RagDb::delete_symbol_index_by_chunk_prefix_conn(conn, &source)?;
        RagDb::delete_reference_index_by_chunk_prefix_conn(conn, &source)?;
        RagDb::delete_summary_by_source_conn(conn, &source)?;
        Ok(())
    })?;

    let embeddings: Option<Vec<Vec<f32>>> = if embedder.is_available() {
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        match embedder.embed_batch(&texts) {
            Ok(vec) => Some(vec),
            Err(e) => {
                tracing::warn!(
                    source = %source,
                    "embedding batch failed: {}; storing {} chunks FTS-only",
                    e,
                    chunks.len()
                );
                None
            }
        }
    } else {
        None
    };
    let mut sym_pairs: Vec<(&str, &str)> = Vec::new();
    let mut ref_pairs: Vec<(&str, &str)> = Vec::new();
    let imports_json = syms.as_imports_json();
    for (i, c) in chunks.iter().enumerate() {
        let id = &ids[i];
        let emb = embeddings
            .as_ref()
            .and_then(|v| v.get(i))
            .map(|e| e.as_slice());
        let defines_json = serde_json::to_string(&c.defines).unwrap_or_else(|_| "[]".to_string());
        let calls_json = serde_json::to_string(&c.calls).unwrap_or_else(|_| "[]".to_string());
        db.upsert_chunk(
            id,
            &c.text,
            &source,
            "",
            &defines_json,
            &imports_json,
            &c.type_,
            &c.name,
            &calls_json,
            emb,
            last_updated,
            "code",     // chunk_type: codebase chunks
            "codebase", // source_type: from local files
        )?;
        for sym in &c.defines {
            sym_pairs.push((sym.as_str(), id.as_str()));
        }
        for sym in &c.calls {
            ref_pairs.push((sym.as_str(), id.as_str()));
        }
        for sym in &syms.imports {
            ref_pairs.push((sym.as_str(), id.as_str()));
        }
    }
    db.batch_insert_symbol_index(&sym_pairs)?;
    db.batch_insert_reference_index(&ref_pairs)?;
    let summary = stub_summary(content);
    let summary_text = if summary.starts_with(STUB_SUMMARY_PREFIX) {
        summary
    } else {
        format!("{} {}", STUB_SUMMARY_PREFIX, summary)
    };
    let summary_emb = if embedder.is_available() {
        embedder.embed(&summary_text).ok()
    } else {
        None
    };
    db.upsert_summary(&source, &summary_text, summary_emb.as_deref())?;
    manifest.insert(manifest_key, file_hash);
    Ok(Some(chunks.len() as u32))
}

/// Load manifest from path. Returns empty map if file missing, unreadable, or config_hash mismatch. Keys: path::config_hash, values: content hash.
/// Full manifest is kept in memory; for 100K+ files consider streaming or chunked loading.
pub fn load_manifest(manifest_path: &Path) -> HashMap<String, String> {
    let current = config_hash();
    let data = match fs::read_to_string(manifest_path) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    if parsed.get("config_hash").and_then(|c| c.as_str()) != Some(current.as_str()) {
        return HashMap::new();
    }
    let empty: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    let files = parsed
        .get("files")
        .and_then(|f| f.as_object())
        .unwrap_or(&empty);
    let mut out = HashMap::new();
    for (k, v) in files {
        if let Some(s) = v.as_str() {
            out.insert(k.clone(), s.to_string());
        }
    }
    out
}

/// Save manifest to path (config_hash + files map as JSON). Overwrites existing file.
pub fn save_manifest(
    manifest_path: &Path,
    manifest: &HashMap<String, String>,
) -> Result<(), std::io::Error> {
    let current = config_hash();
    let obj = serde_json::json!({ "config_hash": current, "files": manifest });
    fs::write(
        manifest_path,
        serde_json::to_string_pretty(&obj).unwrap_or_default(),
    )
}

/// Result of preparing one file for DB write (chunk + embed in parallel worker).
/// When embedder is unavailable or fails, chunk_embeddings and summary_embedding are None (FTS-only).
/// last_updated: file mtime (Unix secs) for RAG last_verified_date; None if metadata unavailable.
struct PreparedFile {
    manifest_key: String,
    file_hash: String,
    source: String,
    chunks: Vec<Chunk>,
    chunk_ids: Vec<String>,
    chunk_embeddings: Option<Vec<Vec<f32>>>,
    symbols: SymbolExtraction,
    summary_text: String,
    summary_embedding: Option<Vec<f32>>,
    last_updated: Option<u64>,
}

/// Prepare one file: read, BLAKE3 hash, skip if unchanged, chunk, symbols, embed. Runs in spawn_blocking.
/// Chunk one file, embed, and build PreparedFile; updates manifest with path+config_hash -> content_hash. Skips if path not under allowed or in skip dirs.
/// Chunk file, embed, return PreparedFile (chunk_ids, chunks, embeddings). Used by ingest_directory_parallel.
fn prepare_one_file(
    path: PathBuf,
    embedder: &RagEmbedder,
    allowed_roots: &[PathBuf],
    manifest: &RwLock<HashMap<String, String>>,
) -> Option<PreparedFile> {
    if !should_ingest(&path, allowed_roots) {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    let content = content.trim();
    if content.is_empty() {
        return None;
    }
    let file_hash = file_content_hash(content.as_bytes());
    let manifest_key = build_manifest_key(&path);
    if manifest
        .read()
        .ok()
        .and_then(|m| m.get(&manifest_key).cloned())
        == Some(file_hash.clone())
    {
        return None;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let symbols = symbols::extract_symbols(content, &ext);
    let source = path.to_string_lossy().to_string();
    let summary = stub_summary(content);
    let (mut chunks, mut chunk_ids) = chunk_content_to_chunks_and_ids(content, &source);

    if chunks.len() > MAX_CHUNKS_PER_FILE {
        tracing::warn!(
            "{} exceeds {} chunks ({} total). Truncating.",
            source,
            MAX_CHUNKS_PER_FILE,
            chunks.len()
        );
        chunks.truncate(MAX_CHUNKS_PER_FILE);
        chunk_ids.truncate(MAX_CHUNKS_PER_FILE);
    }

    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    let chunk_embeddings = if embedder.is_available() {
        embedder.embed_batch(&texts).ok()
    } else {
        None
    };
    let summary_text = if summary.starts_with(STUB_SUMMARY_PREFIX) {
        summary
    } else {
        format!("{} {}", STUB_SUMMARY_PREFIX, summary)
    };
    let summary_embedding = if embedder.is_available() {
        embedder.embed(&summary_text).ok()
    } else {
        None
    };
    let last_updated = fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    Some(PreparedFile {
        manifest_key,
        file_hash,
        source,
        chunks,
        chunk_ids,
        chunk_embeddings,
        symbols,
        summary_text,
        summary_embedding,
        last_updated,
    })
}
/// Writes prepared chunks and summary to DB; deletes old chunks/symbol/ref index for source first.
/// Retained for backward compatibility (non-parallel ingest paths).
#[allow(dead_code)]
fn write_prepared_to_db(db: &RagDb, p: &PreparedFile) -> Result<(), IngestError> {
    db.delete_chunks_by_source(&p.source)?;
    db.delete_symbol_index_by_chunk_prefix(&p.source)?;
    db.delete_reference_index_by_chunk_prefix(&p.source)?;
    db.delete_summary_by_source(&p.source)?;

    let imports_json = p.symbols.as_imports_json();
    let chunk_rows: Vec<ChunkRowForBatch<'_>> = p
        .chunks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let id = p.chunk_ids[i].as_str();
            let defines_json =
                serde_json::to_string(&c.defines).unwrap_or_else(|_| "[]".to_string());
            let calls_json = serde_json::to_string(&c.calls).unwrap_or_else(|_| "[]".to_string());
            let emb = p
                .chunk_embeddings
                .as_ref()
                .and_then(|v| v.get(i))
                .map(|e| e.as_slice());
            (
                id,
                c.text.as_str(),
                p.source.as_str(),
                defines_json,
                imports_json.as_str(),
                c.type_.as_str(),
                c.name.as_str(),
                calls_json,
                emb,
            )
        })
        .collect();
    // Build owned refs for batch: (id, text, source, defines, imports, type_, name, calls, emb).
    let batch_refs: BatchChunkRefs<'_> = chunk_rows
        .iter()
        .map(
            |(id, text, source, defines, imports, type_, name, calls, emb)| {
                (
                    *id,
                    *text,
                    *source,
                    defines.as_str(),
                    *imports,
                    *type_,
                    *name,
                    calls.as_str(),
                    *emb,
                )
            },
        )
        .collect();
    db.upsert_chunks_batch(&batch_refs, "code", "codebase", p.last_updated)?;

    let mut symbol_pairs: Vec<(&str, &str)> = Vec::new();
    let mut ref_pairs: Vec<(&str, &str)> = Vec::new();
    for (i, c) in p.chunks.iter().enumerate() {
        let id = p.chunk_ids[i].as_str();
        for sym in &c.defines {
            symbol_pairs.push((sym.as_str(), id));
        }
        for sym in &c.calls {
            ref_pairs.push((sym.as_str(), id));
        }
        for sym in &p.symbols.imports {
            ref_pairs.push((sym.as_str(), id));
        }
    }
    db.batch_insert_symbol_index(&symbol_pairs)?;
    db.batch_insert_reference_index(&ref_pairs)?;

    db.upsert_summary(&p.source, &p.summary_text, p.summary_embedding.as_deref())?;
    Ok(())
}

/// Writes one prepared file to DB using a raw connection (no mutex lock).
/// Used inside `execute_write_batch` for transactional batch writes.
fn write_prepared_to_db_conn(
    conn: &rusqlite::Connection,
    p: &PreparedFile,
) -> Result<(), crate::rag::db::RagDbError> {
    RagDb::delete_chunks_by_source_conn(conn, &p.source)?;
    RagDb::delete_symbol_index_by_chunk_prefix_conn(conn, &p.source)?;
    RagDb::delete_reference_index_by_chunk_prefix_conn(conn, &p.source)?;
    RagDb::delete_summary_by_source_conn(conn, &p.source)?;

    let imports_json = p.symbols.as_imports_json();
    let chunk_rows: Vec<ChunkRowForBatch<'_>> = p
        .chunks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let id = p.chunk_ids[i].as_str();
            let defines_json =
                serde_json::to_string(&c.defines).unwrap_or_else(|_| "[]".to_string());
            let calls_json = serde_json::to_string(&c.calls).unwrap_or_else(|_| "[]".to_string());
            let emb = p
                .chunk_embeddings
                .as_ref()
                .and_then(|v| v.get(i))
                .map(|e| e.as_slice());
            (
                id,
                c.text.as_str(),
                p.source.as_str(),
                defines_json,
                imports_json.as_str(),
                c.type_.as_str(),
                c.name.as_str(),
                calls_json,
                emb,
            )
        })
        .collect();
    let batch_refs: BatchChunkRefs<'_> = chunk_rows
        .iter()
        .map(
            |(id, text, source, defines, imports, type_, name, calls, emb)| {
                (
                    *id,
                    *text,
                    *source,
                    defines.as_str(),
                    *imports,
                    *type_,
                    *name,
                    calls.as_str(),
                    *emb,
                )
            },
        )
        .collect();
    RagDb::upsert_chunks_batch_conn(conn, &batch_refs, "code", "codebase", p.last_updated)?;

    let mut symbol_pairs: Vec<(&str, &str)> = Vec::new();
    let mut ref_pairs: Vec<(&str, &str)> = Vec::new();
    for (i, c) in p.chunks.iter().enumerate() {
        let id = p.chunk_ids[i].as_str();
        for sym in &c.defines {
            symbol_pairs.push((sym.as_str(), id));
        }
        for sym in &c.calls {
            ref_pairs.push((sym.as_str(), id));
        }
        for sym in &p.symbols.imports {
            ref_pairs.push((sym.as_str(), id));
        }
    }
    RagDb::batch_insert_symbol_index_conn(conn, &symbol_pairs)?;
    RagDb::batch_insert_reference_index_conn(conn, &ref_pairs)?;

    RagDb::upsert_summary_conn(conn, &p.source, &p.summary_text, p.summary_embedding.as_deref())?;
    Ok(())
}

/// Batch size for transactional writes in parallel ingest.
const WRITE_BATCH_SIZE: usize = 50;

/// Ingest directory in parallel: JoinSet + spawn_blocking per file, BLAKE3 skip, sequential DB write.
pub async fn ingest_directory_parallel(
    root: &Path,
    db: Arc<RagDb>,
    embedder: Arc<RagEmbedder>,
    allowed_roots: Vec<PathBuf>,
    manifest_path: PathBuf,
) -> Result<u32, IngestError> {
    check_disk_space(db.db_path(), MIN_FREE_DISK_MB)?;
    let manifest = Arc::new(RwLock::new(load_manifest(&manifest_path)));
    let root_abs = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let paths: Vec<PathBuf> = walkdir::WalkDir::new(&root_abs)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| SKIP_DIRS.contains(&n))
                .unwrap_or(false)
        })
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .filter(|e| {
            let name = e.path().file_name().and_then(|n| n.to_str()).unwrap_or("");
            !name.starts_with('.') && !SKIP_FILENAMES.contains(&name)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let sem = Arc::new(tokio::sync::Semaphore::new(prepare_concurrency_limit()));
    let mut joins = tokio::task::JoinSet::new();
    for path in paths {
        let permit = sem.clone().acquire_owned().await.map_err(|_| {
            IngestError::Io(std::io::Error::other("ingest prepare semaphore closed"))
        })?;
        let embedder = Arc::clone(&embedder);
        let allowed = allowed_roots.clone();
        let manifest = Arc::clone(&manifest);
        joins.spawn(async move {
            let _permit = permit;
            tokio::task::spawn_blocking(move || {
                prepare_one_file(path, &embedder, &allowed, &manifest)
            })
            .await
            .ok()
            .flatten()
        });
    }

    let mut prepared = Vec::new();
    while let Some(res) = joins.join_next().await {
        if let Ok(Some(p)) = res {
            prepared.push(p);
        }
    }

    let mut count = 0u32;
    for batch in prepared.chunks(WRITE_BATCH_SIZE) {
        let batch_result = db.execute_write_batch(|conn| {
            for p in batch {
                write_prepared_to_db_conn(conn, p)?;
            }
            Ok(())
        });

        match batch_result {
            Ok(()) => {
                for p in batch {
                    count += p.chunks.len() as u32;
                    if let Ok(mut m) = manifest.write() {
                        m.insert(p.manifest_key.clone(), p.file_hash.clone());
                    }
                }
            }
            Err(e) => return Err(e.into()),
        }
    }

    let to_remove: Vec<String> = manifest
        .read()
        .map(|m| {
            m.keys()
                .filter(|k| {
                    let path_part = k.split("::").next().unwrap_or("");
                    !Path::new(path_part).exists()
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    for k in &to_remove {
        if let Some(path) = k.split("::").next() {
            let _ = db.delete_chunks_by_source(path);
        }
        if let Ok(mut m) = manifest.write() {
            m.remove(k);
        }
    }
    let m = manifest
        .read()
        .map_err(|_| IngestError::Io(std::io::Error::other("manifest lock")))?;
    save_manifest(&manifest_path, &m)?;
    let _ = db.wal_checkpoint_passive();
    Ok(count)
}

/// Ingest a directory: walk (EXTENSIONS/SKIP_DIRS), filter, ingest_single_file, prune ghosts from manifest, save manifest.
pub fn ingest_directory(
    root: &Path,
    db: Arc<RagDb>,
    embedder: Arc<RagEmbedder>,
    allowed_roots: Vec<PathBuf>,
    manifest_path: PathBuf,
) -> Result<u32, IngestError> {
    check_disk_space(db.db_path(), MIN_FREE_DISK_MB)?;
    let mut manifest = load_manifest(&manifest_path);
    let mut count = 0u32;
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            !e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| SKIP_DIRS.contains(&n))
                .unwrap_or(false)
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || SKIP_FILENAMES.contains(&name) {
            continue;
        }
        if let Some(n) = ingest_single_file(path, &db, &embedder, &allowed_roots, &mut manifest)? {
            count += n;
        }
    }
    let to_remove: Vec<String> = manifest
        .keys()
        .filter(|k| {
            let path_part = k.split("::").next().unwrap_or("");
            let p = Path::new(path_part);
            !p.exists()
        })
        .cloned()
        .collect();
    for k in &to_remove {
        if let Some(path) = k.split("::").next() {
            let _ = db.delete_chunks_by_source(path);
        }
        manifest.remove(k);
    }
    save_manifest(&manifest_path, &manifest)?;
    let _ = db.wal_checkpoint_passive();
    Ok(count)
}

/// Ingest from a JSONL file: each line is `{"path": "source-id", "text": "..."}` (or "source"/"content"),
/// or `{"path": "source-id", "chunks": ["section1", "section2", ...]}` for multi-chunk documents (e.g. Godly RAG master_research.md).
/// Used to merge external sources (e.g. NotebookLM export) into the same RAG index.
/// Deletes existing chunks for each source, then inserts one or more chunks with optional embedding.
/// Resource safeguard: individual text/chunk longer than MAX_FILE_BYTES is skipped to avoid OOM.
pub fn ingest_from_jsonl(
    jsonl_path: &Path,
    db: &RagDb,
    embedder: &RagEmbedder,
) -> Result<u32, IngestError> {
    let content = fs::read_to_string(jsonl_path)?;
    let mut count = 0u32;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let obj: serde_json::Value = serde_json::from_str(line).map_err(|e| {
            IngestError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid JSONL line: {}", e),
            ))
        })?;
        let source = obj
            .get("path")
            .or_else(|| obj.get("source"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Multi-chunk line: {"path": "...", "chunks": ["...", "..."]}
        let chunks_array = obj.get("chunks").and_then(|v| v.as_array());
        let texts: Vec<String> = if let Some(arr) = chunks_array {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            // Single-chunk line: {"path": "...", "text": "..."} or "content"
            let text = obj
                .get("text")
                .or_else(|| obj.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                continue;
            }
            vec![text]
        };

        if texts.is_empty() {
            continue;
        }

        db.delete_chunks_by_source(&source)?;

        let last_updated = if source.starts_with("https://") {
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            )
        } else {
            None
        };

        for (idx, text) in texts.iter().enumerate() {
            if text.len() > MAX_FILE_BYTES as usize {
                tracing::warn!(
                    "ingest_from_jsonl: skipping source {} chunk {} (length {} exceeds {} bytes)",
                    source,
                    idx,
                    text.len(),
                    MAX_FILE_BYTES
                );
                continue;
            }
            let chunk_id = format!("{}#{}", source, idx);
            let summary = if text.len() > 500 {
                format!("{}...", truncate_at_char_boundary(text, 497))
            } else {
                text.clone()
            };
            let embedding = embedder.embed(text).ok();
            let summary_embed = if idx == 0 {
                embedder.embed(&summary).ok()
            } else {
                None
            };
            db.upsert_chunk(
                &chunk_id,
                text,
                &source,
                &summary,
                "[]",
                "[]",
                "text",
                "doc",
                "[]",
                embedding.as_deref(),
                last_updated,
                "summary",
                "external",
            )?;
            if idx == 0 {
                db.upsert_summary(&source, &summary, summary_embed.as_deref())?;
            }
            count += 1;
        }
    }
    let _ = db.wal_checkpoint_passive();
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::embedding::RagEmbedder;

    #[test]
    /// chunk_content_to_chunks_and_ids returns ids source#0, source#1, ... and len matches chunks.
    fn chunk_content_to_chunks_and_ids_returns_stable_ids() {
        let content = "def f(): pass";
        let source = "src/a.py";
        let (chunks, ids) = chunk_content_to_chunks_and_ids(content, source);
        assert_eq!(chunks.len(), ids.len());
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(id, &format!("{}#{}", source, i));
        }
    }

    #[test]
    /// Ingest of .md with headings then get_chunks_by_source returns section chunks with stable ids.
    fn ingest_single_file_md_with_headings_then_get_chunks_by_source_has_section_type() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let db_path = temp_dir.path().join("rag.db");
        let db = RagDb::open(&db_path).expect("open db");
        let embedder = RagEmbedder::stub();
        let allowed_root = temp_dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_dir.path().to_path_buf());
        let allowed = vec![allowed_root];

        let doc_md = temp_dir.path().join("docs").join("guide.md");
        std::fs::create_dir_all(doc_md.parent().unwrap()).expect("create dir");
        std::fs::write(
            &doc_md,
            "# Installation\n\nInstall here.\n\n## Config\n\nSet FOO=1.",
        )
        .expect("write md");

        let mut manifest = HashMap::new();
        ingest_single_file(&doc_md, &db, &embedder, &allowed, &mut manifest).expect("ingest");
        let source = doc_md.to_string_lossy().to_string();
        let rows = db
            .get_chunks_by_source(&source)
            .expect("get_chunks_by_source");
        assert!(!rows.is_empty());
        let section_count = rows.iter().filter(|r| r.type_ == "section").count();
        assert!(
            section_count >= 1,
            "expected at least one section chunk, got {:?}",
            rows.iter().map(|r| &r.type_).collect::<Vec<_>>()
        );
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.id, format!("{}#{}", source, i));
        }
    }

    #[test]
    /// Ingest of def + caller populates symbol_index (definition) and reference_index (caller).
    fn ingest_populates_symbol_index_definitions_only_and_reference_index() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let db_path = temp_dir.path().join("rag.db");
        let db = RagDb::open(&db_path).expect("open db");
        let embedder = RagEmbedder::stub();
        let allowed_root = temp_dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_dir.path().to_path_buf());
        let allowed = vec![allowed_root];

        let def_py = temp_dir.path().join("def_module.py");
        std::fs::write(
            &def_py,
            r#"
def helper():
    return 42
"#,
        )
        .expect("write def");
        let caller_py = temp_dir.path().join("caller.py");
        std::fs::write(
            &caller_py,
            r#"
from def_module import helper

def main():
    x = helper()
    print(x)
"#,
        )
        .expect("write caller");

        let mut manifest = HashMap::new();
        ingest_single_file(&def_py, &db, &embedder, &allowed, &mut manifest).expect("ingest def");
        ingest_single_file(&caller_py, &db, &embedder, &allowed, &mut manifest)
            .expect("ingest caller");

        let def_ids = db.get_chunk_ids_for_symbol("helper").expect("get def");
        let ref_ids = db
            .get_chunk_ids_referencing_symbol("helper")
            .expect("get ref");
        assert!(
            !def_ids.is_empty() && def_ids.iter().any(|id| id.contains("def_module")),
            "symbol_index should contain defining chunk for 'helper', got {:?}",
            def_ids
        );
        let ref_ok = ref_ids.iter().any(|id| id.contains("caller"));
        assert!(
            ref_ok,
            "reference_index should contain caller chunk, got {:?}",
            ref_ids
        );
    }

    #[test]
    /// Stub summary trims and returns first line.
    fn stub_summary_short_line_returns_trimmed_first_line() {
        let out = stub_summary("  hello world  \nsecond");
        assert_eq!(out, "hello world");
    }

    #[test]
    /// Stub summary truncates long first line at 120 chars with ellipsis.
    fn stub_summary_long_line_truncates_with_ellipsis() {
        let long = "a".repeat(150);
        let out = stub_summary(&long);
        assert!(out.len() == 120 && out.ends_with("..."));
    }

    #[test]
    /// load_manifest on missing path returns empty map.
    fn load_manifest_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("no_manifest.json");
        assert!(load_manifest(&path).is_empty());
    }

    #[test]
    /// save_manifest then load_manifest roundtrip preserves entries.
    fn load_manifest_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("manifest.json");
        let mut m = HashMap::new();
        m.insert("a::b".to_string(), "hash1".to_string());
        m.insert("c::d".to_string(), "hash2".to_string());
        save_manifest(&path, &m).expect("save");
        let loaded = load_manifest(&path);
        assert_eq!(loaded.get("a::b"), Some(&"hash1".to_string()));
        assert_eq!(loaded.get("c::d"), Some(&"hash2".to_string()));
        assert_eq!(loaded.len(), 2);
    }

    /// A4 resource safeguard: ingest_from_jsonl skips lines with text > MAX_FILE_BYTES to avoid OOM.
    #[test]
    /// Ingest from JSONL skips rows whose text exceeds max chunk size.
    fn ingest_from_jsonl_skips_oversized_text() {
        let tmp = tempfile::tempdir().unwrap();
        let jsonl = tmp.path().join("huge.jsonl");
        let big_text = "x".repeat((MAX_FILE_BYTES as usize) + 1);
        let line = serde_json::json!({ "path": "huge-source", "text": big_text });
        std::fs::write(&jsonl, format!("{}\n", line)).unwrap();
        let db_path = tmp.path().join("rag.db");
        let db = RagDb::open(&db_path).expect("open db");
        let embedder = RagEmbedder::stub();
        let count = ingest_from_jsonl(&jsonl, &db, &embedder).expect("ingest");
        assert_eq!(count, 0, "oversized line must be skipped");
    }

    #[test]
    /// Ingest from JSONL with "chunks" array creates path#0, path#1, ... with same source (Godly RAG).
    fn ingest_from_jsonl_multi_chunk_line() {
        let tmp = tempfile::tempdir().unwrap();
        let jsonl = tmp.path().join("multi.jsonl");
        let line = serde_json::json!({
            "path": "research/master_research.md",
            "chunks": ["## Section A\nContent A.", "## Section B\nContent B.", "## Section C\nContent C."]
        });
        std::fs::write(&jsonl, format!("{}\n", line)).unwrap();
        let db_path = tmp.path().join("rag.db");
        let db = RagDb::open(&db_path).expect("open db");
        let embedder = RagEmbedder::stub();
        let count = ingest_from_jsonl(&jsonl, &db, &embedder).expect("ingest");
        assert_eq!(count, 3);
        let ids = vec![
            "research/master_research.md#0".to_string(),
            "research/master_research.md#1".to_string(),
            "research/master_research.md#2".to_string(),
        ];
        let rows = db.get_chunks_by_ids(&ids).expect("get_chunks_by_ids");
        assert_eq!(rows.len(), 3);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.source, "research/master_research.md");
            assert!(row
                .text
                .contains(&format!("Section {}", (b'A' + i as u8) as char)));
        }
    }
}
