//! Ingestion and vector-store provider traits and default implementations.
//! Used by AgenticHandler and process_ingestion (and tests).

use crate::rag::chunking::Chunk;
use crate::rag::db::RagDb;
use crate::rag::embedding::RagEmbedder;
use anyhow::Result as AnyhowResult;
use std::path::Path;
use std::sync::Arc;

/// Provider for reading file/content (abstraction for ingestion). Used by process_ingestion and tests.
pub trait IngestionProvider: Send + Sync {
    /// Read content at path. Returns anyhow error on I/O failure.
    fn read_content(&self, path: &Path) -> AnyhowResult<String>;
}

/// Provider for vector/store persistence (abstraction for DB operations). Used by process_ingestion and tests.
pub trait VectorStoreProvider: Send + Sync {
    /// Remove all chunks and summary for the given source.
    fn delete_by_source(&self, source: &str) -> AnyhowResult<()>;
    /// Persist chunks with optional embeddings. Caller should call delete_by_source first when replacing.
    fn save_chunks(
        &self,
        source: &str,
        chunks: &[Chunk],
        embeddings: Option<&[Vec<f32>]>,
    ) -> AnyhowResult<()>;
}

/// Default ingestion: read from filesystem.
#[derive(Clone, Debug, Default)]
pub struct DefaultIngestion;

impl IngestionProvider for DefaultIngestion {
    fn read_content(&self, path: &Path) -> AnyhowResult<String> {
        let s = std::fs::read_to_string(path)?;
        Ok(s.trim().to_string())
    }
}

/// Default storage: write to RagDb with optional embeddings via RagEmbedder.
#[derive(Clone)]
pub struct DefaultStorage {
    db: Arc<RagDb>,
    embedder: Arc<RagEmbedder>,
}

impl DefaultStorage {
    pub fn new(db: Arc<RagDb>, embedder: Arc<RagEmbedder>) -> Self {
        Self { db, embedder }
    }
}

impl VectorStoreProvider for DefaultStorage {
    fn delete_by_source(&self, source: &str) -> AnyhowResult<()> {
        self.db.delete_chunks_by_source(source)?;
        self.db.delete_symbol_index_by_chunk_prefix(source)?;
        self.db.delete_reference_index_by_chunk_prefix(source)?;
        self.db.delete_summary_by_source(source)?;
        Ok(())
    }

    fn save_chunks(
        &self,
        source: &str,
        chunks: &[Chunk],
        embeddings: Option<&[Vec<f32>]>,
    ) -> AnyhowResult<()> {
        // imports_json is always "[]" here: Chunk doesn't carry file-level import data.
        // The full ingest pipeline (ingest.rs::ingest_file) extracts imports via tree-sitter
        // and passes them per-file. DefaultStorage is used by the fast refresh path
        // (process_ingestion / refresh_file_index), which skips symbol extraction by design.
        // For full import indexing, use `rag-mcp ingest` or `ingest_directory`.
        let imports_json = "[]";
        // last_updated is None here: the refresh path does not track file modification times.
        // The full ingest pipeline in ingest.rs sets this from the file's mtime.
        let last_updated = None::<u64>;
        for (i, c) in chunks.iter().enumerate() {
            let id = format!("{}#{}", source, i);
            let defines_json =
                serde_json::to_string(&c.defines).unwrap_or_else(|_| "[]".to_string());
            let calls_json = serde_json::to_string(&c.calls).unwrap_or_else(|_| "[]".to_string());
            let emb = embeddings.and_then(|v| v.get(i)).map(|e| e.as_slice());
            self.db.upsert_chunk(
                &id,
                &c.text,
                source,
                "",
                &defines_json,
                imports_json,
                &c.type_,
                &c.name,
                &calls_json,
                emb,
                last_updated,
                "code",
                "codebase",
            )?;
        }
        let ids: Vec<String> = (0..chunks.len())
            .map(|i| format!("{}#{}", source, i))
            .collect();
        let mut sym_pairs: Vec<(&str, &str)> = Vec::new();
        let mut ref_pairs: Vec<(&str, &str)> = Vec::new();
        for (i, c) in chunks.iter().enumerate() {
            let id = ids[i].as_str();
            for sym in &c.defines {
                sym_pairs.push((sym.as_str(), id));
            }
            for sym in &c.calls {
                ref_pairs.push((sym.as_str(), id));
            }
        }
        self.db.batch_insert_symbol_index(&sym_pairs)?;
        self.db.batch_insert_reference_index(&ref_pairs)?;
        let summary = if let Some(first) = chunks.first() {
            first.text.lines().next().unwrap_or("").to_string()
        } else {
            String::new()
        };
        let summary_emb = if self.embedder.is_available() && !summary.is_empty() {
            self.embedder.embed(&summary).ok()
        } else {
            None
        };
        self.db
            .upsert_summary(source, &summary, summary_emb.as_deref())?;
        Ok(())
    }
}
