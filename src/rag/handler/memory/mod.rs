//! Memory, training, and rule persistence tools: commit_to_memory, log_training_row,
//! approve_pattern, auto_approve_pattern, refresh_file_index, save_rule_to_memory, propose_vault_rule.

mod rules;
mod tools;

pub use rules::{propose_vault_rule_impl, save_rule_to_memory_impl};
pub use tools::{
    approve_pattern_impl, auto_approve_pattern_impl, commit_to_memory_impl, log_training_row_impl,
    refresh_file_index_impl,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Max retries for commit_to_memory file I/O (transient errors).
const COMMIT_TO_MEMORY_RETRY_ATTEMPTS: u32 = 3;
/// Base backoff (ms) for commit_to_memory retry; attempt N uses base * 2^N.
const COMMIT_TO_MEMORY_RETRY_BASE_MS: u64 = 50;

pub(crate) fn is_retryable_io_error(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    matches!(
        e.kind(),
        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
    )
}

pub(crate) const fn commit_to_memory_retry_attempts() -> u32 {
    COMMIT_TO_MEMORY_RETRY_ATTEMPTS
}
pub(crate) const fn commit_to_memory_retry_base_ms() -> u64 {
    COMMIT_TO_MEMORY_RETRY_BASE_MS
}

static HUMANIZER_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

pub(crate) fn humanize_text(text: &str, url: &str) -> Result<String, ()> {
    let client = HUMANIZER_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("humanizer blocking client")
    });
    let body = serde_json::json!({ "text": text }).to_string();
    let res = client
        .post(url)
        .body(body)
        .header("Content-Type", "application/json")
        .send()
        .map_err(|_| ())?;
    if !res.status().is_success() {
        return Err(());
    }
    res.text().map_err(|_| ())
}

// ---------- Params ----------

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// CommitToMemoryParams.
pub struct CommitToMemoryParams {
    /// Short title for the lesson (e.g. "RAG symbol extraction").
    pub title: String,
    /// Lesson or decision text (markdown-friendly).
    pub lesson: String,
    /// Category: e.g. architecture, debugging, security, documentation, testing.
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// LogTrainingRowParams.
pub struct LogTrainingRowParams {
    /// Exact task line from queue (e.g. "[DOCS] Document X in Y"). Do not use placeholder like "run N doc check".
    pub query: String,
    /// Retrieved or edit context.
    pub context: String,
    /// Solution summary or code.
    pub response: String,
    /// Domain tag (default "ouroboros" when omitted).
    #[serde(default)]
    pub domain: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ApprovePatternParams.
pub struct ApprovePatternParams {
    /// The name of the pattern to approve (e.g. 'Golden Vibe React Component').
    pub name: String,
    /// The code or pattern implementing the solution.
    pub code: String,
    /// The language of the code block.
    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// RefreshFileIndexParams.
pub struct RefreshFileIndexParams {
    /// File paths to re-ingest into RAG (parse, chunk, embed/index). Must be under ALLOWED_ROOTS.
    pub paths: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// SaveRuleToMemoryParams.
pub struct SaveRuleToMemoryParams {
    /// Rule text (markdown-friendly). Appended with a timestamp so RECALL can surface it.
    pub rule: String,
}

#[cfg(test)]
mod tests {
    use super::humanize_text;

    #[test]
    fn humanize_text_fails_on_unreachable_url() {
        let res = humanize_text("hello", "http://127.0.0.1:45999");
        assert!(
            res.is_err(),
            "humanize_text should fail on connection refused"
        );
    }
}
