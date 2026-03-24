//! Shell and system tools: get_system_status, execute_shell_command implementations and helpers.

use super::{
    sanitize_shell_output, truncate_for_budget, AgenticHandler, IngestionProvider,
    VectorStoreProvider,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
use tokio::sync::TryAcquireError;

// ---------- Params (re-exported by handler mod) ----------

fn default_true() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GetSystemStatusParams.
pub struct GetSystemStatusParams {
    /// If true (default), query GPU via nvidia-smi for VRAM and utilization.
    #[serde(default = "default_true")]
    pub gpu: bool,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ExecuteShellCommandParams.
pub struct ExecuteShellCommandParams {
    /// Shell command to run (e.g. "cargo test --lib symbols"). Executed in the first allowed project root. Destructive commands are blocked.
    pub command: String,
    /// Optional timeout in seconds. When set, overrides EXECUTE_SHELL_TIMEOUT_SECS for this call (e.g. for stress tests).
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// When true, skip the git destructive-args check only if the server process has `MCP_SHELL_BYPASS_HITL=1` set (defense in depth). Otherwise ignored.
    #[serde(default)]
    pub bypass_hitl: Option<bool>,
}

// ---------- Helpers ----------

/// Default allowed program names when EXECUTE_SHELL_ALLOWLIST is unset.
const ALLOWED_PROGRAMS: &[&str] = &["cargo", "git", "grep", "ls", "npm"];

pub(crate) fn parse_shell_allowlist(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_lowercase())
        .filter(|x| !x.is_empty() && !x.contains('/') && !x.contains('\\') && !x.contains(".."))
        .collect()
}

fn get_shell_allowlist() -> Vec<String> {
    if let Ok(v) = std::env::var("EXECUTE_SHELL_ALLOWLIST") {
        let list = parse_shell_allowlist(v.trim());
        if !list.is_empty() {
            return list;
        }
    }
    ALLOWED_PROGRAMS.iter().map(|s| (*s).to_string()).collect()
}

/// Message returned when execute_shell_command rejects a command not on the allowlist.
pub const COMMAND_REJECTION_MESSAGE: &str =
    "Command rejected: program not on allowlist (EXECUTE_SHELL_ALLOWLIST or built-in: cargo, git, grep, ls, npm).";
/// Message when a redirect target is under the hub (allowed roots).
pub const VAULT_BOUNDARY_MESSAGE: &str =
    "Command rejected: redirect target is inside the workspace (hub boundary). Shell commands must not write into the hub.";
/// Message when a destructive git command is rejected (no bypass).
pub const GIT_DESTRUCTIVE_REJECT_MESSAGE: &str =
    "Command rejected: destructive git command not allowed (push, clean, rebase, or flags --hard/--force/--amend).";

const DEFAULT_SHELL_MAX_OUTPUT_CHARS: usize = 16_000;
const DEFAULT_EXECUTE_SHELL_TIMEOUT_SECS: u64 = 120;
/// Default max concurrent execute_shell_command calls. Override at runtime with MAX_CONCURRENT_SHELL env var.
const MAX_CONCURRENT_SHELL: usize = 3;
/// Safety cap: env overrides above this value are clamped and a warning is emitted.
const MAX_CONCURRENT_SHELL_HARD_CAP: usize = 10;
/// Process-global semaphore bounding concurrent shell executions. Lazily initialized from env at first use.
static SHELL_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();

fn shell_semaphore() -> &'static tokio::sync::Semaphore {
    SHELL_SEMAPHORE.get_or_init(|| {
        let limit = std::env::var("MAX_CONCURRENT_SHELL")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(MAX_CONCURRENT_SHELL);
        if limit > MAX_CONCURRENT_SHELL_HARD_CAP {
            tracing::warn!(
                limit,
                cap = MAX_CONCURRENT_SHELL_HARD_CAP,
                "MAX_CONCURRENT_SHELL exceeds safety cap; clamping"
            );
        }
        tokio::sync::Semaphore::new(limit.min(MAX_CONCURRENT_SHELL_HARD_CAP))
    })
}

fn read_shell_max_output_chars() -> usize {
    std::env::var("EXECUTE_SHELL_MAX_OUTPUT_CHARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_SHELL_MAX_OUTPUT_CHARS)
}

fn read_execute_shell_timeout_secs() -> u64 {
    std::env::var("EXECUTE_SHELL_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_EXECUTE_SHELL_TIMEOUT_SECS)
}

struct KillOnDropChild(Option<tokio::process::Child>);

impl Drop for KillOnDropChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.start_kill();
            tokio::spawn(async move {
                let _ = child.wait().await;
            });
        }
    }
}

/// Returns true if the command's program (first token) is on the allowlist.
pub fn is_command_allowed(command: &str) -> bool {
    let argv = match shlex::split(command) {
        Some(a) if !a.is_empty() => a,
        _ => return false,
    };
    let program = argv[0].trim();
    let name = program
        .strip_suffix(".exe")
        .unwrap_or(program)
        .to_lowercase();
    let allowlist = get_shell_allowlist();
    allowlist.iter().any(|p| p.as_str() == name)
}

const BLOCKED_GIT_SUBCOMMANDS: &[&str] = &["push", "clean", "rebase"];
const BLOCKED_GIT_FLAGS: &[&str] = &["--hard", "--force", "--force-with-lease", "--amend", "-f"];

/// Blocked cargo subcommands (e.g. uninstall can remove crates).
const BLOCKED_CARGO_SUBCOMMANDS: &[&str] = &["uninstall"];

/// Blocked npm subcommands (e.g. uninstall can remove packages).
const BLOCKED_NPM_SUBCOMMANDS: &[&str] = &["uninstall"];

/// Returns false when argv[0] is "git" and argv contains a blocked subcommand or flag.
pub fn is_git_args_safe(argv: &[String]) -> bool {
    if argv.len() < 2 {
        return true;
    }
    let sub = argv[1].to_lowercase();
    if BLOCKED_GIT_SUBCOMMANDS.iter().any(|&b| b == sub) {
        return false;
    }
    argv[1..]
        .iter()
        .all(|a| !BLOCKED_GIT_FLAGS.iter().any(|&f| a.eq_ignore_ascii_case(f)))
}

/// Returns false when argv[0] is "cargo" and argv contains a blocked subcommand.
pub fn is_cargo_args_safe(argv: &[String]) -> bool {
    if argv.len() < 2 {
        return true;
    }
    let sub = argv[1].to_lowercase();
    !BLOCKED_CARGO_SUBCOMMANDS.iter().any(|&b| b == sub)
}

/// Returns false when argv[0] is "npm" and argv contains a blocked subcommand.
pub fn is_npm_args_safe(argv: &[String]) -> bool {
    if argv.len() < 2 {
        return true;
    }
    let sub = argv[1].to_lowercase();
    !BLOCKED_NPM_SUBCOMMANDS.iter().any(|&b| b == sub)
}

fn are_path_args_safe(
    program: &str,
    argv: &[String],
    allowed: &[PathBuf],
    project_root: &Path,
) -> bool {
    let prog = program.to_lowercase();
    let prog = prog.strip_suffix(".exe").unwrap_or(&prog);
    if prog != "grep" && prog != "ls" && prog != "npm" && prog != "cargo" {
        return true;
    }
    for arg in argv.iter().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        if arg.starts_with("http://") || arg.starts_with("https://") {
            continue;
        }
        let p = Path::new(arg);
        let resolved = if p.is_absolute() {
            p.to_path_buf()
        } else {
            project_root.join(p)
        };
        if !crate::rag::path_filter::path_under_allowed(&resolved, allowed, false) {
            return false;
        }
    }
    true
}

pub(crate) fn redirect_targets_from_argv(argv: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let a = argv[i].trim();
        if a == ">" || a == ">>" {
            i += 1;
            if i < argv.len() {
                out.push(PathBuf::from(&argv[i]));
            }
        }
        i += 1;
    }
    out
}

pub async fn get_system_status_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: Parameters<GetSystemStatusParams>,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let (cpu_usage, total_mem, used_mem) = tokio::task::spawn_blocking(|| {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();
        std::thread::sleep(std::time::Duration::from_millis(100));
        sys.refresh_cpu_all();
        (
            sys.global_cpu_usage(),
            sys.total_memory(),
            sys.used_memory(),
        )
    })
    .await
    .map_err(|e| McpError::internal_error(format!("sysinfo spawn: {}", e), None))?;
    let mem_pct = if total_mem > 0 {
        (used_mem as f64 / total_mem as f64) * 100.0
    } else {
        0.0
    };

    let mut lines = vec![
        format!("CPU: {:.1}%", cpu_usage),
        format!(
            "RAM: {} MB / {} MB ({:.1}%)",
            used_mem / 1024 / 1024,
            total_mem / 1024 / 1024,
            mem_pct
        ),
    ];

    if params.0.gpu {
        let vram_out = tokio::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=memory.used,memory.total,utilization.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .await;

        match vram_out {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let first_line = stdout.lines().next().unwrap_or("").trim();
                let parts: Vec<&str> = first_line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 2 {
                    let used_mb: f64 = parts[0].parse().unwrap_or(0.0);
                    let total_mb: f64 = parts[1].parse().unwrap_or(1.0);
                    let gpu_util = if parts.len() >= 3 {
                        parts[2].replace('%', "").trim().parse().unwrap_or(0)
                    } else {
                        0
                    };
                    let vram_pct = if total_mb > 0.0 {
                        (used_mb / total_mb) * 100.0
                    } else {
                        0.0
                    };
                    lines.push(format!(
                        "VRAM: {:.0} MB / {:.0} MB ({:.1}%), GPU util: {}%",
                        used_mb, total_mb, vram_pct, gpu_util
                    ));
                    if vram_pct > 90.0 {
                        lines.push(
                            "VRAM CRITICALLY HIGH: PROCEED WITH CAUTION FOR LOCAL LLM EXECUTION"
                                .to_string(),
                        );
                    }
                } else {
                    lines.push("VRAM: nvidia-smi output could not be parsed".to_string());
                }
            }
            Ok(_) => {
                lines.push("VRAM: nvidia-smi returned non-zero or no output".to_string());
            }
            Err(e) => {
                lines.push(format!("VRAM: nvidia-smi failed ({})", e));
            }
        }
    }

    let db = std::sync::Arc::clone(&handler.store.db);
    let db_status = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::task::spawn_blocking(move || db.count_chunks()),
    )
    .await;
    match db_status {
        Ok(Ok(Ok(_))) => lines.push("DB: available".to_string()),
        Ok(Ok(Err(e))) => lines.push(format!("DB: error: {}", e)),
        Ok(Err(e)) => lines.push(format!("DB: spawn error: {}", e)),
        Err(_) => lines.push("DB: busy or timeout".to_string()),
    }

    let db_path = handler.store.db.db_path().to_path_buf();
    let disk_line = tokio::task::spawn_blocking(move || {
        use sysinfo::Disks;
        let path_buf = db_path.canonicalize().unwrap_or_else(|_| {
            db_path
                .parent()
                .map(std::path::PathBuf::from)
                .unwrap_or(db_path)
        });
        let disks = Disks::new_with_refreshed_list();
        let matching = disks
            .list()
            .iter()
            .filter(|d| path_buf.starts_with(d.mount_point()))
            .max_by_key(|d| d.mount_point().as_os_str().len());
        match matching {
            Some(disk) => format!(
                "Disk ({:?}): {} MB free / {} MB total",
                disk.mount_point(),
                disk.available_space() / (1024 * 1024),
                disk.total_space() / (1024 * 1024)
            ),
            None => "Disk: path not on a known volume".to_string(),
        }
    })
    .await
    .unwrap_or_else(|e| format!("Disk: spawn error: {}", e));
    lines.push(disk_line);

    let (rerank_hits, rerank_misses) = crate::rerank::rerank_stats();
    let reranker_available = handler
        .store
        .reranker
        .as_ref()
        .is_some_and(|r| r.is_available());
    lines.push(format!(
        "Reranker: available={}, hits={}, misses={}",
        reranker_available, rerank_hits, rerank_misses
    ));

    lines.push(crate::metrics::log_metrics_summary());

    let text = lines.join("\n");
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn execute_shell_command_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: Parameters<ExecuteShellCommandParams>,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let command = params.0.command.trim();
    if command.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "execute_shell_command requires a non-empty command.",
        )]));
    }
    if !is_command_allowed(command) {
        return Ok(CallToolResult::success(vec![Content::text(
            COMMAND_REJECTION_MESSAGE,
        )]));
    }
    if command.contains('\n') || command.contains('\r') {
        return Ok(CallToolResult::success(vec![Content::text(
            "Command rejected: newlines not allowed (single command only; possible injection).",
        )]));
    }
    let argv = match shlex::split(command) {
        Some(a) if !a.is_empty() => a,
        _ => {
            return Ok(CallToolResult::success(vec![Content::text(
                "Command could not be parsed (empty or invalid shlex).",
            )]));
        }
    };
    let bypass_hitl = params.0.bypass_hitl == Some(true)
        && std::env::var("MCP_SHELL_BYPASS_HITL")
            .ok()
            .as_deref()
            == Some("1");
    if argv[0].eq_ignore_ascii_case("git") && !is_git_args_safe(&argv) && !bypass_hitl {
        return Ok(CallToolResult::success(vec![Content::text(
            GIT_DESTRUCTIVE_REJECT_MESSAGE.to_string(),
        )]));
    }
    if argv[0].eq_ignore_ascii_case("cargo") && !is_cargo_args_safe(&argv) {
        return Ok(CallToolResult::success(vec![Content::text(
            "Command rejected: cargo subcommand not allowed (e.g. uninstall).",
        )]));
    }
    if argv[0].eq_ignore_ascii_case("npm") && !is_npm_args_safe(&argv) {
        return Ok(CallToolResult::success(vec![Content::text(
            "Command rejected: npm subcommand not allowed (e.g. uninstall).",
        )]));
    }
    let project_root: PathBuf = handler
        .store
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !are_path_args_safe(&argv[0], &argv, &handler.store.allowed_roots, &project_root) {
        return Ok(CallToolResult::success(vec![Content::text(
            "Command rejected: path argument is outside the allowed workspace roots.",
        )]));
    }
    for target in redirect_targets_from_argv(&argv) {
        let resolved = if target.is_absolute() {
            target
        } else {
            project_root.join(&target)
        };
        if crate::rag::path_filter::path_under_allowed(
            &resolved,
            &handler.store.allowed_roots,
            false,
        ) {
            return Ok(CallToolResult::success(vec![Content::text(
                VAULT_BOUNDARY_MESSAGE,
            )]));
        }
    }

    // Rate limiting: cap concurrent shell executions to MAX_CONCURRENT_SHELL (env-configurable).
    let sem = shell_semaphore();
    let _shell_permit = match sem.try_acquire() {
        Ok(p) => p,
        Err(TryAcquireError::NoPermits) => {
            return Ok(CallToolResult::success(vec![Content::text(
                "Too many concurrent shell commands (concurrency limit reached). Retry after the current command completes.".to_string()
            )]));
        }
        Err(TryAcquireError::Closed) => {
            return Err(McpError::internal_error("shell_semaphore closed", None));
        }
    };

    let (program, args) = (argv[0].clone(), argv[1..].to_vec());
    let timeout_secs = params
        .0
        .timeout_secs
        .unwrap_or_else(read_execute_shell_timeout_secs);

    let (stdout, stderr, code) = if timeout_secs == 0 {
        use crate::process_utils::run_command_async;
        match run_command_async(&program, &args, &project_root).await {
            Ok(output) => {
                let code = output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "none".to_string());
                (
                    String::from_utf8_lossy(&output.stdout).to_string(),
                    String::from_utf8_lossy(&output.stderr).to_string(),
                    code,
                )
            }
            Err(e) => {
                return Err(McpError::internal_error(
                    format!("execute_shell_command: {}", e),
                    None,
                ));
            }
        }
    } else {
        let mut child = tokio::process::Command::new(&program)
            .args(&args)
            .current_dir(&project_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| McpError::internal_error(format!("execute_shell_command: {}", e), None))?;
        let mut stdout_handle = child.stdout.take().expect("stdout piped");
        let mut stderr_handle = child.stderr.take().expect("stderr piped");
        let mut guard = KillOnDropChild(Some(child));
        let run = async move {
            let wait_fut = guard.0.as_mut().unwrap().wait();
            let mut stdout_buf = String::new();
            let mut stderr_buf = String::new();
            let (stdout_res, stderr_res, status_res) = tokio::join!(
                stdout_handle.read_to_string(&mut stdout_buf),
                stderr_handle.read_to_string(&mut stderr_buf),
                wait_fut,
            );
            let _ = (stdout_res, stderr_res);
            let stdout = stdout_buf;
            let stderr = stderr_buf;
            let code = status_res
                .ok()
                .and_then(|s| s.code())
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".to_string());
            (stdout, stderr, code)
        };
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), run).await {
            Ok((stdout, stderr, code)) => (stdout, stderr, code),
            Err(_) => {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "Command timed out after {} seconds. Set EXECUTE_SHELL_TIMEOUT_SECS=0 to disable.",
                    timeout_secs
                ))]));
            }
        }
    };

    let stdout = sanitize_shell_output(&stdout);
    let stderr = sanitize_shell_output(&stderr);
    let text = truncate_for_budget(
        &format!(
            "exit_code: {}\n\nstdout:\n{}\n\nstderr:\n{}",
            code, stdout, stderr
        ),
        read_shell_max_output_chars(),
    );
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GitCheckpointParams.
pub struct GitCheckpointParams {
    /// Action: "save" (create checkpoint), "revert" (rollback to last checkpoint), "status" (list checkpoints).
    pub action: String,
    /// Optional label for the checkpoint (used in commit message). Default "auto".
    #[serde(default)]
    pub label: Option<String>,
}

/// Create a git checkpoint commit with the current REQUEST_ID.
pub async fn git_checkpoint_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: GitCheckpointParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    let project_root: PathBuf = handler
        .store
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let action = params.action.trim().to_lowercase();
    let label = params.label.as_deref().unwrap_or("auto").trim().to_string();

    match action.as_str() {
        "save" => {
            // Stage all changes and create a checkpoint commit
            let root = project_root.clone();
            let result = tokio::task::spawn_blocking(move || {
                // git add -A
                let add = std::process::Command::new("git")
                    .args(["add", "-A"])
                    .current_dir(&root)
                    .output();
                if let Err(e) = add {
                    return format!("git add failed: {}", e);
                }

                // Check if there's anything to commit
                let status = std::process::Command::new("git")
                    .args(["status", "--porcelain"])
                    .current_dir(&root)
                    .output();
                let has_changes = status
                    .map(|o| !o.stdout.is_empty())
                    .unwrap_or(false);

                if !has_changes {
                    return "No changes to checkpoint.".to_string();
                }

                // git commit
                let msg = format!("checkpoint: {} [managed-loop]", label);
                let commit = std::process::Command::new("git")
                    .args(["commit", "-m", &msg, "--allow-empty"])
                    .current_dir(&root)
                    .output();
                match commit {
                    Ok(o) if o.status.success() => {
                        let stdout = String::from_utf8_lossy(&o.stdout);
                        format!("Checkpoint saved: {}", stdout.lines().next().unwrap_or("ok"))
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        format!("Checkpoint commit failed: {}", stderr)
                    }
                    Err(e) => format!("git commit failed: {}", e),
                }
            })
            .await
            .map_err(|e| McpError::internal_error(format!("git_checkpoint spawn: {}", e), None))?;

            Ok(CallToolResult::success(vec![Content::text(result)]))
        }
        "revert" => {
            if std::env::var("GIT_CHECKPOINT_REVERT_ALLOW")
                .ok()
                .as_deref()
                != Some("1")
            {
                return Ok(CallToolResult::success(vec![Content::text(
                    "git_checkpoint revert is disabled. Set GIT_CHECKPOINT_REVERT_ALLOW=1 in the server environment to allow destructive rollback (git reset --hard HEAD~1).",
                )]));
            }
            let root = project_root.clone();
            let result = tokio::task::spawn_blocking(move || {
                let output = std::process::Command::new("git")
                    .args(["reset", "--hard", "HEAD~1"])
                    .current_dir(&root)
                    .output();
                match output {
                    Ok(o) if o.status.success() => {
                        let stdout = String::from_utf8_lossy(&o.stdout);
                        format!("Reverted to previous checkpoint: {}", stdout.trim())
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        format!("Revert failed: {}", stderr.trim())
                    }
                    Err(e) => format!("git reset failed: {}", e),
                }
            })
            .await
            .map_err(|e| McpError::internal_error(format!("git_checkpoint spawn: {}", e), None))?;

            Ok(CallToolResult::success(vec![Content::text(result)]))
        }
        "status" => {
            // Show recent checkpoint commits
            let root = project_root.clone();
            let result = tokio::task::spawn_blocking(move || {
                let output = std::process::Command::new("git")
                    .args(["log", "--oneline", "-10", "--grep=checkpoint:"])
                    .current_dir(&root)
                    .output();
                match output {
                    Ok(o) if o.status.success() => {
                        let stdout = String::from_utf8_lossy(&o.stdout);
                        if stdout.trim().is_empty() {
                            "No checkpoint commits found.".to_string()
                        } else {
                            format!("Recent checkpoints:\n{}", stdout.trim())
                        }
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        format!("git log failed: {}", stderr.trim())
                    }
                    Err(e) => format!("git log failed: {}", e),
                }
            })
            .await
            .map_err(|e| McpError::internal_error(format!("git_checkpoint spawn: {}", e), None))?;

            let text = sanitize_shell_output(&result);
            Ok(CallToolResult::success(vec![Content::text(text)]))
        }
        _ => Ok(CallToolResult::success(vec![Content::text(
            "git_checkpoint: action must be 'save', 'revert', or 'status'.",
        )])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_command_allowed() {
        assert!(is_command_allowed("cargo test"));
        assert!(is_command_allowed("git status"));
        assert!(is_command_allowed("ls -la"));
        assert!(is_command_allowed("CARGO build"));
        assert!(is_command_allowed("cargo.exe test"));
        assert!(!is_command_allowed("rm -rf /"));
        assert!(!is_command_allowed("python main.py"));
        assert!(!is_command_allowed(""));
        assert!(!is_command_allowed("   "));
    }

    #[test]
    fn git_reset_hard_rejected_by_is_git_args_safe() {
        let argv = vec!["git".into(), "reset".into(), "--hard".into(), "HEAD".into()];
        assert!(
            !is_git_args_safe(&argv),
            "git reset --hard must be rejected by is_git_args_safe"
        );
    }

    #[test]
    fn cargo_uninstall_rejected_by_is_cargo_args_safe() {
        let argv = vec!["cargo".into(), "uninstall".into(), "some-crate".into()];
        assert!(
            !is_cargo_args_safe(&argv),
            "cargo uninstall must be rejected by is_cargo_args_safe"
        );
    }

    #[test]
    fn npm_uninstall_rejected_by_is_npm_args_safe() {
        let argv = vec!["npm".into(), "uninstall".into(), "lodash".into()];
        assert!(
            !is_npm_args_safe(&argv),
            "npm uninstall must be rejected by is_npm_args_safe"
        );
    }

    #[test]
    fn path_arg_dotdot_resolves_outside_allowed_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let hub = tmp.path().join("hub");
        std::fs::create_dir_all(&hub).unwrap();
        let allowed = vec![hub.clone()];
        let argv = vec!["grep".into(), "x".into(), "..".into()];
        assert!(
            !are_path_args_safe("grep", &argv, &allowed, &hub),
            "relative .. must resolve and fail path_under_allowed"
        );
    }

    #[test]
    fn path_arg_inside_hub_allowed() {
        let tmp = tempfile::tempdir().unwrap();
        let hub = tmp.path().join("hub");
        std::fs::create_dir_all(hub.join("src")).unwrap();
        let allowed = vec![hub.clone()];
        let argv = vec!["ls".into(), "src".into()];
        assert!(are_path_args_safe("ls", &argv, &allowed, &hub));
    }
}
