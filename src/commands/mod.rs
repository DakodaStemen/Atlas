pub(crate) mod audit;
pub(crate) mod data;
pub(crate) mod ingest;
pub(crate) mod ops;
pub(crate) mod query;

pub(crate) use audit::{run_audit, run_complexity, run_review_lessons};
pub(crate) use data::{
    run_bin_to_jsonl, run_prune_manifest_stale, run_prune_orphans, run_trim_training,
};
pub(crate) use ingest::{run_ingest, run_ingest_from_jsonl, run_ingest_web};
pub(crate) use ops::{
    run_background_janitor, run_build_web_sources, run_chaos, run_janitor_cycle,
    run_merge_unsloth_jsonl, run_sample_training,
};
pub(crate) use query::{run_count_chunks, run_query, run_stress_query, run_verify_retrieval};

use std::path::Path;

/// Load Nomic embedder from ONNX + tokenizer; stub if ORT unset or missing files.
pub(crate) fn load_nomic_embedder(onnx_path: &Path) -> rag_mcp::rag::RagEmbedder {
    #[cfg(feature = "onnx")]
    if std::env::var("ORT_DYLIB_PATH").is_err() {
        tracing::info!("ORT_DYLIB_PATH not set; semantic search disabled.");
        return rag_mcp::rag::RagEmbedder::stub();
    }

    let tokenizer_path = onnx_path.with_file_name("tokenizer.json");

    if !onnx_path.exists() || !tokenizer_path.exists() {
        tracing::info!(
            "No Nomic ONNX/tokenizer at {:?}; semantic search disabled",
            onnx_path
        );
        return rag_mcp::rag::RagEmbedder::stub();
    }

    let onnx = onnx_path.to_path_buf();
    let tok = tokenizer_path.to_path_buf();
    match std::panic::catch_unwind(|| rag_mcp::rag::RagEmbedder::new(&onnx, &tok)) {
        Ok(Ok(e)) => {
            tracing::info!("Nomic embedder loaded (768-d). Semantic search active.");
            e
        }
        Ok(Err(e)) => {
            tracing::warn!(
                "Could not load Nomic embedder: {}; semantic search disabled",
                e
            );
            rag_mcp::rag::RagEmbedder::stub()
        }
        Err(_) => {
            tracing::warn!(
                "Nomic embedder init panicked (e.g. ONNX Runtime version mismatch); semantic search disabled. Use ONNX Runtime >= 1.23 or unset ORT_DYLIB_PATH."
            );
            rag_mcp::rag::RagEmbedder::stub()
        }
    }
}

/// Load cross-encoder reranker from ONNX + tokenizer; stub if ORT unset or missing files.
pub(crate) fn load_reranker(onnx_path: &Path) -> rag_mcp::rerank::Reranker {
    #[cfg(feature = "onnx")]
    if std::env::var("ORT_DYLIB_PATH").is_err() {
        tracing::warn!(
            "ORT_DYLIB_PATH not set; reranker disabled. Set ORT_DYLIB_PATH to enable two-stage cross-encoder reranking."
        );
        return rag_mcp::rerank::Reranker::stub();
    }

    let tokenizer_path = onnx_path.with_file_name("reranker-tokenizer.json");
    if !onnx_path.exists() || !tokenizer_path.exists() {
        tracing::warn!(
            "No reranker.onnx at {:?}; two-stage reranking disabled. Place reranker.onnx and reranker-tokenizer.json in data/models/ to enable.",
            onnx_path
        );
        return rag_mcp::rerank::Reranker::stub();
    }
    #[cfg(feature = "onnx")]
    let onnx = onnx_path.to_path_buf();
    #[cfg(feature = "onnx")]
    let tok = tokenizer_path.to_path_buf();
    #[cfg(feature = "onnx")]
    match std::panic::catch_unwind(|| rag_mcp::rerank::Reranker::new(&onnx, &tok)) {
        #[cfg(feature = "onnx")]
        Ok(Ok(r)) => {
            tracing::info!("Loaded Cross-Encoder reranker");
            r
        }
        #[cfg(feature = "onnx")]
        Ok(Err(e)) => {
            tracing::warn!(
                "Could not load reranker: {}; two-stage reranking disabled",
                e
            );
            rag_mcp::rerank::Reranker::stub()
        }
        #[cfg(feature = "onnx")]
        Err(_) => {
            tracing::warn!(
                "Reranker init panicked (e.g. ONNX Runtime version mismatch); two-stage reranking disabled. Use ONNX Runtime >= 1.23 or unset ORT_DYLIB_PATH."
            );
            rag_mcp::rerank::Reranker::stub()
        }
    }
    #[cfg(not(feature = "onnx"))]
    match rag_mcp::rerank::Reranker::new(onnx_path, &tokenizer_path) {
        Ok(r) => {
            tracing::info!("Loaded Cross-Encoder reranker");
            r
        }
        Err(e) => {
            tracing::warn!(
                "Could not load reranker: {}; two-stage reranking disabled",
                e
            );
            rag_mcp::rerank::Reranker::stub()
        }
    }
}
