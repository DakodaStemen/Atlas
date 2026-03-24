//! LoRA flywheel: capture successful local model interactions (context + output) to training.jsonl
//! for future background QLoRA training. Keyword extraction: lowercase, drop stopwords and terms of length ≤ 2 (see extract_keywords).

use crate::rag::store::EMPTY_RAG_CONTEXT;
use regex::Regex;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Max retries for transient file I/O in record_interaction.
const FILE_WRITE_RETRY_ATTEMPTS: u32 = 3;
/// Base backoff (ms) for retry; attempt N uses base * 2^N (50, 100, 200).
const FILE_WRITE_RETRY_BASE_MS: u64 = 50;

/// True if the I/O error is likely transient and retrying may help.
/// ErrorKind::Other is intentionally excluded: it is a catch-all that includes
/// non-transient conditions and retrying it would mask the real failure.
fn is_retryable_io_error(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    matches!(
        e.kind(),
        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
    )
}

/// Lock guard for serializing appends to the training file. No data stored; only mutex ownership.
type AppendGuard = Mutex<()>;

/// Minimum response length to log; shorter responses are treated as low-value.
const MIN_RESPONSE_LEN: usize = 20;
/// Max characters for the context field in a training row; avoids unbounded token-heavy rows (aligns with RAG caps).
const MAX_TRAINING_CONTEXT_CHARS: usize = 32_000;
/// Maximum training file size before rotation (100 MiB). When exceeded, the current file is
/// renamed to `training.jsonl.bak` (overwriting any previous backup) and a fresh file is started.
const MAX_TRAINING_FILE_BYTES: u64 = 100 * 1024 * 1024;

/// Response strings (trimmed, case-insensitive) that are considered low-value and not logged.
const LOW_VALUE_RESPONSES: &[&str] = &[
    "ok",
    "existing or verified doc.",
    "doc/test/refactor",
    "no code change",
    "no code changes",
    "no changes",
    "no fixes needed",
    "all passed",
    "no warnings",
    "no warnings.",
];

/// Regex for low-value query: "run N" or "run N doc check" (case-insensitive). Same as prep_training_for_unsloth.py.
fn low_value_query_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)^run \d+( doc check)?\s*$").expect("static regex"))
}

/// True if query is a generic placeholder (e.g. "run 42", "run 61 doc check").
fn is_low_value_query(query: &str) -> bool {
    low_value_query_regex().is_match(query.trim())
}

/// True if response is too short or is a known generic string.
fn is_low_value_response(response: &str) -> bool {
    let r = response.trim();
    if r.len() < MIN_RESPONSE_LEN {
        return true;
    }
    let r_lower = r.to_lowercase();
    LOW_VALUE_RESPONSES.iter().any(|&s| r_lower == s)
}

/// Returns true if this (query, response) pair would be skipped by record_interaction (low-value).
/// Use when cleaning existing training.jsonl (e.g. bin-to-jsonl --strip-low-value).
pub fn is_low_value_training_row(query: &str, response: &str) -> bool {
    is_low_value_query(query) || is_low_value_response(response)
}

/// Rotate `path` if it exceeds `MAX_TRAINING_FILE_BYTES`: rename to `<path>.bak` and let
/// the next append create a fresh file. Silently ignores errors (best-effort rotation).
fn rotate_training_file_if_needed(path: &std::path::Path) {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size < MAX_TRAINING_FILE_BYTES {
        return;
    }
    let backup = path.with_extension("jsonl.bak");
    let _ = std::fs::rename(path, &backup);
}

/// Common English stopwords for keyword extraction; excludes them from extract_keywords so relevance checks avoid noise.
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
    "from", "as", "is", "was", "are", "were", "be", "been", "being", "have", "has", "had", "do",
    "does", "did", "will", "would", "should", "could", "may", "might", "can", "what", "how",
    "where", "when", "why", "which", "who", "this", "that", "these",
];

/// Extract keywords from query: lowercase, split on whitespace, drop stopwords and terms of length ≤ 2. Used by is_context_relevant. Allocates one Vec; suitable for per-query use.
fn extract_keywords(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split_whitespace()
        .filter(|word| {
            word.len() > 2
                && !STOPWORDS.contains(&word.trim_matches(|c: char| !c.is_alphanumeric()))
        })
        .map(|word| {
            word.trim_matches(|c: char| !c.is_alphanumeric())
                .to_string()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// Check if context is relevant to the query by looking for keyword overlap (case-insensitive).
/// Returns true if: (1) at least 2 query keywords appear in context, or
/// (2) for short queries (≤4 keywords), more than 30% of keywords appear in context.
fn is_context_relevant(query: &str, context: &str) -> bool {
    let keywords = extract_keywords(query);
    if keywords.is_empty() {
        return true; // Can't judge, allow it
    }

    let context_lower = context.to_lowercase();
    let matches = keywords
        .iter()
        .filter(|kw| context_lower.contains(kw.as_str()))
        .count();

    // Require at least 2 keywords or >30% match for short queries
    matches >= 2 || (keywords.len() <= 4 && matches as f64 / keywords.len() as f64 > 0.3)
}

/// Thread-safe collector: appends JSONL lines to a training file. Safe to share via `Arc<DatasetCollector>`;
/// internal `Mutex` serializes file open + write so concurrent calls never interleave.
#[derive(Debug)]
/// Writes interaction records to training.jsonl; use [`default_path`] for path. Mutex serializes appends.
pub struct DatasetCollector {
    training_path: PathBuf,
    /// When set (e.g. GOLDEN_SET_DIR / .prism), approve_pattern writes golden_set.md here for Prism compatibility.
    golden_set_dir: Option<PathBuf>,
    /// Serializes append (create_dir_all + open + write_all) so shared use is 100% thread-safe.
    write_mutex: AppendGuard,
}

impl DatasetCollector {
    /// Create collector writing to `training_path`. Parent directory is created on first write.
    /// When `golden_set_dir` is Some (e.g. from GOLDEN_SET_DIR), approve_pattern writes to `golden_set_dir/golden_set.md` and training to `golden_set_dir/training.jsonl` (caller should set training_path accordingly).
    pub fn new(training_path: PathBuf, golden_set_dir: Option<PathBuf>) -> Self {
        Self {
            training_path,
            golden_set_dir,
            write_mutex: Mutex::new(()),
        }
    }

    /// Default path for training.jsonl: data_dir/training.jsonl.
    /// Path for training.jsonl: data_dir/training.jsonl (used by main and bin-to-jsonl).
    pub fn default_path(data_dir: &str) -> PathBuf {
        PathBuf::from(data_dir).join("training.jsonl")
    }

    /// Append one JSONL line: `{"query": "...", "context": "...", "domain": "...", "ts": <unix_epoch>}`.
    /// 100% thread-safe: internal Mutex serializes create_dir_all + open + write. Safe when shared via Arc.
    /// Skips when context is empty or EMPTY_RAG_CONTEXT; skips when context is not relevant to query;
    /// skips when query or response is low-value (e.g. "run N doc check", "ok").
    pub fn record_interaction(
        &self,
        query: &str,
        context: &str,
        response: &str,
        domain: &str,
    ) -> Result<(), std::io::Error> {
        if query.is_empty() && context.is_empty() {
            return Ok(());
        }
        if context.is_empty() || context == EMPTY_RAG_CONTEXT {
            return Ok(());
        }
        if !is_context_relevant(query, context) {
            return Ok(());
        }
        if is_low_value_query(query) || is_low_value_response(response) {
            return Ok(());
        }
        let context_trimmed = if context.chars().count() > MAX_TRAINING_CONTEXT_CHARS {
            context
                .chars()
                .take(MAX_TRAINING_CONTEXT_CHARS)
                .collect::<String>()
        } else {
            context.to_string()
        };
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let line = serde_json::json!({
            "query": query,
            "context": context_trimmed,
            "response": response,
            "domain": domain,
            "ts": ts
        });
        let mut line_str = serde_json::to_string(&line).unwrap_or_default();
        line_str.push('\n');
        let path = self.training_path.clone();
        let mut last_err = None::<std::io::Error>;
        for attempt in 0..FILE_WRITE_RETRY_ATTEMPTS {
            if attempt > 0 {
                let ms = FILE_WRITE_RETRY_BASE_MS * (1 << attempt);
                std::thread::sleep(Duration::from_millis(ms));
            }
            if let Some(parent) = path.parent() {
                match std::fs::create_dir_all(parent) {
                    Ok(()) => {}
                    Err(e) => {
                        if is_retryable_io_error(&e) {
                            last_err = Some(e);
                            continue;
                        }
                        return Err(e);
                    }
                }
            }
            let _guard = self.write_mutex.lock().map_err(|e| {
                std::io::Error::other(format!("DatasetCollector mutex poisoned: {}", e))
            })?;
            rotate_training_file_if_needed(&path);
            match OpenOptions::new().create(true).append(true).open(&path) {
                Ok(mut f) => match f.write_all(line_str.as_bytes()) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        if is_retryable_io_error(&e) {
                            last_err = Some(e);
                            continue;
                        }
                        return Err(e);
                    }
                },
                Err(e) => {
                    if is_retryable_io_error(&e) {
                        last_err = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(last_err
            .unwrap_or_else(|| std::io::Error::other("record_interaction failed after 3 retries")))
    }

    /// Golden Vibe Flywheel: Appends an approved pattern to golden_set.md and logs it as a training example.
    /// When golden_set_dir is set (GOLDEN_SET_DIR / .prism), writes to golden_set_dir/golden_set.md; else docs/golden_set.md.
    pub fn approve_pattern(
        &self,
        name: &str,
        code: &str,
        language: Option<&str>,
    ) -> Result<(), std::io::Error> {
        let lang = language.unwrap_or("text");
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!(
            "\n\n## PATTERN: {} ({})\n```{}\n{}\n```\n",
            name, timestamp, lang, code
        );

        let docs_dir = if let Some(ref d) = self.golden_set_dir {
            d.clone()
        } else {
            self.training_path
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("docs"))
                .unwrap_or_else(|| PathBuf::from("docs"))
        };
        std::fs::create_dir_all(&docs_dir)?;
        let project_path = docs_dir.join("golden_set.md");

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&project_path)?;
        file.write_all(entry.as_bytes())?;

        // Also log it to the training.jsonl dataset.
        // Pass code as context so the keyword relevance check can find the pattern name.
        self.record_interaction(
            &format!("How do I implement {}?", name),
            code,
            code,
            "golden_vibe",
        )?;

        Ok(())
    }
}

/// Optional wrapper: Mutex<DatasetCollector> for use from async handler. DatasetCollector is already
/// thread-safe internally (Arc<DatasetCollector> is safe); this type preserves existing API.
pub type DatasetCollectorGuard = Mutex<DatasetCollector>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// test_extract_keywords.
    fn test_extract_keywords() {
        let kw = extract_keywords("What is the Python GIL?");
        assert!(kw.contains(&"python".to_string()));
        assert!(kw.contains(&"gil".to_string()));
        assert!(!kw.contains(&"is".to_string())); // stopword
        assert!(!kw.contains(&"the".to_string())); // stopword
    }

    #[test]
    /// test_extract_keywords_filters_short_words.
    fn test_extract_keywords_filters_short_words() {
        let kw = extract_keywords("ab cd abc def");
        assert!(!kw.iter().any(|w| w.len() <= 2));
        assert!(kw.contains(&"abc".to_string()));
        assert!(kw.contains(&"def".to_string()));
    }

    #[test]
    /// test_extract_keywords_single_word_returns_one.
    fn test_extract_keywords_single_word_returns_one() {
        let kw = extract_keywords("hello");
        assert_eq!(kw, vec!["hello".to_string()]);
    }

    #[test]
    /// test_extract_keywords_empty_string_returns_empty.
    fn test_extract_keywords_empty_string_returns_empty() {
        assert!(extract_keywords("").is_empty());
        assert!(extract_keywords("   ").is_empty());
    }

    #[test]
    /// test_is_context_relevant.
    fn test_is_context_relevant() {
        // Relevant: context contains query keywords
        let query = "Python GIL threading";
        let context = "<web_answer>The GIL (Global Interpreter Lock) in Python prevents multiple threads from executing Python bytecode simultaneously.</web_answer>";
        assert!(is_context_relevant(query, context));

        // Irrelevant: generic JSONL answer for specific code query
        let query = "DatasetCollector record_interaction training.jsonl";
        let context =
            "JSONL is a file format where each line is a valid JSON object. Common uses include...";
        assert!(!is_context_relevant(query, context));

        // Relevant: at least 2 keywords match
        let query = "RagStore hybrid search algorithm";
        let context = "The RagStore implements a hybrid search combining vector and FTS...";
        assert!(is_context_relevant(query, context));
    }

    #[test]
    /// record_interaction_writes_valid_jsonl.
    fn record_interaction_writes_valid_jsonl() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let collector = DatasetCollector::new(path.clone(), None);
        let response = "RagStore implements hybrid search combining vector and FTS.";
        collector
            .record_interaction(
                "RagStore hybrid search",
                "<ctx>RagStore hybrid search algorithm</ctx>",
                response,
                "ouroboros",
            )
            .expect("ok");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(content.contains("\"query\""));
        assert!(content.contains("\"context\""));
        let _: serde_json::Value =
            serde_json::from_str(content.lines().next().unwrap()).expect("valid JSON");
    }

    #[test]
    /// record_interaction_does_not_write_when_context_is_empty_rag_context.
    fn record_interaction_does_not_write_when_context_is_empty_rag_context() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let collector = DatasetCollector::new(path.clone(), None);
        collector
            .record_interaction("some query", EMPTY_RAG_CONTEXT, "response", "ouroboros")
            .expect("ok");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !content.contains("some query"),
            "should not write when context is EMPTY_RAG_CONTEXT"
        );
    }

    #[test]
    /// record_interaction_does_not_write_when_query_is_low_value.
    fn record_interaction_does_not_write_when_query_is_low_value() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let collector = DatasetCollector::new(path.clone(), None);
        let ctx = "ouroboros batch run doc check 61-100";
        collector
            .record_interaction(
                "run 42 doc check",
                ctx,
                "Existing or verified doc.",
                "ouroboros",
            )
            .expect("ok");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !content.contains("run 42"),
            "should not write when query is low-value (run N doc check)"
        );
    }

    /// record_interaction caps context at MAX_TRAINING_CONTEXT_CHARS in the written JSONL.
    #[test]
    fn record_interaction_caps_context_at_max_chars() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let collector = DatasetCollector::new(path.clone(), None);
        let over_cap = MAX_TRAINING_CONTEXT_CHARS + 1000;
        let mut context = "a".repeat(over_cap);
        context.push_str(" keyword_for_relevance");
        collector
            .record_interaction(
                "keyword_for_relevance",
                &context,
                "A meaningful response that is long enough to pass low-value check.",
                "ouroboros",
            )
            .expect("ok");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let line = content.lines().next().expect("one line");
        let row: serde_json::Value = serde_json::from_str(line).expect("valid JSON");
        let ctx = row.get("context").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            ctx.len() <= MAX_TRAINING_CONTEXT_CHARS,
            "context field must be capped at {} chars, got {}",
            MAX_TRAINING_CONTEXT_CHARS,
            ctx.len()
        );
    }

    #[test]
    /// record_interaction_does_not_write_when_response_is_short_or_blocklisted.
    fn record_interaction_does_not_write_when_response_is_short_or_blocklisted() {
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();
        let collector = DatasetCollector::new(path.clone(), None);
        let query = "[DOCS] Document X in Y";
        let ctx = "Document X and Y are documented in the module.";
        collector
            .record_interaction(query, ctx, "ok", "ouroboros")
            .expect("ok");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !content.contains(query),
            "should not write when response is blocklisted (ok)"
        );
    }

    /// A1 Golden Vibe: approve_pattern with golden_set_dir writes to that dir (Prism-compatible path).
    #[test]
    /// approve_pattern_with_golden_set_dir_writes_to_dir.
    fn approve_pattern_with_golden_set_dir_writes_to_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let training_path = tmp.path().join("training.jsonl");
        let golden_dir = tmp.path().join(".prism");
        let collector = DatasetCollector::new(training_path, Some(golden_dir.clone()));
        collector
            .approve_pattern("TestPattern", "fn foo() {}", Some("rust"))
            .expect("approve_pattern");
        let golden_md = golden_dir.join("golden_set.md");
        assert!(
            golden_md.exists(),
            "golden_set.md must be created under golden_set_dir"
        );
        let content = std::fs::read_to_string(&golden_md).unwrap_or_default();
        assert!(content.contains("PATTERN: TestPattern"));
        assert!(content.contains("fn foo() {}"));
    }
}
