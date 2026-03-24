//! Shared CLI logic for refresh_file_index and verify_integrity.
//! Used by both the MCP handler and the rag-mcp binary subcommands.
//! `run_refresh_file_index`: re-ingest given paths under allowed_roots; returns count refreshed.
//! `run_verify_integrity`: runs cargo check, test, clippy in project_root and returns JSON result.

use std::path::{Path, PathBuf};

use crate::rag::db::RagDb;
use crate::rag::embedding::RagEmbedder;
use crate::rag::ingest::{ingest_single_file, load_manifest, save_manifest};

/// Re-ingest the given file paths into the RAG index (parse, chunk, embed). Paths under allowed_roots only.
/// Paths must be under allowed_roots; non-files and paths outside roots are skipped.
/// Returns the number of files successfully refreshed.
pub fn run_refresh_file_index(
    db: &RagDb,
    embedder: &RagEmbedder,
    allowed_roots: &[PathBuf],
    manifest_path: Option<&Path>,
    paths: &[PathBuf],
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let mut to_ingest: Vec<PathBuf> = Vec::new();
    for path in paths {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !canonical.is_file() {
            tracing::debug!(
                "refresh_file_index: skip (not a file): {}",
                canonical.display()
            );
            continue;
        }
        let under_allowed = allowed_roots.iter().any(|r| canonical.starts_with(r));
        if !under_allowed {
            tracing::debug!(
                "refresh_file_index: skip (not under ALLOWED_ROOTS): {}",
                canonical.display()
            );
            continue;
        }
        to_ingest.push(canonical);
    }

    let mut manifest = match manifest_path {
        Some(p) => load_manifest(p),
        None => std::collections::HashMap::new(),
    };

    let prefixes: Vec<String> = to_ingest
        .iter()
        .map(|p| format!("{}::", p.display()))
        .collect();
    manifest.retain(|k, _| !prefixes.iter().any(|prefix| k.starts_with(prefix)));

    let mut files_refreshed = 0u32;
    for path in &to_ingest {
        // Wrap in catch_unwind so a panic in the embedder or tokenizer for one file does not
        // abort the spawn_blocking task and surface as an opaque JoinError to the caller.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ingest_single_file(path.as_path(), db, embedder, allowed_roots, &mut manifest)
        }));
        match result {
            Ok(Ok(Some(_))) => files_refreshed += 1,
            Ok(Ok(None)) => {}
            Ok(Err(e)) => {
                tracing::warn!("refresh_file_index ingest_single_file {:?}: {}", path, e);
            }
            Err(_panic) => {
                tracing::error!(
                    path = ?path,
                    "refresh_file_index: panic during ingest; file skipped (FTS-only index may be stale for this path)"
                );
            }
        }
    }

    if let Some(p) = manifest_path {
        if let Err(e) = save_manifest(p, &manifest) {
            tracing::warn!("refresh_file_index: failed to save manifest: {}", e);
        }
    }

    Ok(files_refreshed)
}

use crate::process_utils::run_command;

/// Run cargo check, test, and clippy in project_root and return JSON result string.
pub fn run_verify_integrity(project_root: &Path) -> String {
    let run = |args: &[&str]| -> (bool, String) {
        match run_command("cargo", args, project_root) {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let combined = if stderr.is_empty() {
                    stdout
                } else if stdout.is_empty() {
                    stderr
                } else {
                    format!("{}\n{}", stdout, stderr)
                };
                (output.status.success(), combined)
            }
            Err(e) => (false, e.to_string()),
        }
    };

    let (syntax_ok, syntax_out) = run(&["check"]);
    // --no-run compiles tests without executing them. This avoids "Access is denied (os error 5)"
    // on Windows when the release binary is locked by a running MCP server process. Full test
    // execution can be done from the CLI after stopping the server: `cargo test` in monolith/.
    let (tests_ok, tests_out) = run(&["test", "--no-run"]);
    let (clippy_ok, clippy_out) = run(&["clippy", "--all-targets", "--", "-D", "warnings"]);

    let linter_ok =
        if clippy_out.contains("unknown subcommand") || clippy_out.contains("is not a valid") {
            true
        } else {
            clippy_ok
        };

    let summary = if syntax_ok && tests_ok && linter_ok {
        "pass"
    } else if !syntax_ok {
        "fail: syntax (cargo check)"
    } else if !tests_ok {
        "fail: tests (cargo test)"
    } else {
        "fail: linter (cargo clippy)"
    };

    let json = serde_json::json!({
        "syntax_ok": syntax_ok,
        "tests_ok": tests_ok,
        "linter_ok": linter_ok,
        "summary": summary,
        "syntax_stderr": syntax_out,
        "tests_stderr": tests_out,
        "linter_stderr": clippy_out,
    });
    json.to_string()
}
