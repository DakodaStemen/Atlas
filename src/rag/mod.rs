//! RAG module: workspace chunk store, Nomic embeddings (768-d), retrieval, ingest.
//! Parity with Python rag_mcp_server + lancedb_store + ingest_workspace.
//! Data flow: ingest → db → store → handler.
//!
//! Submodules: `chunking` (split by language), `db` (SQLite FTS+vector), `embedding` (Nomic),
//! `ingest` (walk + chunk + embed), `store` (hybrid search, MMR, related code), `handler` (MCP tools),
//! `domain_classifier`, `dataset_collector`, `symbols` (Rust/Python).

pub mod chunking;
pub mod cli_helpers;
pub mod dataset_collector;
pub mod db;
pub mod domain_classifier;
pub mod embedding;
pub mod handler;
pub mod ingest;
#[cfg(test)]
mod integration_tests;
pub mod path_filter;
pub mod store;
pub mod symbols;
pub mod xml;

/// Re-exports used by `main` and MCP entrypoints: store/db/embedder/handler, ingest fns,
/// dataset collector, and sanitize_shell_output for log redaction. Other types stay in submodules.
pub use dataset_collector::{is_low_value_training_row, DatasetCollector, DatasetCollectorGuard};
pub use db::RagDb;
pub use embedding::RagEmbedder;
pub use handler::{sanitize_shell_output, verify_ui_integrity_check, AgenticHandler, ManagedLoop};
pub use ingest::{ingest_directory, ingest_directory_parallel, ingest_from_jsonl};
pub use store::RagStore;
