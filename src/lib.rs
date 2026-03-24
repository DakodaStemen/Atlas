//! Library surface for rag-mcp (Monolith: RAG + tools).
//! Crate layout: `config` (paths, env), `rag` (ingest, db, store, handler), `rerank`, `tools` (web), `ui` (theme).
//! See [`tools`] for MCP tool backends (web fetch).

/// Config: paths, env, allowed roots, model paths, web sources.
pub mod config;
/// Lock-free in-process metrics: histograms, counters (AtomicU64).
pub mod metrics;
/// Process spawning utilities (Sync and Async).
pub mod process_utils;
/// RAG: ingest, db, store, handler, chunking, embedding, symbols.
pub mod rag;
/// Cross-encoder reranking for hybrid search.
pub mod rerank;
/// MCP tool backends (e.g. web fetch).
pub mod tools;
/// UI: design tokens and theme (CLI colors, typography).
pub mod ui;

/// Reranker for cross-encoder reranking. Use [`rerank::Reranker::predict_batch`] for (query, doc) pairs.
///
/// **Why re-export only Reranker:** RagStore and the MCP handler need the Reranker type for hybrid
/// search (rerank FTS + vector candidates). Re-exporting at crate root lets them use `crate::Reranker`
/// without `use crate::rerank::Reranker`. Config, rag, and tools are used via their modules from main
/// and do not need crate-level re-exports.
pub use crate::rerank::Reranker;
