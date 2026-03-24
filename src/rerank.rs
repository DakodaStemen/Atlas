//! Two-stage reranking: Cross-Encoder (e.g. ms-marco-MiniLM-L-6-v2) over top-K recall.
//! Model path is resolved by `Config`: prefer `data_dir/models/reranker.onnx`, then
//! `data_dir/reranker.onnx`, else default `data_dir/models/reranker.onnx`. Tokenizer
//! at `reranker-tokenizer.json` in the same dir as the ONNX. Requires feature "onnx".
//!
//! Scores are used for **ranking only** (no threshold). If you swap models, ensure they produce
//! comparable relevance ordering; score scale may differ. See docs/RAG_OPERATIONS.md.
//! Max sequence length per (query, passage) pair is 512 tokens (`RERANKER_MAX_LEN`). Tokenizer encodes (query, passage) pairs; sequences longer than max are truncated, shorter are padded with `PAD_ID` (0) for batch input.
//! Stub: `stub()` returns an instance with `is_available()` false; `predict_batch` returns zeros (no onnx) or `Err(NotLoaded)` (onnx).

use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use thiserror::Error;

/// Max token length per (query, passage) pair. Sequences longer than this are truncated.
const RERANKER_MAX_LEN: usize = 512;

/// Incremented each time the cross-encoder reranker successfully scores a batch.
pub static RERANK_HITS: AtomicU64 = AtomicU64::new(0);
/// Incremented each time reranking falls back to RRF order (stub or model unavailable/error).
pub static RERANK_MISSES: AtomicU64 = AtomicU64::new(0);

/// Returns (hits, misses) since process start.
pub fn rerank_stats() -> (u64, u64) {
    (
        RERANK_HITS.load(Ordering::Relaxed),
        RERANK_MISSES.load(Ordering::Relaxed),
    )
}

/// Padding token id and attention mask value for sequences shorter than max_len. Used in tokenize_and_pad_pairs for both input_ids and attention_mask.
const PAD_ID: i64 = 0;

/// Tokenize (query, passage) pairs and pad to max_len; returns (input_ids, attention_mask, seq_len) for ONNX.
/// Pairs longer than max_len are truncated to the first max_len tokens (no mid-sequence truncation).
#[cfg(feature = "onnx")]
/// tokenize_and_pad_pairs.
fn tokenize_and_pad_pairs(
    tokenizer: &tokenizers::Tokenizer,
    pairs: &[(String, String)],
    max_len: usize,
) -> Result<(Vec<i64>, Vec<i64>, usize), RerankError> {
    let mut all_input_ids: Vec<i64> = Vec::new();
    let mut all_attention: Vec<i64> = Vec::new();
    let mut seq_len = 0usize;
    for (q, p) in pairs {
        let enc = tokenizer
            .encode((q.as_str(), p.as_str()), true)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let mut ids: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
        let mut attention = vec![1i64; ids.len()];
        if ids.len() > max_len {
            ids.truncate(max_len);
            attention.truncate(max_len);
        }
        while ids.len() < max_len {
            ids.push(PAD_ID); // pad to max_len for batch
            attention.push(PAD_ID);
        }
        seq_len = ids.len();
        all_input_ids.extend(ids);
        all_attention.extend(attention);
    }
    Ok((all_input_ids, all_attention, seq_len))
}

/// Errors from reranker: Ort (onnx), Io (tokenizer), NotLoaded (stub or unload), LockPoisoned (RwLock).
#[derive(Error, Debug)]
/// Errors: Ort (onnx), Io (tokenizer), NotLoaded (stub/unload), LockPoisoned.
pub enum RerankError {
    #[cfg(feature = "onnx")]
    #[error("ort: {0}")]
    Ort(#[from] ort::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("model not loaded")]
    NotLoaded,
    #[error("lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Cross-encoder reranker with a pool of ONNX sessions for concurrent inference.
#[cfg_attr(not(feature = "onnx"), allow(dead_code))]
/// Cross-encoder reranker; predict_batch scores (query, passage) pairs for ordering.
pub struct Reranker {
    /// Pool of sessions for concurrent inference via round-robin selection.
    #[cfg(feature = "onnx")]
    sessions: Vec<Mutex<Option<ort::session::Session>>>,
    #[cfg(not(feature = "onnx"))]
    sessions: Vec<Mutex<Option<()>>>,
    /// Round-robin counter for session selection.
    next_session: AtomicUsize,
    #[cfg(feature = "onnx")]
    tokenizer: Option<tokenizers::Tokenizer>,
    #[cfg(not(feature = "onnx"))]
    tokenizer: Option<()>,
    max_len: usize,
}

impl Reranker {
    /// Builds a reranker with a pool of ONNX sessions for concurrent inference.
    /// Pool size is controlled by `RERANKER_POOL_SIZE` env var (default: min(4, max(1, num_cpus/2))).
    #[cfg(feature = "onnx")]
    /// Build reranker from ONNX and tokenizer paths; loads model pool.
    pub fn new(onnx_path: &Path, tokenizer_path: &Path) -> Result<Self, RerankError> {
        use ort::session::builder::GraphOptimizationLevel;
        let threads = std::cmp::max(1, num_cpus::get());
        let pool_size = std::env::var("RERANKER_POOL_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| (num_cpus::get() / 2).clamp(1, 4));
        let mut sessions = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            #[cfg(feature = "cuda")]
            let session = ort::session::Session::builder()?
                .with_execution_providers([ort::ep::CUDA::default().build()])?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(threads)?
                .commit_from_file(onnx_path)?;
            #[cfg(not(feature = "cuda"))]
            let session = ort::session::Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(threads)?
                .commit_from_file(onnx_path)?;
            sessions.push(Mutex::new(Some(session)));
        }
        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(Self {
            sessions,
            next_session: AtomicUsize::new(0),
            tokenizer: Some(tokenizer),
            max_len: RERANKER_MAX_LEN,
        })
    }

    #[cfg(not(feature = "onnx"))]
    /// When the onnx feature is disabled, always returns Err(RerankError::NotLoaded).
    pub fn new(_onnx_path: &Path, _tokenizer_path: &Path) -> Result<Self, RerankError> {
        Err(RerankError::NotLoaded)
    }
    /// Stub instance (is_available false, predict_batch returns zeros or NotLoaded).
    pub fn stub() -> Self {
        Self {
            sessions: vec![Mutex::new(None)],
            next_session: AtomicUsize::new(0),
            tokenizer: None,
            max_len: RERANKER_MAX_LEN,
        }
    }

    /// True if session pool and tokenizer are loaded. Stub returns false.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "onnx")]
        return self.sessions[0]
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false);
        #[cfg(not(feature = "onnx"))]
        return false;
    }

    /// Batch score for many (query, passage) pairs. Runs in one session run when possible.
    /// ort Session::run takes &mut self, so we hold a write lock for inference.
    pub fn predict_batch(&self, pairs: &[(String, String)]) -> Result<Vec<f32>, RerankError> {
        #[cfg(not(feature = "onnx"))]
        return Ok(vec![0.0; pairs.len()]);

        #[cfg(feature = "onnx")]
        {
            use ort::session::SessionInputValue;
            use ort::value::Value;

            if pairs.is_empty() {
                return Ok(vec![]);
            }
            let idx = self.next_session.fetch_add(1, Ordering::Relaxed) % self.sessions.len();
            let mut session_guard = self.sessions[idx]
                .lock()
                .map_err(|e| RerankError::LockPoisoned(e.to_string()))?;
            let session = session_guard.as_mut().ok_or(RerankError::NotLoaded)?;
            let tokenizer = self.tokenizer.as_ref().ok_or(RerankError::NotLoaded)?;

            let (all_input_ids, all_attention, seq_len) =
                tokenize_and_pad_pairs(tokenizer, pairs, self.max_len)?;

            let batch_size = pairs.len();
            let shape_ids = [batch_size, seq_len];
            let shape_att = [batch_size, seq_len];
            let input_ids_value = Value::from_array((shape_ids, all_input_ids))?;
            let attention_value = Value::from_array((shape_att, all_attention))?;
            let inputs: Vec<(&str, SessionInputValue<'_>)> = vec![
                ("input_ids", input_ids_value.into()),
                ("attention_mask", attention_value.into()),
            ];
            let outputs = session.run(inputs)?;
            let (out_shape, logits) = outputs[0].try_extract_tensor::<f32>()?;
            let dims: Vec<usize> = out_shape.as_ref().iter().map(|&d| d as usize).collect();
            let scores: Vec<f32> = if dims.len() == 2 && dims[0] == batch_size {
                if dims[1] >= 1 {
                    (0..batch_size).map(|i| logits[i * dims[1]]).collect()
                } else {
                    vec![0.0; batch_size]
                }
            } else if dims.len() == 1 && dims[0] == batch_size {
                logits.to_vec()
            } else {
                let n = dims.iter().product::<usize>().min(batch_size);
                logits.iter().take(n).copied().collect()
            };
            Ok(scores)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// stub() yields is_available() false.
    fn stub_is_not_available() {
        let r = Reranker::stub();
        assert!(!r.is_available());
    }

    #[cfg(not(feature = "onnx"))]
    #[test]
    /// Without onnx, stub predict_batch returns zeros.
    fn stub_predict_batch_returns_zeros() {
        let r = Reranker::stub();
        let pairs = vec![
            ("q1".to_string(), "p1".to_string()),
            ("q2".to_string(), "p2".to_string()),
        ];
        let scores = r.predict_batch(&pairs).unwrap();
        assert_eq!(scores.len(), 2);
        assert_eq!(scores[0], 0.0);
        assert_eq!(scores[1], 0.0);
    }

    #[cfg(feature = "onnx")]
    #[test]
    /// With onnx, stub predict_batch returns NotLoaded error.
    fn stub_predict_batch_returns_not_loaded() {
        let r = Reranker::stub();
        let pairs = vec![("q".to_string(), "p".to_string())];
        let res = r.predict_batch(&pairs);
        assert!(res.is_err());
    }

    #[test]
    /// Empty pairs yields empty scores.
    fn stub_predict_batch_empty_returns_empty() {
        let r = Reranker::stub();
        let scores = r.predict_batch(&[]).unwrap();
        assert!(scores.is_empty());
    }
}
