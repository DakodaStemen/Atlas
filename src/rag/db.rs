//! RAG SQLite schema: workspace_chunks (metadata) + vec0 for 768-d vectors,
//! file_summaries + vec0, symbol_index, FTS5 for hybrid search.
//! Uses sqlite-vec extension (registered at open). All vector dims are 768 (Nomic).
//! **Schema:** workspace_chunks (id, text, source, embedding in vec0) + FTS5 for keyword search;
//! file_summaries for hierarchical retrieval; symbol_index (definitions) and reference_index (call sites) for get_related_code. Hybrid search = FTS + vector + RRF fusion.
//! get_chunks_by_ids: fetch ChunkRows by id list for store hybrid/hierarchical retrieval.

use rusqlite::Connection;
use rusqlite::ErrorCode;
use std::path::Path;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Duration;
use thiserror::Error;

/// Guards sqlite3_auto_extension registration so it runs exactly once per process (C-3).
/// Calling sqlite3_auto_extension multiple times for the same function registers it multiple
/// times, causing the extension init to run on every new connection unnecessarily.
static SQLITE_VEC_REGISTERED: OnceLock<()> = OnceLock::new();

/// Vector dimension for all embeddings (Nomic). Must match sqlite-vec vec0 columns and embedder output.
pub const RAG_EMBED_DIM: usize = 768;
/// Reciprocal Rank Fusion constant (score = 1 / (rank + RRF_K)). Typical value is 60; we use 20 to weight top ranks more heavily so the first few results dominate the fused score.
const RRF_K: f64 = 20.0;

/// Bytes per embedding dimension (f32 = 4). Used for blob length check.
const BYTES_PER_DIM: usize = std::mem::size_of::<f32>();
/// Serialize embedding slice to little-endian blob for chunk_vectors table.
fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize little-endian blob to embedding (for semantic_cache). Fails if length != 768*4.
fn blob_to_embedding(bytes: &[u8]) -> Result<Vec<f32>, RagDbError> {
    if bytes.len() != RAG_EMBED_DIM * BYTES_PER_DIM {
        return Err(RagDbError::EmbedDim(bytes.len() / BYTES_PER_DIM));
    }
    let mut out = Vec::with_capacity(RAG_EMBED_DIM);
    for chunk in bytes.chunks_exact(BYTES_PER_DIM) {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(chunk);
        out.push(f32::from_le_bytes(buf));
    }
    Ok(out)
}

/// Max write retries on SQLITE_BUSY.
const WRITE_RETRY_MAX_ATTEMPTS: u32 = 5;
/// Initial backoff ms for write retries; doubled each attempt up to WRITE_RETRY_CAP_MS.
const WRITE_RETRY_BASE_MS: u64 = 50;
/// Max backoff ms (50, 100, 200, 400).
const WRITE_RETRY_CAP_MS: u64 = 400;

/// Backoff delay in ms for write retry attempt (1-based). Formula: WRITE_RETRY_BASE_MS * 2^(attempt-1), capped at WRITE_RETRY_CAP_MS.
fn write_retry_backoff_ms(attempt: u32) -> u64 {
    std::cmp::min(
        WRITE_RETRY_BASE_MS * (1 << (attempt.saturating_sub(1))),
        WRITE_RETRY_CAP_MS,
    )
}

/// One row from semantic_cache_fetch_recent: (query_text, response_text, embedding, created_at).
type SemanticCacheRow = (String, String, Vec<f32>, i64);

/// One row for upsert_chunks_batch: (id, text, source, defines, imports, type_, name, calls_metadata, embedding).
type ChunkBatchRowRef<'a> = (
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    &'a str,
    Option<&'a [f32]>,
);

#[derive(Error, Debug)]
/// RagDbError.
pub enum RagDbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("embedding dimension mismatch: got {0} floats (expected 768). Ensure ORT_DYLIB_PATH is set when running ingest and serve; delete data/rag.db and re-run ingest if the index was built without embeddings.")]
    EmbedDim(usize),
    #[error("embedding failed: {0}")]
    EmbedFailed(String),
    #[error("disk full or I/O: {0}")]
    DiskFull(String),
    #[error("mutex poisoned: {0}")]
    Poisoned(String),
    #[error("parallel search thread panicked or failed to join")]
    SearchWorker,
}

/// True if the error message indicates an embedding dimension mismatch. Used by map_sqlite_vec_error to map to RagDbError::EmbedDim.
pub(crate) fn error_message_indicates_embed_dim(msg: &str) -> bool {
    msg.contains("expected 0")
        || msg.to_lowercase().contains("embedding dimension")
        || msg.contains("dimension mismatch")
}

/// Escapes `%`, `_`, and `\` for use in SQLite `LIKE` with `ESCAPE '\'`.
pub(crate) fn escape_sqlite_like_literal(src: &str) -> String {
    let mut out = String::with_capacity(src.len().saturating_add(8));
    for ch in src.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            _ => out.push(ch),
        }
    }
    out
}

/// Prepare arbitrary user text for FTS5 `MATCH`. Raw strings like `error: smoke` are interpreted
/// as column `error` + term `smoke`; we quote each whitespace-delimited token as a phrase and OR
/// them so colons and operators stay literal.
pub(crate) fn fts5_prepare_match_query(query: &str) -> String {
    let tokens: Vec<&str> = query.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }
    if tokens.len() == 1 {
        return fts5_quote_token_as_phrase(tokens[0]);
    }
    tokens
        .into_iter()
        .map(fts5_quote_token_as_phrase)
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn fts5_quote_token_as_phrase(token: &str) -> String {
    let esc = token.replace('"', "\"\"");
    format!("\"{esc}\"")
}

/// Maps sqlite-vec / rusqlite errors that indicate wrong or 0-dim vectors into EmbedDim for clear recovery message.
/// Use at FTS+vector query boundaries so callers get a consistent EmbedDim error instead of raw sqlite strings.
pub(crate) fn map_sqlite_vec_error(e: rusqlite::Error) -> RagDbError {
    if error_message_indicates_embed_dim(&e.to_string()) {
        RagDbError::EmbedDim(0)
    } else {
        RagDbError::Sqlite(e)
    }
}

/// One row from workspace_chunks: id, text, source, summary, defines/imports/calls (JSON), type_, name, chunk_type, source_type, last_updated.
#[derive(Clone, Debug)]
/// ChunkRow.
pub struct ChunkRow {
    pub id: String,
    pub text: String,
    pub source: String,
    pub summary: String,
    pub defines: String,
    pub imports: String,
    pub type_: String,
    pub name: String,
    /// JSON array of called function/method names (outgoing calls).
    pub calls: String,
    /// Chunk type: "summary" (web snippet or file summary) or "detail" (full page chunk).
    pub chunk_type: String,
    /// Source type: "official", "stackoverflow", "blog", "unknown".
    pub source_type: String,
    /// Unix timestamp when chunk was last verified/ingested (web chunks); None for codebase chunks.
    pub last_updated: Option<u64>,
}

/// One row from summary_vectors: source path/URL and summary text for hierarchical search.
#[derive(Clone, Debug)]
/// SummaryRow.
pub struct SummaryRow {
    pub source: String,
    pub summary: String,
}
/// RagDb.
pub struct RagDb {
    conn: Mutex<Connection>,
    db_path: std::path::PathBuf,
}

impl RagDb {
    /// Open or create the RAG database and register sqlite-vec.
    /// Call once per process; uses rusqlite bundled + sqlite3_auto_extension.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RagDbError> {
        // Register the sqlite-vec extension exactly once per process (C-3).
        // SAFETY:
        //   1. The transmute casts `sqlite3_vec_init as *const ()` to the exact type expected by
        //      `sqlite3_auto_extension`: `unsafe extern "C" fn(*mut sqlite3, *mut *mut c_char,
        //      *const sqlite3_api_routines) -> c_int`.
        //   2. `sqlite_vec::sqlite3_vec_init` is a C-ABI function exported by the sqlite-vec crate
        //      and matches that signature exactly.
        //   3. The OnceLock ensures this call runs at most once per process, preventing the
        //      double-registration UB that would occur if the extension init ran on every new connection.
        SQLITE_VEC_REGISTERED.get_or_init(|| {
            #[allow(clippy::missing_transmute_annotations)]
            unsafe {
                rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                    sqlite_vec::sqlite3_vec_init as *const (),
                )));
            }
        });
        let db_path = path.as_ref().to_path_buf();
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;",
        )?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path,
        })
    }

    /// Path to the database file (for parallel read connections).
    /// Path to the SQLite RAG database file (e.g. data/rag.db).
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Open a second connection for parallel read (FTS and vector in parallel).
    /// Sets WAL, synchronous=NORMAL, busy_timeout so the reader waits for the writer instead of failing with "database is locked".
    pub(crate) fn open_reader(path: &Path) -> Result<Connection, RagDbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;",
        )?;
        Ok(conn)
    }

    /// Run a read-only closure with a connection. For file DBs uses a dedicated read connection (no write mutex); for :memory: uses the main connection.
    fn with_read_conn<F, T>(&self, f: F) -> Result<T, RagDbError>
    where
        F: FnOnce(&Connection) -> Result<T, RagDbError>,
    {
        if self.db_path.as_os_str() == ":memory:" {
            let conn = self
                .conn
                .lock()
                .map_err(|e| RagDbError::Poisoned(e.to_string()))?;
            f(&conn)
        } else {
            let conn = Self::open_reader(self.db_path())?;
            f(&conn)
        }
    }

    /// Run a write closure with retry on SQLITE_BUSY (exponential backoff). Maps DiskFull/IoErr to RagDbError::DiskFull.
    fn execute_write_with_retry<F, T>(&self, f: F) -> Result<T, RagDbError>
    where
        F: Fn(&Connection) -> Result<T, RagDbError>,
    {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let conn = self
                .conn
                .lock()
                .map_err(|e| RagDbError::Poisoned(e.to_string()))?;
            match f(&conn) {
                Ok(t) => return Ok(t),
                Err(RagDbError::Sqlite(e)) => {
                    let code = e.sqlite_error_code();
                    let is_busy = code == Some(ErrorCode::DatabaseBusy);
                    let is_disk_io = code == Some(ErrorCode::DiskFull)
                        || code == Some(ErrorCode::SystemIoFailure);
                    if is_disk_io {
                        return Err(RagDbError::DiskFull(e.to_string()));
                    }
                    if is_busy && attempt < WRITE_RETRY_MAX_ATTEMPTS {
                        drop(conn);
                        let backoff = write_retry_backoff_ms(attempt);
                        tracing::debug!(
                            "SQLITE_BUSY (attempt {}/{}), retrying in {}ms",
                            attempt,
                            WRITE_RETRY_MAX_ATTEMPTS,
                            backoff
                        );
                        std::thread::sleep(Duration::from_millis(backoff));
                        continue;
                    }
                    return Err(RagDbError::Sqlite(e));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Execute multiple write operations in a single transaction.
    /// Locks the write connection once, runs BEGIN IMMEDIATE, executes the closure, then COMMIT.
    /// On error, rolls back the transaction. Retries on SQLITE_BUSY with exponential backoff.
    pub fn execute_write_batch<F, T>(&self, f: F) -> Result<T, RagDbError>
    where
        F: FnOnce(&Connection) -> Result<T, RagDbError>,
    {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let conn = self
                .conn
                .lock()
                .map_err(|e| RagDbError::Poisoned(e.to_string()))?;
            match conn.execute_batch("BEGIN IMMEDIATE") {
                Ok(()) => {}
                Err(e) => {
                    let code = e.sqlite_error_code();
                    if code == Some(ErrorCode::DatabaseBusy)
                        && attempt < WRITE_RETRY_MAX_ATTEMPTS
                    {
                        drop(conn);
                        let backoff = write_retry_backoff_ms(attempt);
                        tracing::debug!(
                            "SQLITE_BUSY on BEGIN (attempt {}/{}), retrying in {}ms",
                            attempt,
                            WRITE_RETRY_MAX_ATTEMPTS,
                            backoff
                        );
                        std::thread::sleep(Duration::from_millis(backoff));
                        continue;
                    }
                    return Err(RagDbError::Sqlite(e));
                }
            }
            match f(&conn) {
                Ok(val) => {
                    conn.execute_batch("COMMIT")
                        .map_err(RagDbError::Sqlite)?;
                    return Ok(val);
                }
                Err(e) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    return Err(e);
                }
            }
        }
    }

    /// Run VACUUM to reclaim space and keep the sqlite-vec file from bloating. Call once on server startup.
    /// No-op for in-memory DBs. Returns Err on poisoned mutex or I/O failure.
    pub fn vacuum(&self) -> Result<(), RagDbError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RagDbError::Poisoned(e.to_string()))?;
        conn.execute_batch("VACUUM;")?;
        Ok(())
    }

    /// Run a PASSIVE WAL checkpoint to move WAL content into the main DB.
    /// Call after bulk writes to keep the WAL small and read performance high (per SQLite WAL docs).
    /// Returns Err on poisoned mutex so callers can log and degrade instead of panicking.
    pub fn wal_checkpoint_passive(&self) -> Result<(), RagDbError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RagDbError::Poisoned(e.to_string()))?;
        conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")?;
        Ok(())
    }

    /// Run a PASSIVE WAL checkpoint with retries on SQLITE_BUSY. Use after background ingest
    /// when multiple writers may contend; retries up to 3 times with 200ms backoff.
    pub fn wal_checkpoint_passive_retry(&self) {
        /// MAX_ATTEMPTS.
        const MAX_ATTEMPTS: u32 = 3;
        /// BACKOFF_MS.
        const BACKOFF_MS: u64 = 200;
        for attempt in 1..=MAX_ATTEMPTS {
            match self.wal_checkpoint_passive() {
                Ok(()) => return,
                Err(RagDbError::Sqlite(ref e)) => {
                    let is_busy = e
                        .sqlite_error_code()
                        .map(|c| c == ErrorCode::DatabaseBusy)
                        .unwrap_or(false);
                    if is_busy && attempt < MAX_ATTEMPTS {
                        tracing::debug!(
                            "WAL checkpoint busy (attempt {}/{}), retrying in {}ms",
                            attempt,
                            MAX_ATTEMPTS,
                            BACKOFF_MS
                        );
                        std::thread::sleep(Duration::from_millis(BACKOFF_MS));
                    } else {
                        if is_busy {
                            tracing::warn!(
                                "WAL checkpoint skipped after {} attempts (database busy)",
                                MAX_ATTEMPTS
                            );
                        } else {
                            tracing::warn!("WAL checkpoint failed: {}", e);
                        }
                        return;
                    }
                }
                Err(e) => {
                    tracing::warn!("WAL checkpoint failed: {}", e);
                    return;
                }
            }
        }
    }

    /// Insert a row into semantic_cache (query_text, response_text, embedding, created_at).
    pub fn semantic_cache_insert(
        &self,
        query_text: &str,
        response_text: &str,
        embedding: &[f32],
    ) -> Result<(), RagDbError> {
        if embedding.len() != RAG_EMBED_DIM {
            return Err(RagDbError::EmbedDim(embedding.len()));
        }
        let blob = embedding_to_blob(embedding);
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "INSERT INTO semantic_cache (query_text, response_text, embedding, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![query_text, response_text, blob.as_slice(), created_at],
            )?;
            let cache_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO semantic_cache_vectors (cache_id, embedding) VALUES (?1, ?2)",
                rusqlite::params![cache_id, blob.as_slice()],
            )?;
            Ok(())
        })
    }

    /// Fetch recent semantic_cache rows (query_text, response_text, embedding, created_at) for similarity check. Order by created_at DESC, limit.
    pub fn semantic_cache_fetch_recent(
        &self,
        limit: usize,
    ) -> Result<Vec<SemanticCacheRow>, RagDbError> {
        self.with_read_conn(|conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT query_text, response_text, embedding, created_at FROM semantic_cache ORDER BY created_at DESC LIMIT ?1",
            )?;
            let rows = stmt.query_map(rusqlite::params![limit as i64], |row| {
                let query_text: String = row.get(0)?;
                let response_text: String = row.get(1)?;
                let blob: Vec<u8> = row.get(2)?;
                let created_at: i64 = row.get(3)?;
                let embedding = blob_to_embedding(&blob)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok((query_text, response_text, embedding, created_at))
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    /// Prune semantic_cache: keep at most max_rows, and delete rows older than older_than_secs (from now).
    pub fn semantic_cache_prune(
        &self,
        max_rows: usize,
        older_than_secs: i64,
    ) -> Result<(), RagDbError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let cutoff = now.saturating_sub(older_than_secs);
        self.execute_write_with_retry(|conn| {
            conn.execute("DELETE FROM semantic_cache WHERE created_at < ?1", rusqlite::params![cutoff])?;
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM semantic_cache", [], |r| r.get(0))?;
            if count > max_rows as i64 {
                let keep = max_rows as i64;
                conn.execute(
                    "DELETE FROM semantic_cache WHERE id NOT IN (SELECT id FROM semantic_cache ORDER BY created_at DESC LIMIT ?1)",
                    rusqlite::params![keep],
                )?;
            }
            conn.execute(
                "DELETE FROM semantic_cache_vectors WHERE cache_id NOT IN (SELECT id FROM semantic_cache)",
                [],
            )?;
            Ok(())
        })
    }

    /// KNN lookup in semantic cache using vec0 index. Returns cached response if similarity >= threshold.
    pub fn semantic_cache_knn(
        &self,
        query_embedding: &[f32],
        threshold: f32,
    ) -> Result<Option<String>, RagDbError> {
        if query_embedding.len() != RAG_EMBED_DIM {
            return Err(RagDbError::EmbedDim(query_embedding.len()));
        }
        let blob = embedding_to_blob(query_embedding);
        self.with_read_conn(|conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT sc.response_text, scv.distance
                 FROM semantic_cache_vectors scv
                 JOIN semantic_cache sc ON sc.id = scv.cache_id
                 WHERE scv.embedding MATCH ?1 AND k = 1",
            ).map_err(map_sqlite_vec_error)?;
            let mut rows = stmt.query_map(rusqlite::params![blob.as_slice()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            }).map_err(map_sqlite_vec_error)?;
            if let Some(Ok((response, distance))) = rows.next() {
                // For unit-normalized embeddings (Nomic): cos_sim ≈ 1 - dist^2/2
                let sim = 1.0 - (distance * distance / 2.0) as f32;
                if sim >= threshold {
                    return Ok(Some(response));
                }
            }
            Ok(None)
        })
    }

    /// FTS5 search with an existing connection (for parallel hybrid).
    pub(crate) fn search_fts_impl(
        conn: &Connection,
        query: &str,
        limit: usize,
        source_filter: Option<&[String]>,
    ) -> Result<Vec<(String, f64)>, RagDbError> {
        const MAX_FTS_SOURCE_FILTER: usize = 512;

        let fts_query = fts5_prepare_match_query(query);
        if fts_query.is_empty() {
            return Ok(vec![]);
        }

        fn push_ranked_rows(
            mut rows: rusqlite::Rows<'_>,
        ) -> Result<Vec<(String, f64)>, rusqlite::Error> {
            let mut out = Vec::new();
            let mut idx = 0usize;
            while let Some(row) = rows.next()? {
                let id = row.get::<_, String>(0)?;
                let _bm25 = row.get::<_, f64>(1)?;
                idx += 1;
                let rank = idx as f64;
                out.push((id, 1.0 / (rank + RRF_K)));
            }
            Ok(out)
        }

        match source_filter {
            None => {
                let mut stmt = conn.prepare_cached(
                    r#"
            SELECT c.id, bm25(chunk_fts) AS r
            FROM chunk_fts
            JOIN workspace_chunks c ON c.rowid = chunk_fts.rowid
            WHERE chunk_fts MATCH ?1
            ORDER BY r
            LIMIT ?2
            "#,
                )?;
                let rows = stmt.query(rusqlite::params![&fts_query, limit as i64])?;
                push_ranked_rows(rows).map_err(RagDbError::from)
            }
            Some(&[]) => Ok(vec![]),
            Some(sources) => {
                let sources = if sources.len() > MAX_FTS_SOURCE_FILTER {
                    tracing::warn!(
                        len = sources.len(),
                        cap = MAX_FTS_SOURCE_FILTER,
                        "FTS source_filter truncated"
                    );
                    &sources[..MAX_FTS_SOURCE_FILTER]
                } else {
                    sources
                };
                let placeholders = (3..sources.len() + 3)
                    .map(|i| format!("?{i}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    r#"
            SELECT c.id, bm25(chunk_fts) AS r
            FROM chunk_fts
            JOIN workspace_chunks c ON c.rowid = chunk_fts.rowid
            WHERE chunk_fts MATCH ?1 AND c.source IN ({placeholders})
            ORDER BY r
            LIMIT ?2
            "#
                );
                let mut stmt = conn.prepare(&sql)?;
                stmt.raw_bind_parameter(1, &fts_query)?;
                stmt.raw_bind_parameter(2, limit as i64)?;
                for (i, src) in sources.iter().enumerate() {
                    stmt.raw_bind_parameter(i + 3, src.as_str())?;
                }
                let rows = stmt.raw_query();
                push_ranked_rows(rows).map_err(RagDbError::from)
            }
        }
    }

    /// Vector KNN with an existing connection (for parallel hybrid).
    /// Rejects empty or wrong-dim query vectors; 0-dim can also indicate corrupt/stale DB (run check_embedding_dimension at startup).
    pub(crate) fn search_vector_knn_impl(
        conn: &Connection,
        query_embedding: &[f32],
        limit: usize,
        source_filter: Option<&[String]>,
    ) -> Result<Vec<(String, f64)>, RagDbError> {
        if query_embedding.is_empty() || query_embedding.len() != RAG_EMBED_DIM {
            return Err(RagDbError::EmbedDim(query_embedding.len()));
        }
        let blob: Vec<u8> = query_embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let fetch_limit = if source_filter.is_some() {
            limit * 3
        } else {
            limit
        };
        let mut stmt = conn
            .prepare(
                "SELECT chunk_id, distance FROM chunk_vectors WHERE embedding MATCH ?1 AND k = ?2",
            )
            .map_err(map_sqlite_vec_error)?;
        let rows = stmt
            .query_map(
                rusqlite::params![blob.as_slice(), fetch_limit as i64],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
            )
            .map_err(map_sqlite_vec_error)?;
        let mut with_dist: Vec<(String, f64)> = rows.filter_map(|r| r.ok()).collect();
        if let Some(sources) = source_filter {
            let set: std::collections::HashSet<_> = sources.iter().map(|s| s.as_str()).collect();
            with_dist.retain(|(id, _)| {
                let path_part = id.rsplit_once('#').map(|(p, _)| p).unwrap_or(id.as_str());
                set.contains(path_part)
            });
        }
        with_dist.truncate(limit);
        for (idx, (_, score)) in with_dist.iter_mut().enumerate() {
            *score = 1.0 / ((idx as f64) + 1.0 + RRF_K);
        }
        Ok(with_dist)
    }

    /// Creates workspace_chunks, vec0, file_summaries, symbol_index, FTS5 tables if not exist.
    /// Creates workspace_chunks, summary_vectors, FTS5, vec0; called once from open().
    fn init_schema(conn: &Connection) -> Result<(), RagDbError> {
        conn.execute_batch(
            r#"
            -- Chunk metadata (no vector here; vector in vec0 table)
            CREATE TABLE IF NOT EXISTS workspace_chunks (
                id TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                source TEXT NOT NULL,
                summary TEXT NOT NULL DEFAULT '',
                defines TEXT NOT NULL DEFAULT '[]',
                imports TEXT NOT NULL DEFAULT '[]',
                type TEXT NOT NULL DEFAULT 'text',
                name TEXT NOT NULL DEFAULT 'unknown',
                calls_metadata TEXT NOT NULL DEFAULT '[]',
                chunk_type TEXT NOT NULL DEFAULT 'code',
                source_type TEXT NOT NULL DEFAULT 'unknown'
            );

            -- vec0 virtual table: 768-d Nomic embeddings, chunk_id primary key to join
            CREATE VIRTUAL TABLE IF NOT EXISTS chunk_vectors USING vec0(
                chunk_id TEXT primary key,
                embedding float[768]
            );

            -- FTS5 for hybrid search over chunk text
            CREATE VIRTUAL TABLE IF NOT EXISTS chunk_fts USING fts5(
                text,
                content='workspace_chunks',
                content_rowid='rowid'
            );
            CREATE TRIGGER IF NOT EXISTS chunk_fts_insert AFTER INSERT ON workspace_chunks BEGIN
                INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);
            END;
            CREATE TRIGGER IF NOT EXISTS chunk_fts_delete AFTER DELETE ON workspace_chunks BEGIN
                INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
            END;
            CREATE TRIGGER IF NOT EXISTS chunk_fts_update AFTER UPDATE ON workspace_chunks BEGIN
                INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
                INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);
            END;

            -- File-level summaries for hierarchical search
            CREATE TABLE IF NOT EXISTS file_summaries (
                source TEXT PRIMARY KEY,
                summary TEXT NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS summary_vectors USING vec0(
                source TEXT primary key,
                embedding float[768]
            );

            -- Symbol -> chunk_id for definitions only (jump-to-definition)
            CREATE TABLE IF NOT EXISTS symbol_index (
                symbol TEXT NOT NULL,
                chunk_id TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_symbol_index_symbol ON symbol_index(symbol);
            CREATE INDEX IF NOT EXISTS idx_symbol_index_chunk_id ON symbol_index(chunk_id);

            -- Chunks that reference a symbol (import or call)
            CREATE TABLE IF NOT EXISTS reference_index (
                symbol TEXT NOT NULL,
                chunk_id TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_reference_index_symbol ON reference_index(symbol);
            CREATE INDEX IF NOT EXISTS idx_reference_index_chunk_id ON reference_index(chunk_id);

            -- Golden set: preferred patterns for auto-approve (pattern recognition + preferences over time)
            CREATE TABLE IF NOT EXISTS golden_patterns (
                pattern_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                code TEXT NOT NULL,
                language TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS golden_pattern_vectors USING vec0(
                pattern_id TEXT primary key,
                embedding float[768]
            );

            -- Semantic cache for query_knowledge: similar queries return cached response (optional, SEMANTIC_CACHE_ENABLED).
            CREATE TABLE IF NOT EXISTS semantic_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query_text TEXT NOT NULL,
                response_text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_semantic_cache_created_at ON semantic_cache(created_at);

            CREATE VIRTUAL TABLE IF NOT EXISTS semantic_cache_vectors USING vec0(
                cache_id INTEGER PRIMARY KEY,
                embedding float[768]
            );
            "#,
        )?;
        // Migration: backfill existing semantic_cache rows into vec0
        let orphan_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM semantic_cache WHERE id NOT IN (SELECT cache_id FROM semantic_cache_vectors)",
            [],
            |r| r.get(0),
        ).unwrap_or(0);
        if orphan_count > 0 {
            let mut backfill_stmt = conn.prepare(
                "SELECT id, embedding FROM semantic_cache WHERE id NOT IN (SELECT cache_id FROM semantic_cache_vectors)"
            )?;
            let backfill_rows: Vec<(i64, Vec<u8>)> = backfill_stmt.query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?.filter_map(|r| r.ok()).collect();
            for (id, blob) in &backfill_rows {
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO semantic_cache_vectors (cache_id, embedding) VALUES (?1, ?2)",
                    rusqlite::params![id, blob.as_slice()],
                );
            }
        }
        // Migration: add calls_metadata for existing DBs created before this column existed.
        let _ = conn.execute(
            "ALTER TABLE workspace_chunks ADD COLUMN calls_metadata TEXT NOT NULL DEFAULT '[]'",
            [],
        );
        // Migration: add last_updated for web chunk TTL (NULL = never prune).
        let _ = conn.execute(
            "ALTER TABLE workspace_chunks ADD COLUMN last_updated INTEGER DEFAULT NULL",
            [],
        );
        // Migration: add chunk_type for hybrid retrieval (summary vs. detail).
        let _ = conn.execute(
            "ALTER TABLE workspace_chunks ADD COLUMN chunk_type TEXT NOT NULL DEFAULT 'code'",
            [],
        );
        // Migration: add source_type for quality scoring (official, stackoverflow, blog, unknown).
        let _ = conn.execute(
            "ALTER TABLE workspace_chunks ADD COLUMN source_type TEXT NOT NULL DEFAULT 'unknown'",
            [],
        );
        // Performance: indexes on columns used in WHERE/ORDER BY.
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_source_type ON workspace_chunks(source_type)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_last_updated ON workspace_chunks(last_updated)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_source ON workspace_chunks(source)",
            [],
        );
        Ok(())
    }

    /// Insert or replace a chunk. If embedding is Some, it must have length 768 and is written to chunk_vectors.
    /// If None, only workspace_chunks (and FTS) are written; no vector row (FTS-only ingest).
    /// last_updated: set for web chunks (Unix timestamp) for TTL pruning; None for workspace chunks.
    /// chunk_type: "summary" for web/file summaries, "detail" for full page chunks, "code" for codebase.
    /// source_type: "official", "stackoverflow", "blog", "unknown" for quality scoring.
    #[allow(clippy::too_many_arguments)]
    /// upsert_chunk.
    pub fn upsert_chunk(
        &self,
        id: &str,
        text: &str,
        source: &str,
        summary: &str,
        defines: &str,
        imports: &str,
        type_: &str,
        name: &str,
        calls_metadata: &str,
        embedding: Option<&[f32]>,
        last_updated: Option<u64>,
        chunk_type: &str,
        source_type: &str,
    ) -> Result<(), RagDbError> {
        if let Some(emb) = embedding {
            if emb.len() != RAG_EMBED_DIM {
                return Err(RagDbError::EmbedDim(emb.len()));
            }
        }
        let last_ts: Option<i64> = last_updated.map(|u| u as i64);
        let embedding_blob: Option<Vec<u8>> = embedding.map(embedding_to_blob);
        self.execute_write_with_retry(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO workspace_chunks (id, text, source, summary, defines, imports, type, name, calls_metadata, last_updated, chunk_type, source_type)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
                rusqlite::params![
                    id, text, source, summary, defines, imports, type_, name, calls_metadata,
                    last_ts, chunk_type, source_type
                ],
            )?;
            if let Some(ref blob) = embedding_blob {
                conn.execute(
                    "INSERT OR REPLACE INTO chunk_vectors (chunk_id, embedding) VALUES (?1, ?2)",
                    rusqlite::params![id, blob.as_slice()],
                )?;
            } else {
                conn.execute(
                    "DELETE FROM chunk_vectors WHERE chunk_id = ?1",
                    rusqlite::params![id],
                )?;
            }
            Ok(())
        })
    }

    /// One row for batch chunk insert. Used by ingest to write all chunks of a file in a single transaction.
    /// last_updated: file mtime (Unix secs) for workspace chunks so RAG can show last_verified_date; None for no timestamp.
    #[allow(clippy::too_many_arguments)]
    /// upsert_chunks_batch.
    pub fn upsert_chunks_batch(
        &self,
        rows: &[ChunkBatchRowRef<'_>],
        chunk_type: &str,
        source_type: &str,
        last_updated: Option<u64>,
    ) -> Result<(), RagDbError> {
        if rows.is_empty() {
            return Ok(());
        }
        for (_, _, _, _, _, _, _, _, emb) in rows.iter() {
            if let Some(e) = emb {
                if e.len() != RAG_EMBED_DIM {
                    return Err(RagDbError::EmbedDim(e.len()));
                }
            }
        }
        let last_ts: Option<i64> = last_updated.map(|u| u as i64);
        self.execute_write_with_retry(|conn| {
            let mut chunk_stmt = conn.prepare_cached(
                r#"
                INSERT OR REPLACE INTO workspace_chunks (id, text, source, summary, defines, imports, type, name, calls_metadata, last_updated, chunk_type, source_type)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
            )?;
            let mut vec_stmt = conn.prepare_cached(
                "INSERT OR REPLACE INTO chunk_vectors (chunk_id, embedding) VALUES (?1, ?2)",
            )?;
            for (id, text, source, defines, imports, type_, name, calls_metadata, embedding) in rows {
                chunk_stmt.execute(rusqlite::params![
                    id,
                    text,
                    source,
                    "",
                    defines,
                    imports,
                    type_,
                    name,
                    calls_metadata,
                    last_ts,
                    chunk_type,
                    source_type,
                ])?;
                if let Some(emb) = embedding {
                    vec_stmt.execute(rusqlite::params![id, embedding_to_blob(emb).as_slice()])?;
                } else {
                    conn.execute(
                        "DELETE FROM chunk_vectors WHERE chunk_id = ?1",
                        rusqlite::params![id],
                    )?;
                }
            }
            Ok(())
        })
    }

    /// Delete all chunks for a source path (and their vectors/symbol_index/reference_index rows).
    pub fn delete_chunks_by_source(&self, source: &str) -> Result<(), RagDbError> {
        // Batch-delete auxiliary rows with LIKE/exact patterns: 4 statements regardless of chunk count,
        // instead of the previous O(n) per-chunk DELETE loop.
        let escaped = escape_sqlite_like_literal(source);
        let pattern = format!("{escaped}#%");
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "DELETE FROM symbol_index WHERE chunk_id LIKE ?1 ESCAPE '\\'",
                rusqlite::params![pattern],
            )?;
            conn.execute(
                "DELETE FROM reference_index WHERE chunk_id LIKE ?1 ESCAPE '\\'",
                rusqlite::params![pattern],
            )?;
            conn.execute(
                "DELETE FROM chunk_vectors WHERE chunk_id LIKE ?1 ESCAPE '\\'",
                rusqlite::params![pattern],
            )?;
            conn.execute(
                "DELETE FROM workspace_chunks WHERE source = ?1",
                rusqlite::params![source],
            )?;
            Ok(())
        })
    }

    /// Prune web chunks (source LIKE 'https://%') with last_updated older than the given number of days.
    /// Deletes from chunk_vectors, workspace_chunks (FTS via trigger), file_summaries, and summary_vectors for those sources; symbol/reference index typically empty for web.
    pub fn prune_web_chunks_older_than(&self, days: u32) -> Result<u32, RagDbError> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .saturating_sub(days as u64 * 86400);
        let cutoff_i: i64 = cutoff as i64;
        // Subquery-based batch deletes: O(1) statements instead of O(n) per-chunk loop.
        let web_filter =
            "source LIKE 'https://%' AND last_updated IS NOT NULL AND last_updated < ?1";
        self.execute_write_with_retry(|conn| {
            let n: u32 = conn.query_row(
                &format!("SELECT COUNT(*) FROM workspace_chunks WHERE {web_filter}"),
                rusqlite::params![cutoff_i],
                |row| row.get(0),
            ).unwrap_or(0);
            if n == 0 {
                return Ok(0);
            }
            // Regular tables: delete via subquery join in one statement each.
            conn.execute(
                &format!("DELETE FROM symbol_index WHERE chunk_id IN (SELECT id FROM workspace_chunks WHERE {web_filter})"),
                rusqlite::params![cutoff_i],
            )?;
            conn.execute(
                &format!("DELETE FROM reference_index WHERE chunk_id IN (SELECT id FROM workspace_chunks WHERE {web_filter})"),
                rusqlite::params![cutoff_i],
            )?;
            // sqlite-vec virtual table: LIKE-pattern delete on chunk_id (scans by prefix).
            conn.execute(
                &format!("DELETE FROM chunk_vectors WHERE chunk_id IN (SELECT id FROM workspace_chunks WHERE {web_filter})"),
                rusqlite::params![cutoff_i],
            )?;
            // Delete chunks and associated summaries in batch.
            conn.execute(
                &format!("DELETE FROM file_summaries WHERE source IN (SELECT DISTINCT source FROM workspace_chunks WHERE {web_filter})"),
                rusqlite::params![cutoff_i],
            )?;
            conn.execute(
                &format!("DELETE FROM summary_vectors WHERE source IN (SELECT DISTINCT source FROM workspace_chunks WHERE {web_filter})"),
                rusqlite::params![cutoff_i],
            )?;
            conn.execute(
                &format!("DELETE FROM workspace_chunks WHERE {web_filter}"),
                rusqlite::params![cutoff_i],
            )?;
            Ok(n)
        })
    }

    /// FTS5 search: returns (chunk_id, rrf_score). Uses workspace_chunks.rowid join.
    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<(String, f64)>, RagDbError> {
        self.with_read_conn(|conn| Self::search_fts_impl(conn, query, limit, None))
    }

    /// Vector KNN via sqlite-vec. Returns (chunk_id, distance) then we convert to RRF score.
    /// Query embedding must be 768-d.
    pub fn search_vector_knn(
        &self,
        query_embedding: &[f32],
        limit: usize,
        source_filter: Option<&[String]>,
    ) -> Result<Vec<(String, f64)>, RagDbError> {
        self.with_read_conn(|conn| {
            Self::search_vector_knn_impl(conn, query_embedding, limit, source_filter)
        })
    }

    /// Delete chunk_vectors rows whose chunk_id no longer exists in workspace_chunks.
    /// Run after bulk deletes (e.g. prune_orphans, Python-based cleanup) to keep the vector
    /// index in sync. Returns the number of orphaned rows removed.
    pub fn prune_orphaned_chunk_vectors(&self) -> Result<u64, RagDbError> {
        self.execute_write_with_retry(|conn| {
            let n = conn.execute(
                "DELETE FROM chunk_vectors WHERE chunk_id NOT IN (SELECT id FROM workspace_chunks)",
                [],
            )?;
            Ok(n as u64)
        })
    }

    /// Returns true if chunk_vectors has at least one row (used for startup warning when embedder is stub).
    pub fn has_any_chunk_vectors(&self) -> Result<bool, RagDbError> {
        self.with_read_conn(|conn| {
            let has = conn
                .query_row("SELECT 1 FROM chunk_vectors LIMIT 1", [], |_| Ok(()))
                .is_ok();
            Ok(has)
        })
    }

    /// Total number of rows in workspace_chunks.
    pub fn count_chunks(&self) -> Result<u64, RagDbError> {
        self.with_read_conn(|conn| {
            let n: i64 = conn.query_row("SELECT COUNT(*) FROM workspace_chunks", [], |row| {
                row.get(0)
            })?;
            Ok(n as u64)
        })
    }

    /// If chunk_vectors has any rows, verify one row's embedding blob has dimension 768.
    /// Returns Err when the DB has inconsistent/0-dim vectors (e.g. ingest was run without ORT_DYLIB_PATH).
    /// Call at startup (serve/verify/query) and on error suggest: delete rag.db, set ORT_DYLIB_PATH, re-run ingest.
    pub fn check_embedding_dimension(&self) -> Result<(), RagDbError> {
        self.with_read_conn(|conn| {
            let has = conn
                .query_row("SELECT 1 FROM chunk_vectors LIMIT 1", [], |_| Ok(()))
                .is_ok();
            if !has {
                return Ok(());
            }
            let blob: Vec<u8> =
                conn.query_row("SELECT embedding FROM chunk_vectors LIMIT 1", [], |row| {
                    row.get(0)
                })?;
            let expected_bytes = RAG_EMBED_DIM * BYTES_PER_DIM;
            if blob.len() != expected_bytes {
                let got_dims = blob.len() / BYTES_PER_DIM;
                return Err(RagDbError::EmbedDim(got_dims));
            }
            Ok(())
        })
    }
    /// Merge FTS and vector result lists using Reciprocal Rank Fusion. Uses [`RRF_K`]; score = 1 / (rank + RRF_K).
    pub fn rrf_merge(
        fts: Vec<(String, f64)>,
        vec: Vec<(String, f64)>,
        limit: usize,
    ) -> Vec<String> {
        let mut combined: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for (id, s) in fts {
            *combined.entry(id).or_insert(0.0) += s;
        }
        for (id, s) in vec {
            *combined.entry(id).or_insert(0.0) += s;
        }
        let mut order: Vec<(String, f64)> = combined.into_iter().collect();
        order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        order.into_iter().take(limit).map(|(id, _)| id).collect()
    }

    /// Load chunk rows by id (preserve order). Uses a dedicated read connection so search is not blocked by the write mutex during ingest.
    /// For in-memory DB (":memory:") uses the main connection since a second :memory: connection would be a different empty DB.
    pub fn get_chunks_by_ids(&self, ids: &[String]) -> Result<Vec<ChunkRow>, RagDbError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        self.with_read_conn(|conn| Self::get_chunks_by_ids_impl(conn, ids))
    }
    /// get_chunks_by_ids_impl.
    fn get_chunks_by_ids_impl(
        conn: &Connection,
        ids: &[String],
    ) -> Result<Vec<ChunkRow>, RagDbError> {
        let placeholders = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, text, source, summary, defines, imports, type, name, COALESCE(calls_metadata, '[]'), COALESCE(chunk_type, 'code'), COALESCE(source_type, 'unknown'), last_updated FROM workspace_chunks WHERE id IN ({})",
            placeholders
        );
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> =
            ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(param_refs), |row| {
            let last_updated: Option<i64> = row.get(11)?;
            Ok(ChunkRow {
                id: row.get(0)?,
                text: row.get(1)?,
                source: row.get(2)?,
                summary: row.get(3)?,
                defines: row.get(4)?,
                imports: row.get(5)?,
                type_: row.get(6)?,
                name: row.get(7)?,
                calls: row.get(8)?,
                chunk_type: row.get(9)?,
                source_type: row.get(10)?,
                last_updated: last_updated.map(|t| t as u64),
            })
        })?;
        let by_id: std::collections::HashMap<String, ChunkRow> = rows
            .filter_map(Result::ok)
            .map(|r| (r.id.clone(), r))
            .collect();
        let out = ids.iter().filter_map(|id| by_id.get(id).cloned()).collect();
        Ok(out)
    }

    /// Chunks for one source path.
    pub fn get_chunks_by_source(&self, source: &str) -> Result<Vec<ChunkRow>, RagDbError> {
        self.with_read_conn(|conn| Self::get_chunks_by_source_impl(conn, source))
    }

    /// get_chunks_by_source_impl.
    fn get_chunks_by_source_impl(
        conn: &Connection,
        source: &str,
    ) -> Result<Vec<ChunkRow>, RagDbError> {
        let mut stmt = conn.prepare_cached(
            "SELECT id, text, source, summary, defines, imports, type, name, COALESCE(calls_metadata, '[]'), COALESCE(chunk_type, 'code'), COALESCE(source_type, 'unknown'), last_updated FROM workspace_chunks WHERE source = ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![source], |row| {
            let last_updated: Option<i64> = row.get(11)?;
            Ok(ChunkRow {
                id: row.get(0)?,
                text: row.get(1)?,
                source: row.get(2)?,
                summary: row.get(3)?,
                defines: row.get(4)?,
                imports: row.get(5)?,
                type_: row.get(6)?,
                name: row.get(7)?,
                calls: row.get(8)?,
                chunk_type: row.get(9)?,
                source_type: row.get(10)?,
                last_updated: last_updated.map(|t| t as u64),
            })
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    /// Symbol index: chunk_ids that define the symbol (definition-only).
    pub fn get_chunk_ids_for_symbol(&self, symbol: &str) -> Result<Vec<String>, RagDbError> {
        self.with_read_conn(|conn| Self::get_chunk_ids_for_symbol_impl(conn, symbol))
    }

    /// get_chunk_ids_for_symbol_impl.
    fn get_chunk_ids_for_symbol_impl(
        conn: &Connection,
        symbol: &str,
    ) -> Result<Vec<String>, RagDbError> {
        let mut stmt =
            conn.prepare_cached("SELECT chunk_id FROM symbol_index WHERE symbol = ?1 LIMIT 100")?;
        let rows = stmt.query_map(rusqlite::params![symbol], |row| row.get::<_, String>(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    /// Batch symbol lookup: returns `symbol → Vec<chunk_id>` for all given symbols in one query (H-2).
    /// Replaces repeated `get_chunk_ids_for_symbol` calls in graph-walk expansion, reducing
    /// up to O(rows × symbols_per_row) DB round-trips to a single round-trip.
    pub fn get_chunk_ids_for_symbols_batch(
        &self,
        symbols: &[String],
    ) -> Result<std::collections::HashMap<String, Vec<String>>, RagDbError> {
        if symbols.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        self.with_read_conn(|conn| {
            let placeholders = (1..=symbols.len())
                .map(|i| format!("?{}", i))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "SELECT symbol, chunk_id FROM symbol_index WHERE symbol IN ({}) LIMIT 1000",
                placeholders
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut result: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            let rows = stmt.query_map(
                rusqlite::params_from_iter(symbols.iter().map(|s| s.as_str())),
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?;
            for row in rows.filter_map(Result::ok) {
                result.entry(row.0).or_default().push(row.1);
            }
            Ok(result)
        })
    }

    /// insert_symbol_index.
    pub fn insert_symbol_index(&self, symbol: &str, chunk_id: &str) -> Result<(), RagDbError> {
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "INSERT INTO symbol_index (symbol, chunk_id) VALUES (?1, ?2)",
                rusqlite::params![symbol, chunk_id],
            )?;
            Ok(())
        })
    }
    /// delete_symbol_index_by_chunk_prefix.
    pub fn delete_symbol_index_by_chunk_prefix(&self, source: &str) -> Result<(), RagDbError> {
        let escaped = escape_sqlite_like_literal(source);
        let pattern = format!("{escaped}#%");
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "DELETE FROM symbol_index WHERE chunk_id LIKE ?1 ESCAPE '\\'",
                rusqlite::params![pattern],
            )?;
            Ok(())
        })
    }

    /// Reference index: chunk_ids that reference the symbol (import or call).
    pub fn get_chunk_ids_referencing_symbol(
        &self,
        symbol: &str,
    ) -> Result<Vec<String>, RagDbError> {
        self.with_read_conn(|conn| Self::get_chunk_ids_referencing_symbol_impl(conn, symbol))
    }

    /// get_chunk_ids_referencing_symbol_impl.
    fn get_chunk_ids_referencing_symbol_impl(
        conn: &Connection,
        symbol: &str,
    ) -> Result<Vec<String>, RagDbError> {
        let mut stmt =
            conn.prepare_cached("SELECT chunk_id FROM reference_index WHERE symbol = ?1 LIMIT 100")?;
        let rows = stmt.query_map(rusqlite::params![symbol], |row| row.get::<_, String>(0))?;
        Ok(rows.filter_map(Result::ok).collect())
    }
    /// insert_reference_index.
    pub fn insert_reference_index(&self, symbol: &str, chunk_id: &str) -> Result<(), RagDbError> {
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "INSERT INTO reference_index (symbol, chunk_id) VALUES (?1, ?2)",
                rusqlite::params![symbol, chunk_id],
            )?;
            Ok(())
        })
    }
    /// Batch insert (symbol, chunk_id) pairs into symbol_index in a single transaction.
    pub fn batch_insert_symbol_index(&self, pairs: &[(&str, &str)]) -> Result<(), RagDbError> {
        if pairs.is_empty() {
            return Ok(());
        }
        self.execute_write_with_retry(|conn| {
            let mut stmt =
                conn.prepare_cached("INSERT INTO symbol_index (symbol, chunk_id) VALUES (?1, ?2)")?;
            for (symbol, chunk_id) in pairs {
                stmt.execute(rusqlite::params![symbol, chunk_id])?;
            }
            Ok(())
        })
    }
    /// Batch insert (symbol, chunk_id) pairs into reference_index in a single transaction.
    pub fn batch_insert_reference_index(&self, pairs: &[(&str, &str)]) -> Result<(), RagDbError> {
        if pairs.is_empty() {
            return Ok(());
        }
        self.execute_write_with_retry(|conn| {
            let mut stmt = conn
                .prepare_cached("INSERT INTO reference_index (symbol, chunk_id) VALUES (?1, ?2)")?;
            for (symbol, chunk_id) in pairs {
                stmt.execute(rusqlite::params![symbol, chunk_id])?;
            }
            Ok(())
        })
    }
    /// delete_reference_index_by_chunk_prefix.
    pub fn delete_reference_index_by_chunk_prefix(&self, source: &str) -> Result<(), RagDbError> {
        let escaped = escape_sqlite_like_literal(source);
        let pattern = format!("{escaped}#%");
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "DELETE FROM reference_index WHERE chunk_id LIKE ?1 ESCAPE '\\'",
                rusqlite::params![pattern],
            )?;
            Ok(())
        })
    }

    /// Insert a golden pattern (preferred pattern for auto-approve). Call after approve_pattern so preferences accumulate.
    pub fn insert_golden_pattern(
        &self,
        pattern_id: &str,
        name: &str,
        code: &str,
        language: &str,
        embedding: &[f32],
    ) -> Result<(), RagDbError> {
        if embedding.len() != RAG_EMBED_DIM {
            return Err(RagDbError::EmbedDim(embedding.len()));
        }
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let blob = embedding_to_blob(embedding);
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO golden_patterns (pattern_id, name, code, language, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![pattern_id, name, code, language, created_at],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO golden_pattern_vectors (pattern_id, embedding) VALUES (?1, ?2)",
                rusqlite::params![pattern_id, blob.as_slice()],
            )?;
            Ok(())
        })
    }

    /// KNN over golden pattern embeddings. Returns (pattern_id, distance) with raw L2 distance (smaller = more similar).
    pub fn search_golden_patterns_knn(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<(String, f64)>, RagDbError> {
        if query_embedding.len() != RAG_EMBED_DIM {
            return Err(RagDbError::EmbedDim(query_embedding.len()));
        }
        let blob = embedding_to_blob(query_embedding);
        self.with_read_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT pattern_id, distance FROM golden_pattern_vectors WHERE embedding MATCH ?1 AND k = ?2")
                .map_err(map_sqlite_vec_error)?;
            let rows = stmt
                .query_map(
                    rusqlite::params![blob.as_slice(), k as i64],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
                )
                .map_err(map_sqlite_vec_error)?;
            Ok(rows.filter_map(Result::ok).collect())
        })
    }

    /// File summaries: upsert one summary. If embedding is Some (length 768), also write to summary_vectors.
    /// If None, only file_summaries is written (FTS-only ingest).
    pub fn upsert_summary(
        &self,
        source: &str,
        summary: &str,
        embedding: Option<&[f32]>,
    ) -> Result<(), RagDbError> {
        if let Some(emb) = embedding {
            if emb.len() != RAG_EMBED_DIM {
                return Err(RagDbError::EmbedDim(emb.len()));
            }
        }
        let embedding_blob: Option<Vec<u8>> =
            embedding.map(|e| e.iter().flat_map(|f| f.to_le_bytes()).collect());
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO file_summaries (source, summary) VALUES (?1, ?2)",
                rusqlite::params![source, summary],
            )?;
            if let Some(ref blob) = embedding_blob {
                conn.execute(
                    "INSERT OR REPLACE INTO summary_vectors (source, embedding) VALUES (?1, ?2)",
                    rusqlite::params![source, blob.as_slice()],
                )?;
            }
            Ok(())
        })
    }
    /// delete_summary_by_source.
    pub fn delete_summary_by_source(&self, source: &str) -> Result<(), RagDbError> {
        self.execute_write_with_retry(|conn| {
            conn.execute(
                "DELETE FROM file_summaries WHERE source = ?1",
                rusqlite::params![source],
            )?;
            conn.execute(
                "DELETE FROM summary_vectors WHERE source = ?1",
                rusqlite::params![source],
            )?;
            Ok(())
        })
    }

    // ── Connection-level variants for batch transactions ──────────────

    /// Delete all chunks for a source path using an existing connection (no mutex).
    pub(crate) fn delete_chunks_by_source_conn(
        conn: &Connection,
        source: &str,
    ) -> Result<(), RagDbError> {
        let escaped = escape_sqlite_like_literal(source);
        let pattern = format!("{escaped}#%");
        conn.execute(
            "DELETE FROM symbol_index WHERE chunk_id LIKE ?1 ESCAPE '\\'",
            rusqlite::params![pattern],
        )?;
        conn.execute(
            "DELETE FROM reference_index WHERE chunk_id LIKE ?1 ESCAPE '\\'",
            rusqlite::params![pattern],
        )?;
        conn.execute(
            "DELETE FROM chunk_vectors WHERE chunk_id LIKE ?1 ESCAPE '\\'",
            rusqlite::params![pattern],
        )?;
        conn.execute(
            "DELETE FROM workspace_chunks WHERE source = ?1",
            rusqlite::params![source],
        )?;
        Ok(())
    }

    /// Delete symbol index entries by chunk prefix using an existing connection (no mutex).
    pub(crate) fn delete_symbol_index_by_chunk_prefix_conn(
        conn: &Connection,
        source: &str,
    ) -> Result<(), RagDbError> {
        let escaped = escape_sqlite_like_literal(source);
        let pattern = format!("{escaped}#%");
        conn.execute(
            "DELETE FROM symbol_index WHERE chunk_id LIKE ?1 ESCAPE '\\'",
            rusqlite::params![pattern],
        )?;
        Ok(())
    }

    /// Delete reference index entries by chunk prefix using an existing connection (no mutex).
    pub(crate) fn delete_reference_index_by_chunk_prefix_conn(
        conn: &Connection,
        source: &str,
    ) -> Result<(), RagDbError> {
        let escaped = escape_sqlite_like_literal(source);
        let pattern = format!("{escaped}#%");
        conn.execute(
            "DELETE FROM reference_index WHERE chunk_id LIKE ?1 ESCAPE '\\'",
            rusqlite::params![pattern],
        )?;
        Ok(())
    }

    /// Delete summary by source using an existing connection (no mutex).
    pub(crate) fn delete_summary_by_source_conn(
        conn: &Connection,
        source: &str,
    ) -> Result<(), RagDbError> {
        conn.execute(
            "DELETE FROM file_summaries WHERE source = ?1",
            rusqlite::params![source],
        )?;
        conn.execute(
            "DELETE FROM summary_vectors WHERE source = ?1",
            rusqlite::params![source],
        )?;
        Ok(())
    }

    /// Upsert chunks batch using an existing connection (no mutex).
    pub(crate) fn upsert_chunks_batch_conn(
        conn: &Connection,
        rows: &[ChunkBatchRowRef<'_>],
        chunk_type: &str,
        source_type: &str,
        last_updated: Option<u64>,
    ) -> Result<(), RagDbError> {
        if rows.is_empty() {
            return Ok(());
        }
        for (_, _, _, _, _, _, _, _, emb) in rows.iter() {
            if let Some(e) = emb {
                if e.len() != RAG_EMBED_DIM {
                    return Err(RagDbError::EmbedDim(e.len()));
                }
            }
        }
        let last_ts: Option<i64> = last_updated.map(|u| u as i64);
        let mut chunk_stmt = conn.prepare_cached(
            r#"
            INSERT OR REPLACE INTO workspace_chunks (id, text, source, summary, defines, imports, type, name, calls_metadata, last_updated, chunk_type, source_type)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
        )?;
        let mut vec_stmt = conn.prepare_cached(
            "INSERT OR REPLACE INTO chunk_vectors (chunk_id, embedding) VALUES (?1, ?2)",
        )?;
        for (id, text, source, defines, imports, type_, name, calls_metadata, embedding) in rows {
            chunk_stmt.execute(rusqlite::params![
                id,
                text,
                source,
                "",
                defines,
                imports,
                type_,
                name,
                calls_metadata,
                last_ts,
                chunk_type,
                source_type,
            ])?;
            if let Some(emb) = embedding {
                vec_stmt.execute(rusqlite::params![id, embedding_to_blob(emb).as_slice()])?;
            }
        }
        Ok(())
    }

    /// Batch insert symbol index pairs using an existing connection (no mutex).
    pub(crate) fn batch_insert_symbol_index_conn(
        conn: &Connection,
        pairs: &[(&str, &str)],
    ) -> Result<(), RagDbError> {
        if pairs.is_empty() {
            return Ok(());
        }
        let mut stmt =
            conn.prepare_cached("INSERT INTO symbol_index (symbol, chunk_id) VALUES (?1, ?2)")?;
        for (symbol, chunk_id) in pairs {
            stmt.execute(rusqlite::params![symbol, chunk_id])?;
        }
        Ok(())
    }

    /// Batch insert reference index pairs using an existing connection (no mutex).
    pub(crate) fn batch_insert_reference_index_conn(
        conn: &Connection,
        pairs: &[(&str, &str)],
    ) -> Result<(), RagDbError> {
        if pairs.is_empty() {
            return Ok(());
        }
        let mut stmt =
            conn.prepare_cached("INSERT INTO reference_index (symbol, chunk_id) VALUES (?1, ?2)")?;
        for (symbol, chunk_id) in pairs {
            stmt.execute(rusqlite::params![symbol, chunk_id])?;
        }
        Ok(())
    }

    /// Upsert summary using an existing connection (no mutex).
    pub(crate) fn upsert_summary_conn(
        conn: &Connection,
        source: &str,
        summary: &str,
        embedding: Option<&[f32]>,
    ) -> Result<(), RagDbError> {
        if let Some(emb) = embedding {
            if emb.len() != RAG_EMBED_DIM {
                return Err(RagDbError::EmbedDim(emb.len()));
            }
        }
        let embedding_blob: Option<Vec<u8>> =
            embedding.map(|e| e.iter().flat_map(|f| f.to_le_bytes()).collect());
        conn.execute(
            "INSERT OR REPLACE INTO file_summaries (source, summary) VALUES (?1, ?2)",
            rusqlite::params![source, summary],
        )?;
        if let Some(ref blob) = embedding_blob {
            conn.execute(
                "INSERT OR REPLACE INTO summary_vectors (source, embedding) VALUES (?1, ?2)",
                rusqlite::params![source, blob.as_slice()],
            )?;
        }
        Ok(())
    }

    /// Search summary vectors (KNN); return source paths.
    pub fn search_summaries_knn(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<String>, RagDbError> {
        if query_embedding.len() != RAG_EMBED_DIM {
            return Err(RagDbError::EmbedDim(query_embedding.len()));
        }
        self.with_read_conn(|conn| Self::search_summaries_knn_impl(conn, query_embedding, limit))
    }

    /// search_summaries_knn_impl.
    fn search_summaries_knn_impl(
        conn: &Connection,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<String>, RagDbError> {
        let blob: Vec<u8> = query_embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let mut stmt = conn
            .prepare("SELECT source FROM summary_vectors WHERE embedding MATCH ?1 AND k = ?2")
            .map_err(map_sqlite_vec_error)?;
        let rows = stmt
            .query_map(rusqlite::params![blob.as_slice(), limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(map_sqlite_vec_error)?;
        Ok(rows.filter_map(Result::ok).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory DB per test: no file path, no cross-test locking. Each test gets an isolated DB.
    const TEST_DB_MEMORY: &str = ":memory:";

    #[test]
    /// search_fts_empty_query_returns_empty.
    fn search_fts_empty_query_returns_empty() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        let result = db.search_fts("", 10);
        // FTS5 with empty MATCH returns no rows or error; we accept either.
        if let Ok(rows) = result {
            assert!(rows.is_empty());
        }
    }

    #[test]
    /// schema_creates_reference_index_and_symbol_index.
    fn schema_creates_reference_index_and_symbol_index() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        db.insert_symbol_index("DefinedFunc", "src.py#0")
            .expect("insert symbol");
        db.insert_reference_index("DefinedFunc", "other.py#1")
            .expect("insert ref");
        let def_ids = db.get_chunk_ids_for_symbol("DefinedFunc").expect("get def");
        let ref_ids = db
            .get_chunk_ids_referencing_symbol("DefinedFunc")
            .expect("get ref");
        assert_eq!(def_ids, vec!["src.py#0"]);
        assert_eq!(ref_ids, vec!["other.py#1"]);
    }

    #[test]
    /// symbol_index_definition_only_refs_in_reference_index.
    fn symbol_index_definition_only_refs_in_reference_index() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        db.insert_symbol_index("Foo", "def.py#0").expect("symbol");
        db.insert_reference_index("Foo", "caller.py#0")
            .expect("ref");
        assert_eq!(
            db.get_chunk_ids_for_symbol("Foo").unwrap(),
            vec!["def.py#0"]
        );
        assert_eq!(
            db.get_chunk_ids_referencing_symbol("Foo").unwrap(),
            vec!["caller.py#0"]
        );
    }

    #[test]
    /// delete_reference_index_by_chunk_prefix.
    fn delete_reference_index_by_chunk_prefix() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        db.insert_reference_index("X", "p/a.py#0").expect("ref");
        db.insert_reference_index("X", "p/a.py#1").expect("ref");
        assert_eq!(db.get_chunk_ids_referencing_symbol("X").unwrap().len(), 2);
        db.delete_reference_index_by_chunk_prefix("p/a.py")
            .expect("delete");
        assert!(db.get_chunk_ids_referencing_symbol("X").unwrap().is_empty());
    }

    #[test]
    fn escape_sqlite_like_literal_escapes_wildcards() {
        assert_eq!(escape_sqlite_like_literal("a%b_c\\"), "a\\%b\\_c\\\\");
    }

    #[test]
    fn fts5_prepare_match_query_quotes_tokens_so_colon_is_not_column() {
        assert!(fts5_prepare_match_query("").is_empty());
        assert_eq!(fts5_prepare_match_query("x"), "\"x\"");
        assert_eq!(
            fts5_prepare_match_query("error: smoke"),
            "\"error:\" OR \"smoke\""
        );
        assert_eq!(
            fts5_prepare_match_query("lessons learned"),
            "\"lessons\" OR \"learned\""
        );
        assert_eq!(fts5_prepare_match_query("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn upsert_chunk_none_embedding_removes_vector_row() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        let emb = vec![0.1f32; RAG_EMBED_DIM];
        db.upsert_chunk(
            "g.rs#0",
            "fn g() {}",
            "g.rs",
            "",
            "[]",
            "[]",
            "function",
            "g",
            "[]",
            Some(&emb),
            None,
            "code",
            "codebase",
        )
        .expect("upsert with vec");
        assert!(db.has_any_chunk_vectors().expect("has vec"));
        db.upsert_chunk(
            "g.rs#0",
            "fn g() {}",
            "g.rs",
            "",
            "[]",
            "[]",
            "function",
            "g",
            "[]",
            None,
            None,
            "code",
            "codebase",
        )
        .expect("upsert without vec");
        assert!(
            !db.has_any_chunk_vectors().expect("vector cleared"),
            "embedding None must delete chunk_vectors row"
        );
    }

    #[test]
    /// upsert_chunk_stores_calls_metadata.
    fn upsert_chunk_stores_calls_metadata() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        let emb = vec![0.0f32; RAG_EMBED_DIM];
        db.upsert_chunk(
            "f.py#0",
            "def f(): g()",
            "f.py",
            "",
            "[\"f\"]",
            "[]",
            "function",
            "f",
            "[\"g\"]",
            Some(&emb),
            None,
            "code",
            "codebase",
        )
        .expect("upsert");
        let rows = db.get_chunks_by_ids(&["f.py#0".to_string()]).expect("get");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].calls, "[\"g\"]");
    }

    #[test]
    /// delete_chunks_by_source_removes_reference_index_rows.
    fn delete_chunks_by_source_removes_reference_index_rows() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        let emb = vec![0.0f32; RAG_EMBED_DIM];
        db.upsert_chunk(
            "src/a.py#0",
            "def f(): pass",
            "src/a.py",
            "",
            "[\"f\"]",
            "[]",
            "function",
            "f",
            "[]",
            Some(&emb),
            None,
            "code",
            "codebase",
        )
        .expect("upsert");
        db.insert_symbol_index("f", "src/a.py#0").expect("symbol");
        db.insert_reference_index("f", "src/a.py#0").expect("ref");
        assert_eq!(db.get_chunk_ids_referencing_symbol("f").unwrap().len(), 1);
        assert!(db.has_any_chunk_vectors().expect("has_any before delete"));
        db.delete_chunks_by_source("src/a.py").expect("delete");
        assert!(db.get_chunk_ids_referencing_symbol("f").unwrap().is_empty());
        assert!(db.get_chunk_ids_for_symbol("f").unwrap().is_empty());
        assert!(db
            .get_chunks_by_ids(&["src/a.py#0".to_string()])
            .unwrap()
            .is_empty());
        assert!(
            !db.has_any_chunk_vectors().expect("has_any after delete"),
            "delete_chunks_by_source must remove chunk_vectors rows"
        );
    }

    #[test]
    /// vector_knn_with_768d_returns_results: Vector KNN accepts 768-d embedding and returns results.
    fn vector_knn_with_768d_returns_results() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        let embedding: Vec<f32> = (0..RAG_EMBED_DIM).map(|i| (i as f32) * 0.001).collect();
        db.upsert_chunk(
            "test.rs#0",
            "fn main() { }",
            "test.rs",
            "",
            "[\"main\"]",
            "[]",
            "function",
            "main",
            "[]",
            Some(&embedding),
            None,
            "code",
            "codebase",
        )
        .expect("upsert");
        let results = db
            .search_vector_knn(&embedding, 5, None)
            .expect("search_vector_knn");
        assert!(
            !results.is_empty(),
            "search_vector_knn with 768-d should return at least the inserted chunk"
        );
        assert!(results.iter().any(|(id, _)| id == "test.rs#0"));
    }

    #[test]
    /// rrf_merge_produces_deterministic_order.
    fn rrf_merge_produces_deterministic_order() {
        let fts = vec![
            ("a#0".to_string(), 0.5),
            ("a#1".to_string(), 0.3),
            ("b#0".to_string(), 0.1),
        ];
        let vec = vec![
            ("b#0".to_string(), 0.4),
            ("a#0".to_string(), 0.2),
            ("c#0".to_string(), 0.1),
        ];
        let out1 = RagDb::rrf_merge(fts.clone(), vec.clone(), 5);
        let out2 = RagDb::rrf_merge(fts, vec, 5);
        assert_eq!(out1, out2);
        assert!(out1.len() <= 5);
        assert!(out1.contains(&"a#0".to_string()));
    }

    #[test]
    /// error_message_indicates_embed_dim_maps_dimension_like_messages.
    fn error_message_indicates_embed_dim_maps_dimension_like_messages() {
        assert!(error_message_indicates_embed_dim("expected 0"));
        assert!(error_message_indicates_embed_dim(
            "embedding dimension mismatch"
        ));
        assert!(error_message_indicates_embed_dim("EMBEDDING DIMENSION"));
        assert!(error_message_indicates_embed_dim("dimension mismatch"));
        assert!(!error_message_indicates_embed_dim("other error"));
    }

    #[test]
    /// get_chunks_by_ids_empty_returns_empty.
    fn get_chunks_by_ids_empty_returns_empty() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        let rows = db.get_chunks_by_ids(&[]).expect("get");
        assert!(rows.is_empty());
    }

    #[test]
    fn insert_golden_pattern_and_search_golden_patterns_knn_roundtrip() {
        let db = RagDb::open(TEST_DB_MEMORY).expect("open");
        let embedding: Vec<f32> = (0..RAG_EMBED_DIM).map(|i| (i as f32) * 0.001).collect();
        let pattern_id = "test_pattern::12345";
        let name = "Test pattern";
        let code = "fn example() { }";
        let language = "rust";

        db.insert_golden_pattern(pattern_id, name, code, language, &embedding)
            .expect("insert_golden_pattern");

        let results = db
            .search_golden_patterns_knn(&embedding, 1)
            .expect("search_golden_patterns_knn");
        assert_eq!(results.len(), 1, "expect one result");
        assert_eq!(results[0].0, pattern_id);
        assert!(
            results[0].1 < 0.01,
            "distance to self should be ~0, got {}",
            results[0].1
        );
    }
}
