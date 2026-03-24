//! Analysis tool implementations: verify_integrity, verify_module_tree, read_manifest,
//! project_packer, scan_secrets, get_file_history, security_audit, module_graph.

use super::super::{
    internal_error_sanitized, read_rag_max_response_chars, sanitize_shell_output,
    truncate_for_budget, AgenticHandler, IngestionProvider, VectorStoreProvider,
};
use super::{
    build_project_tree, normalize_path_display, parse_cargo_toml_deps, run_secret_scan,
    summarize_package_json, AggregateAuditParams, GetFileHistoryParams, ModuleGraphParams,
    ProjectPackerParams, ReadManifestParams, ScanSecretsParams, SecurityAuditParams,
    VerifyIntegrityParams, VerifyModuleTreeParams,
};
use crate::rag::cli_helpers::run_verify_integrity;
use crate::rag::path_filter::path_under_allowed;
use crate::rag::symbols;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use std::path::PathBuf;
use std::process::Command;

/// Resolve `workspace_path` from MCP params against `allowed_roots`.
///
/// - Empty string → first allowed root (or CWD fallback).
/// - Relative path → joined onto first allowed root, then canonicalized.
///   This prevents Windows "os error 267" (`ERROR_DIRECTORY`) that occurs when
///   `Command::current_dir` receives a path relative to the MCP server's CWD,
///   which is often a system directory (e.g. `C:\Windows\System32`) rather than
///   the project root.
/// - Absolute path → canonicalized to normalize UNC/drive-letter variants.
fn resolve_workspace_path(workspace_path: &str, allowed_roots: &[PathBuf]) -> PathBuf {
    let fallback = || {
        allowed_roots
            .first()
            .cloned()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    };
    if workspace_path.is_empty() {
        return fallback();
    }
    let raw = PathBuf::from(workspace_path);
    let resolved = if raw.is_absolute() {
        raw
    } else {
        fallback().join(raw)
    };
    resolved.canonicalize().unwrap_or(resolved)
}

pub async fn security_audit_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: SecurityAuditParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let path = params.path.trim();
    let ruleset = params
        .ruleset
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    if path.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "security_audit requires a non-empty path.",
        )]));
    }
    let resolved_path = PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path));
    if !path_under_allowed(&resolved_path, &handler.store.allowed_roots, false) {
        return Ok(CallToolResult::success(vec![Content::text(
            "security_audit: path must be under an allowed workspace root (ALLOWED_ROOTS).",
        )]));
    }
    let path_owned = path.to_string();
    let ruleset_owned = ruleset.map(String::from);
    let allowed_roots_clone = handler.store.allowed_roots.clone();
    let output_result = tokio::task::spawn_blocking(move || {
        let mut cmd = Command::new("semgrep");
        cmd.arg("scan").arg("--json").arg(&path_owned);
        if let Some(ref r) = ruleset_owned {
            // Only allow Semgrep registry rulesets (p/..., r/...) or absolute paths under allowed roots.
            let is_registry = r.starts_with("p/") || r.starts_with("r/");
            let is_safe_path = crate::rag::path_filter::path_under_allowed(
                std::path::Path::new(r),
                &allowed_roots_clone,
                false,
            );
            if is_registry || is_safe_path {
                cmd.arg("--config").arg(r);
            }
            // Silently ignore invalid rulesets — semgrep defaults are sufficient.
        }
        cmd.output()
    })
    .await
    .map_err(|e| McpError::internal_error(format!("spawn_blocking: {}", e), None))?;

    let output = match output_result {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CallToolResult::success(vec![Content::text(
                "Semgrep not installed. Install from https://semgrep.dev and ensure it is in PATH.",
            )]));
        }
        Err(e) => {
            return Err(McpError::internal_error(format!("semgrep io: {}", e), None));
        }
    };
    let raw = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found")
            || stderr.contains("command not found")
            || output.status.code() == Some(127)
        {
            return Ok(CallToolResult::success(vec![Content::text(
                "Semgrep not installed. Install from https://semgrep.dev and ensure it is in PATH.",
            )]));
        }
        format!(
            "Semgrep exited with code {:?}\nstderr:\n{}",
            output.status.code(),
            stderr
        )
    };
    let text = truncate_for_budget(&raw, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn module_graph_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ModuleGraphParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let workspace_path = params.workspace_path.trim();
    let format_lower = params.format.trim().to_lowercase();
    let is_text = format_lower == "text";

    if is_text && workspace_path.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "module_graph with format \"text\" requires a non-empty workspace_path (cargo-modules runs in that directory).",
        )]));
    }

    let resolved_ws = resolve_workspace_path(workspace_path, &handler.store.allowed_roots);
    if !path_under_allowed(&resolved_ws, &handler.store.allowed_roots, false) {
        return Ok(CallToolResult::success(vec![Content::text(
            "module_graph: workspace_path must be under an allowed workspace root (ALLOWED_ROOTS).",
        )]));
    }

    if is_text {
        let result = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new("cargo");
            cmd.arg("modules")
                .arg("structure")
                .current_dir(&resolved_ws);
            cmd.output()
        })
        .await
        .map_err(|e| McpError::internal_error(format!("spawn_blocking: {}", e), None))?
        .map_err(|e| McpError::internal_error(format!("cargo io: {}", e), None))?;
        let raw = if result.status.success() {
            let stdout = String::from_utf8_lossy(&result.stdout).into_owned();
            let n = stdout.lines().count();
            format!(
                "Generated module structure ({} lines). Paste into a viewer or use for architecture overview.\n\n{}",
                n, stdout
            )
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
            if result.status.code() == Some(101)
                || stderr.contains("unknown subcommand")
                || stderr.contains("is not a valid")
            {
                return Ok(CallToolResult::success(vec![Content::text(
                    "cargo-modules not found. Install with: cargo install cargo-modules.",
                )]));
            }
            format!(
                "cargo modules failed: {}\nstderr:\n{}",
                result.status, stderr
            )
        };
        let text = truncate_for_budget(&raw, read_rag_max_response_chars());
        return Ok(CallToolResult::success(vec![Content::text(text)]));
    }

    // mermaid (default)
    let src_root = resolved_ws.join("src");
    if !src_root.exists() {
        return Ok(CallToolResult::success(vec![Content::text(
            "module_graph: src/ directory not found. Pass workspace_path (directory containing Cargo.toml) or ensure first allowed root has src/.",
        )]));
    }
    let mermaid = symbols::generate_module_graph(&src_root)
        .map_err(|e| McpError::internal_error(format!("module_graph: {}", e), None))?;
    let raw = if mermaid.trim().is_empty() {
        "No Rust modules found under src/.".to_string()
    } else {
        format!(
            "Here is the current architecture graph:\n\n```mermaid\n{}\n```",
            mermaid
        )
    };
    let text = truncate_for_budget(&raw, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn verify_integrity_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: VerifyIntegrityParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let workspace_path = params.workspace_path.trim();
    let mut project_root: PathBuf =
        resolve_workspace_path(workspace_path, &handler.store.allowed_roots);
    if !project_root.join("Cargo.toml").exists() {
        let monolith = project_root.join("monolith");
        if monolith.join("Cargo.toml").exists() {
            project_root = monolith;
        }
    }

    let result = tokio::task::spawn_blocking(move || run_verify_integrity(&project_root))
        .await
        .map_err(|e| internal_error_sanitized("verify_integrity", &e))?;
    let text = truncate_for_budget(&result, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn verify_module_tree_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: VerifyModuleTreeParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let workspace_path = params.workspace_path.trim();
    let src_root: PathBuf =
        resolve_workspace_path(workspace_path, &handler.store.allowed_roots).join("src");
    if !src_root.exists() {
        return Ok(CallToolResult::success(vec![Content::text(
            "verify_module_tree: src/ directory not found. Pass workspace_path (directory containing Cargo.toml) or ensure first allowed root has src/.",
        )]));
    }
    let (_reachable, unreachable) = symbols::modules_reachable_from_root(&src_root)
        .map_err(|e| McpError::internal_error(format!("verify_module_tree: {}", e), None))?;
    let msg = if unreachable.is_empty() {
        "All modules are wired (reachable from lib.rs or main.rs).".to_string()
    } else {
        format!(
            "The following modules are not reachable from lib.rs or main.rs: {}. Add the missing mod declaration (e.g. mod auth; or mod bar;) in the appropriate parent.",
            unreachable.join(", ")
        )
    };
    Ok(CallToolResult::success(vec![Content::text(msg)]))
}

pub async fn read_manifest_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ReadManifestParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let workspace_path = params.workspace_path.trim();
    let project_root: PathBuf =
        resolve_workspace_path(workspace_path, &handler.store.allowed_roots);
    let cargo_toml = project_root.join("Cargo.toml");
    let inner = tokio::task::spawn_blocking(move || {
        let content = std::fs::read_to_string(&cargo_toml)
            .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;
        let deps = parse_cargo_toml_deps(&content);
        serde_json::to_string(&deps).map_err(|e| format!("JSON: {}", e))
    })
    .await
    .map_err(|e| McpError::internal_error(format!("read_manifest spawn: {}", e), None))?;
    let text = inner.map_err(|e| McpError::invalid_params(e, None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn project_packer_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ProjectPackerParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let workspace_path = params.workspace_path.trim();
    let project_root: PathBuf =
        resolve_workspace_path(workspace_path, &handler.store.allowed_roots);
    let root = project_root.clone();
    let out = tokio::task::spawn_blocking(move || {
        let tree = build_project_tree(&root);
        let cargo_toml = root.join("Cargo.toml");
        let cargo_section = if cargo_toml.exists() {
            std::fs::read_to_string(&cargo_toml)
                .map(|c| {
                    let deps = parse_cargo_toml_deps(&c);
                    format!("dependencies (crate -> version): {:?}", deps)
                })
                .unwrap_or_else(|_| "Cargo.toml unreadable".to_string())
        } else {
            "Cargo.toml not found".to_string()
        };
        let package_section = summarize_package_json(&root.join("package.json"));
        format!(
            "## Tree\n{}\n\n## Cargo.toml\n{}\n\n## package.json\n{}",
            tree, cargo_section, package_section
        )
    })
    .await
    .map_err(|e| McpError::internal_error(format!("project_packer spawn: {}", e), None))?;
    let text = truncate_for_budget(&out, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn scan_secrets_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ScanSecretsParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let workspace_path = params.workspace_path.trim();
    let project_root: PathBuf =
        resolve_workspace_path(workspace_path, &handler.store.allowed_roots);
    let root = project_root.clone();
    let findings = tokio::task::spawn_blocking(move || run_secret_scan(&root))
        .await
        .map_err(|e| internal_error_sanitized("scan_secrets", &e))?;
    let list: Vec<serde_json::Value> = findings
        .into_iter()
        .map(|(p, line, snippet)| {
            serde_json::json!({
                "path": normalize_path_display(&p),
                "line": line,
                "snippet": snippet,
            })
        })
        .collect();
    let json = serde_json::json!({ "findings": list, "ok": list.is_empty() });
    Ok(CallToolResult::success(vec![Content::text(
        json.to_string(),
    )]))
}

pub async fn get_file_history_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: GetFileHistoryParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let path_arg = params.path.trim().to_string();
    if path_arg.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_file_history requires a non-empty path.",
        )]));
    }
    let project_root: PathBuf = handler
        .store
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let path_buf = PathBuf::from(&path_arg);
    let canonical = if path_buf.is_absolute() {
        path_buf.canonicalize().map_err(|e| {
            McpError::invalid_params(
                format!("get_file_history: path not found or not absolute: {}", e),
                None,
            )
        })?
    } else {
        project_root.join(&path_arg).canonicalize().map_err(|e| {
            McpError::invalid_params(format!("get_file_history: path not found: {}", e), None)
        })?
    };
    if !canonical.is_file() {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_file_history: path is not a file.",
        )]));
    }
    let under_allowed = handler
        .store
        .allowed_roots
        .iter()
        .any(|r| canonical.starts_with(r));
    if !under_allowed {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_file_history: path is not under ALLOWED_ROOTS.",
        )]));
    }
    let line_start = params.line_start;
    let line_end = params.line_end;
    let path_for_git: PathBuf = canonical
        .strip_prefix(&project_root)
        .map(PathBuf::from)
        .unwrap_or_else(|_| canonical.clone());
    let result = tokio::task::spawn_blocking(move || {
        let path_str = path_for_git.to_string_lossy().replace('\\', "/");
        let output = if let (Some(s), Some(e)) = (line_start, line_end) {
            std::process::Command::new("git")
                .args([
                    "log",
                    "-L",
                    &format!("{},{}:{}", s, e, path_str),
                    "-n",
                    "5",
                    "--",
                ])
                .current_dir(&project_root)
                .output()
        } else {
            std::process::Command::new("git")
                .args(["blame", "-l", &path_str])
                .current_dir(&project_root)
                .output()
        };
        match output {
            Ok(o) => {
                let raw = String::from_utf8_lossy(if o.status.success() {
                    &o.stdout
                } else {
                    &o.stderr
                });
                sanitize_shell_output(&raw)
            }
            Err(e) => format!("git failed: {}", e),
        }
    })
    .await
    .map_err(|e| internal_error_sanitized("get_file_history", &e))?;
    Ok(CallToolResult::success(vec![Content::text(result)]))
}

/// Aggregate audit: combines verify_integrity + scan_secrets + verify_module_tree
/// into a single structured report with an overall score (0.0-1.0).
pub async fn aggregate_audit_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: AggregateAuditParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let workspace_path = params.workspace_path.trim();
    let project_root = resolve_workspace_path(workspace_path, &handler.store.allowed_roots);

    let mut violations: Vec<serde_json::Value> = Vec::new();
    let mut checks_passed = 0u32;
    let mut checks_total = 0u32;

    // 1. verify_integrity (cargo check + test + clippy)
    checks_total += 1;
    let integrity_root = project_root.clone();
    let integrity_result = tokio::task::spawn_blocking({
        let mut root = integrity_root;
        move || {
            if !root.join("Cargo.toml").exists() {
                let monolith = root.join("monolith");
                if monolith.join("Cargo.toml").exists() {
                    root = monolith;
                }
            }
            run_verify_integrity(&root)
        }
    })
    .await
    .map_err(|e| internal_error_sanitized("aggregate_audit (integrity)", &e))?;

    let integrity_pass = integrity_result.contains("\"pass\": true")
        || integrity_result.contains("\"pass\":true");
    if integrity_pass {
        checks_passed += 1;
    } else {
        violations.push(serde_json::json!({
            "check": "verify_integrity",
            "severity": "critical",
            "detail": "cargo check/test/clippy failed"
        }));
    }

    // 2. scan_secrets
    checks_total += 1;
    let secrets_root = project_root.clone();
    let findings = tokio::task::spawn_blocking(move || run_secret_scan(&secrets_root))
        .await
        .map_err(|e| internal_error_sanitized("aggregate_audit (secrets)", &e))?;

    if findings.is_empty() {
        checks_passed += 1;
    } else {
        for (path, line, snippet) in &findings {
            violations.push(serde_json::json!({
                "check": "scan_secrets",
                "severity": "critical",
                "detail": format!("{}:{} — {}", normalize_path_display(path), line, snippet)
            }));
        }
    }

    // 3. verify_module_tree
    checks_total += 1;
    let src_root = project_root.join("src");
    if src_root.exists() {
        match symbols::modules_reachable_from_root(&src_root) {
            Ok((_reachable, unreachable)) => {
                if unreachable.is_empty() {
                    checks_passed += 1;
                } else {
                    violations.push(serde_json::json!({
                        "check": "verify_module_tree",
                        "severity": "high",
                        "detail": format!("Phantom modules: {}", unreachable.join(", "))
                    }));
                }
            }
            Err(e) => {
                violations.push(serde_json::json!({
                    "check": "verify_module_tree",
                    "severity": "medium",
                    "detail": format!("Module tree check failed: {}", e)
                }));
            }
        }
    } else {
        checks_passed += 1; // No src/ means not applicable
    }

    let score = if checks_total > 0 {
        checks_passed as f32 / checks_total as f32
    } else {
        1.0
    };

    let report = serde_json::json!({
        "score": score,
        "checks_passed": checks_passed,
        "checks_total": checks_total,
        "valid": violations.is_empty(),
        "violations": violations,
        "workspace": normalize_path_display(&project_root),
    });

    let text = serde_json::to_string_pretty(&report)
        .map_err(|e| McpError::internal_error(format!("aggregate_audit JSON: {}", e), None))?;
    let text = truncate_for_budget(&text, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}
