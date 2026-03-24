//! Nomic-Embed-v1.5 (768-d) via ONNX (ort). No query/passage prefix; raw text only (Python parity).
//! Embedding dimension is 768 (`crate::rag::db::RAG_EMBED_DIM`). Model path is resolved by `Config::resolve_model_path`.
//! **mean_pool_3d:** last-hidden-state (batch, seq, 768) is mean-pooled over the sequence dimension to produce one 768-d vector per batch item.
//! Stub: when model is not loaded, `stub()` returns an instance with `is_available()` false and `embed` / `embed_batch` return `Err(NotLoaded)`.

use std::path::Path;
use std::sync::RwLock;
use thiserror::Error;

/// Default max token length per sequence for the embedder (512). Sequences longer than this are truncated. Shared with rerank.
pub(crate) const DEFAULT_MAX_LEN: usize = 512;
/// Max sequences in one ONNX forward pass for embed_batch. Reduces ONNX calls from O(chunks) to O(chunks/this).
pub(crate) const EMBED_BATCH_SIZE: usize = 16;

#[derive(Error, Debug)]
/// EmbedError.
pub enum EmbedError {
    #[cfg(feature = "onnx")]
    #[error("ort: {0}")]
    Ort(#[from] ort::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Embedder not loaded (stub or model path missing).
    #[error("model not loaded")]
    NotLoaded,
    /// Output dimension did not match expected (e.g. model vs RAG_EMBED_DIM).
    #[error("dimension mismatch")]
    DimMismatch,
    /// RwLock poisoned (e.g. a prior thread panicked while holding the lock).
    #[error("lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Nomic embedder. Fields: session (ONNX, read lock for inference), tokenizer, max_len (sequence cap).
#[cfg_attr(not(feature = "onnx"), allow(dead_code))]
/// RagEmbedder.
pub struct RagEmbedder {
    #[cfg(feature = "onnx")]
    session: RwLock<Option<ort::session::Session>>,
    #[cfg(not(feature = "onnx"))]
    session: RwLock<Option<()>>,
    #[cfg(feature = "onnx")]
    tokenizer: Option<tokenizers::Tokenizer>,
    #[cfg(not(feature = "onnx"))]
    tokenizer: Option<()>,
    max_len: usize,
}

/// Mean-pool over sequence dimension with attention mask (for Nomic embed).
/// If attention_mask is shorter than seq_len, missing positions are treated as mask=0 (excluded from pool).
/// No panic on length mismatch; uses `.get(i).copied().unwrap_or(0.0)` throughout.
/// Could use ndarray axis ops for shorter code; kept explicit for clarity.
#[cfg(feature = "onnx")]
/// mean_pool_3d.
fn mean_pool_3d(
    last_hidden: &ndarray::Array3<f32>,
    attention_mask: &[f32],
) -> ndarray::Array1<f32> {
    use ndarray::Array1;
    let (_, seq_len, hidden_size) = last_hidden.dim();
    let mut out = Array1::zeros(hidden_size);
    let mut sum_mask = 0.0f32;
    for i in 0..seq_len {
        let m = attention_mask.get(i).copied().unwrap_or(0.0);
        sum_mask += m;
        for j in 0..hidden_size {
            out[j] += last_hidden[[0, i, j]] * m;
        }
    }
    if sum_mask > 0.0 {
        for x in out.iter_mut() {
            *x /= sum_mask;
        }
    }
    out
}

/// Mean-pool over sequence dimension for each batch item. last_hidden shape (B, L, H), attention_masks rows (B, L). Returns one vector per batch index.
#[cfg(feature = "onnx")]
/// mean_pool_3d_batch.
fn mean_pool_3d_batch(
    last_hidden: &ndarray::Array3<f32>,
    attention_masks: &[Vec<f32>],
) -> Vec<ndarray::Array1<f32>> {
    use ndarray::Array1;
    let (batch_size, seq_len, hidden_size) = last_hidden.dim();
    let mut out = Vec::with_capacity(batch_size);
    for b in 0..batch_size {
        let mask = attention_masks.get(b).map(|v| v.as_slice()).unwrap_or(&[]);
        let mut vec = Array1::zeros(hidden_size);
        let mut sum_mask = 0.0f32;
        for i in 0..seq_len {
            let m = mask.get(i).copied().unwrap_or(0.0);
            sum_mask += m;
            for j in 0..hidden_size {
                vec[j] += last_hidden[[b, i, j]] * m;
            }
        }
        if sum_mask > 0.0 {
            for x in vec.iter_mut() {
                *x /= sum_mask;
            }
        }
        out.push(vec);
    }
    out
}

#[cfg(feature = "onnx")]
/// normalize.
fn normalize(v: &mut [f32]) {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}

impl RagEmbedder {
    #[cfg(feature = "onnx")]
    /// new.
    pub fn new(model_path: &Path, tokenizer_path: &Path) -> Result<Self, EmbedError> {
        #[cfg(feature = "cuda")]
        let session = ort::session::Session::builder()?
            .with_execution_providers([ort::ep::CUDA::default().build()])?
            .commit_from_file(model_path)?;
        #[cfg(not(feature = "cuda"))]
        let session = ort::session::Session::builder()?.commit_from_file(model_path)?;
        let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(Self {
            session: RwLock::new(Some(session)),
            tokenizer: Some(tokenizer),
            max_len: DEFAULT_MAX_LEN,
        })
    }

    #[cfg(not(feature = "onnx"))]
    /// new.
    pub fn new(_model_path: &Path, _tokenizer_path: &Path) -> Result<Self, EmbedError> {
        Err(EmbedError::NotLoaded)
    }
    /// stub.
    pub fn stub() -> Self {
        Self {
            session: RwLock::new(None),
            tokenizer: None,
            max_len: DEFAULT_MAX_LEN,
        }
    }

    /// True if model is loaded (session some). Stub returns false.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "onnx")]
        return self.session.read().map(|g| g.is_some()).unwrap_or(false);
        #[cfg(not(feature = "onnx"))]
        return false;
    }

    /// Token count for text (BPE). Returns None when tokenizer not loaded or onnx disabled. Used for token-based truncation.
    pub fn count_tokens(&self, text: &str) -> Option<usize> {
        #[cfg(feature = "onnx")]
        {
            let tok = self.tokenizer.as_ref()?;
            let enc = tok.encode(text, true).map_err(|_| ()).ok()?;
            Some(enc.get_ids().len())
        }
        #[cfg(not(feature = "onnx"))]
        {
            let _ = text;
            None
        }
    }

    #[cfg(feature = "onnx")]
    /// tokenize.
    fn tokenize(&self, text: &str) -> Result<(Vec<i64>, Vec<f32>), EmbedError> {
        let tok = self.tokenizer.as_ref().ok_or(EmbedError::NotLoaded)?;
        let enc = tok
            .encode(text, true)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let mut ids: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
        let mut attention = vec![1.0f32; ids.len()];
        if ids.len() > self.max_len {
            ids.truncate(self.max_len);
            attention.truncate(self.max_len);
        }
        while ids.len() < self.max_len {
            ids.push(0);
            attention.push(0.0);
        }
        Ok((ids, attention))
    }

    /// Embed one text (no prefix; Nomic parity).
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        #[cfg(not(feature = "onnx"))]
        let _ = text;
        #[cfg(not(feature = "onnx"))]
        return Err(EmbedError::NotLoaded);

        #[cfg(feature = "onnx")]
        {
            use ndarray::Array3;
            use ort::session::SessionInputValue;
            use ort::value::Value;

            let mut session_guard = self
                .session
                .write()
                .map_err(|e| EmbedError::LockPoisoned(e.to_string()))?;
            let session = session_guard.as_mut().ok_or(EmbedError::NotLoaded)?;
            let (input_ids, attention_mask) = self.tokenize(text)?;
            let len = input_ids.len();
            let shape = [1usize, len];
            let input_ids_value = Value::from_array((shape, input_ids))?;
            let attention_value = Value::from_array((shape, attention_mask.clone()))?;
            let inputs: Vec<(&str, SessionInputValue<'_>)> = vec![
                ("input_ids", input_ids_value.into()),
                ("attention_mask", attention_value.into()),
            ];
            let outputs = session.run(inputs)?;
            let (shape, data) = outputs[0].try_extract_tensor::<f32>()?;
            let dims: Vec<usize> = shape.as_ref().iter().map(|&d| d as usize).collect();
            if dims.len() != 3 {
                return Err(EmbedError::DimMismatch);
            }
            let last_hidden = Array3::from_shape_vec((dims[0], dims[1], dims[2]), data.to_vec())
                .map_err(|_| EmbedError::DimMismatch)?;
            let pooled = mean_pool_3d(&last_hidden, &attention_mask);
            let mut vec: Vec<f32> = pooled.iter().copied().collect();
            normalize(&mut vec);
            if vec.len() != crate::rag::db::RAG_EMBED_DIM {
                return Err(EmbedError::DimMismatch);
            }
            Ok(vec)
        }
    }
    /// embed_query.
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed(query)
    }
    /// Embed multiple texts. When ONNX is available, runs up to EMBED_BATCH_SIZE sequences per forward pass (true batch); otherwise falls back to sequential embed().
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        #[cfg(feature = "onnx")]
        {
            if self
                .session
                .read()
                .map_err(|e| EmbedError::LockPoisoned(e.to_string()))?
                .is_none()
            {
                return Err(EmbedError::NotLoaded);
            }
            let mut out = Vec::with_capacity(texts.len());
            let mut start = 0;
            while start < texts.len() {
                // Up to EMBED_BATCH_SIZE texts per ONNX forward pass (see constant).
                let batch_len = (texts.len() - start).min(EMBED_BATCH_SIZE);
                let batch_texts = &texts[start..start + batch_len];
                let mut input_ids_rows: Vec<Vec<i64>> = Vec::with_capacity(batch_len);
                let mut attention_rows: Vec<Vec<f32>> = Vec::with_capacity(batch_len);
                for t in batch_texts {
                    let (ids, attn) = self.tokenize(t)?;
                    input_ids_rows.push(ids);
                    attention_rows.push(attn);
                }
                let seq_len = input_ids_rows[0].len();
                let flat_ids: Vec<i64> = input_ids_rows.into_iter().flatten().collect();
                let flat_attn: Vec<f32> = attention_rows
                    .iter()
                    .flat_map(|v| v.iter().copied())
                    .collect();
                let shape = [batch_len, seq_len];
                let input_ids_value =
                    ort::value::Value::from_array((shape, flat_ids)).map_err(EmbedError::Ort)?;
                let attention_value =
                    ort::value::Value::from_array((shape, flat_attn)).map_err(EmbedError::Ort)?;
                let mut session_guard = self
                    .session
                    .write()
                    .map_err(|e| EmbedError::LockPoisoned(e.to_string()))?;
                let session = session_guard.as_mut().ok_or(EmbedError::NotLoaded)?;
                let inputs: Vec<(&str, ort::session::SessionInputValue<'_>)> = vec![
                    ("input_ids", input_ids_value.into()),
                    ("attention_mask", attention_value.into()),
                ];
                let outputs = session.run(inputs).map_err(EmbedError::Ort)?;
                let (out_shape, data) = outputs[0]
                    .try_extract_tensor::<f32>()
                    .map_err(EmbedError::Ort)?;
                let dims: Vec<usize> = out_shape.as_ref().iter().map(|&d| d as usize).collect();
                if dims.len() != 3 || dims[0] != batch_len || dims[1] != seq_len {
                    return Err(EmbedError::DimMismatch);
                }
                let last_hidden =
                    ndarray::Array3::from_shape_vec((dims[0], dims[1], dims[2]), data.to_vec())
                        .map_err(|_| EmbedError::DimMismatch)?;
                let pooled = mean_pool_3d_batch(&last_hidden, &attention_rows);
                for vec in pooled {
                    let mut v: Vec<f32> = vec.to_vec();
                    if v.len() != crate::rag::db::RAG_EMBED_DIM {
                        return Err(EmbedError::DimMismatch);
                    }
                    normalize(&mut v);
                    out.push(v);
                }
                start += batch_len;
            }
            Ok(out)
        }
        #[cfg(not(feature = "onnx"))]
        {
            let mut out = Vec::with_capacity(texts.len());
            for t in texts {
                out.push(self.embed(t)?);
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// stub_is_not_available.
    fn stub_is_not_available() {
        let e = RagEmbedder::stub();
        assert!(!e.is_available());
    }

    #[test]
    /// stub_embed_returns_not_loaded.
    fn stub_embed_returns_not_loaded() {
        let e = RagEmbedder::stub();
        let res = e.embed("hello");
        assert!(res.is_err());
    }

    #[test]
    /// stub_embed_batch_returns_not_loaded.
    fn stub_embed_batch_returns_not_loaded() {
        let e = RagEmbedder::stub();
        let texts = vec!["a".to_string(), "b".to_string()];
        let res = e.embed_batch(&texts);
        assert!(res.is_err());
    }
}
