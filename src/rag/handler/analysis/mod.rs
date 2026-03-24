//! Verification, analysis, and code-structure tools: verify_integrity, verify_module_tree,
//! analyze_error_log, scaffold_reproduction_test, review_diff, security_audit, module_graph,
//! read_manifest, project_packer, scan_secrets, get_file_history.

mod prompts;
mod tools;

pub use prompts::{
    analyze_error_log_impl, build_analyze_error_log_text, build_review_diff_audit,
    build_scaffold_reproduction_test_text, review_diff_impl, scaffold_reproduction_test_impl,
    skeptic_review_impl,
};
pub use tools::{
    aggregate_audit_impl, get_file_history_impl, module_graph_impl, project_packer_impl,
    read_manifest_impl, scan_secrets_impl, security_audit_impl, verify_integrity_impl,
    verify_module_tree_impl,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use regex::Regex;
use std::sync::OnceLock;

// ---------- Params ----------

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// SecurityAuditParams.
pub struct SecurityAuditParams {
    /// Directory or file path to scan with Semgrep.
    pub path: String,
    /// Semgrep ruleset: "p/python", "p/rust", or omit for default auto rules.
    #[serde(default)]
    pub ruleset: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ModuleGraphParams. Unified params for Rust module structure: text tree (cargo-modules) or Mermaid.
pub struct ModuleGraphParams {
    /// Workspace path (directory containing Cargo.toml / src/). Default empty = first allowed root.
    #[serde(default)]
    pub workspace_path: String,
    /// Output format: "text" (cargo-modules structure) or "mermaid" (Mermaid diagram). Default "mermaid".
    #[serde(default = "default_module_graph_format")]
    pub format: String,
}

fn default_module_graph_format() -> String {
    "mermaid".to_string()
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// VerifyIntegrityParams.
pub struct VerifyIntegrityParams {
    /// Workspace path (directory containing Cargo.toml). Default empty = first allowed root.
    #[serde(default)]
    pub workspace_path: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ReadManifestParams.
pub struct ReadManifestParams {
    /// Workspace path (directory containing Cargo.toml). Default empty = first allowed root.
    #[serde(default)]
    pub workspace_path: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ProjectPackerParams.
pub struct ProjectPackerParams {
    /// Workspace path. Default empty = first allowed root.
    #[serde(default)]
    pub workspace_path: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// VerifyModuleTreeParams.
pub struct VerifyModuleTreeParams {
    /// Workspace path (directory containing src/). Default empty = first allowed root.
    #[serde(default)]
    pub workspace_path: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ScanSecretsParams.
pub struct ScanSecretsParams {
    /// Workspace path to scan. If empty, uses first allowed root.
    #[serde(default)]
    pub workspace_path: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GetFileHistoryParams.
pub struct GetFileHistoryParams {
    /// File path (relative to project root or absolute). Must be under ALLOWED_ROOTS.
    pub path: String,
    /// Optional start line for `git log -L` (inclusive).
    #[serde(default)]
    pub line_start: Option<u32>,
    /// Optional end line for `git log -L` (inclusive).
    #[serde(default)]
    pub line_end: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// AnalyzeErrorLogParams.
pub struct AnalyzeErrorLogParams {
    #[serde(default)]
    pub error_output: String,
    #[serde(default)]
    pub recent_errors: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ScaffoldReproductionTestParams.
pub struct ScaffoldReproductionTestParams {
    #[serde(default)]
    pub bug_description: String,
    #[serde(default)]
    pub error_output: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ReviewDiffParams.
pub struct ReviewDiffParams {
    #[serde(default)]
    pub diff: String,
    /// Pass "short" for a minimal security-only audit (APPROVE/REQUEST_CHANGES + brief issues). Default = full checklist.
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// SkepticReviewParams.
pub struct SkepticReviewParams {
    /// Unified diff or changed code to review.
    pub diff: String,
    /// Optional objective/context — what was the change trying to achieve?
    #[serde(default)]
    pub objective: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// AggregateAuditParams.
pub struct AggregateAuditParams {
    /// Workspace path (directory containing Cargo.toml). Default empty = first allowed root.
    #[serde(default)]
    pub workspace_path: String,
}

// ---------- Helpers (used by tools) ----------

const PROJECT_TREE_SKIP: &[&str] = &["node_modules", "target", ".git", "dist", "build", ".cursor"];
const PROJECT_TREE_MAX_DEPTH: usize = 4;
const MAX_PACKAGE_DEPS_FOR_SUMMARY: usize = 20;

/// Normalize path for display: backslashes to forward slashes.
pub(crate) fn normalize_path_display(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

struct SecretScanRegexes {
    assign: Regex,
    payment_key: Regex,
    google: Regex,
    github_pat: Regex,
    jwt: Regex,
    slack_token: Regex,
    aws_key: Regex,
}

/// Compiled-once regexes for secret scanning (file content, not output sanitization).
/// Cached via OnceLock to avoid recompiling on every scan_secrets tool call.
fn secret_scan_regexes() -> &'static SecretScanRegexes {
    static REGEXES: OnceLock<SecretScanRegexes> = OnceLock::new();
    REGEXES.get_or_init(|| SecretScanRegexes {
        assign: Regex::new(r#"(?i)(key|token|secret|password|api_key)\s*=\s*"([^"]{20,})""#)
            .expect("assign regex"),
        payment_key: Regex::new(r#""(sk_live_[^"]*|sk_test_[^"]*)"#).expect("payment key regex"),
        google: Regex::new(r#""(AIza[^"]{30,})"#).expect("google regex"),
        github_pat: Regex::new(r#""(ghp_[a-zA-Z0-9]{36}|github_pat_[^"]+)""#)
            .expect("github pat regex"),
        jwt: Regex::new(r#""(eyJ[a-zA-Z0-9_-]{20,}\.[a-zA-Z0-9_-]{20,}\.[a-zA-Z0-9_-]{20,})""#)
            .expect("jwt regex"),
        slack_token: Regex::new(r#""(xoxb-[0-9]+-[0-9]+-[a-zA-Z0-9]+)""#)
            .expect("slack token regex"),
        aws_key: Regex::new(r#""(AKIA[0-9A-Z]{16})""#).expect("aws access key regex"),
    })
}

/// Mask the value portion of a secret assignment so output doesn't leak the actual secret.
/// e.g. `API_KEY = "sk-abc123xyz"` → `API_KEY = "[REDACTED]"` (keeps key name visible for triage).
fn mask_secret_value(line: &str) -> String {
    // Truncate to 80 chars first for display.
    let snippet: String = line.chars().take(80).collect();
    // Replace quoted values of 8+ chars with [REDACTED].
    static MASK_RE: OnceLock<Regex> = OnceLock::new();
    let re = MASK_RE.get_or_init(|| {
        Regex::new(r#""[^"]{8,}""#).expect("mask_secret_value regex")
    });
    re.replace_all(&snippet, "\"[REDACTED]\"").into_owned()
}

/// Public so chaos runner and tests can invoke the same logic as scan_secrets tool.
pub fn run_secret_scan(root: &Path) -> Vec<(PathBuf, u32, String)> {
    let mut findings = Vec::new();
    let re = secret_scan_regexes();
    let (assign_re, payment_key_re, google_re, github_pat_re, jwt_re, slack_token_re, aws_key_re) = (
        &re.assign,
        &re.payment_key,
        &re.google,
        &re.github_pat,
        &re.jwt,
        &re.slack_token,
        &re.aws_key,
    );
    let skip_dirs = ["target", ".git", "node_modules"];
    let exts = [
        "rs", "ts", "tsx", "js", "jsx", "toml", "env", "json", "yaml", "yml",
    ];

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if path.components().any(|c| {
            c.as_os_str()
                .to_str()
                .map(|s| skip_dirs.contains(&s))
                .unwrap_or(false)
        }) {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !exts.contains(&ext) {
            continue;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (line_no, line) in content.lines().enumerate() {
            let line_num = (line_no + 1) as u32;
            let trimmed = line.trim();
            // Explicit opt-out annotation for intentional test fixtures.
            if trimmed.contains("scan-secrets-ignore") {
                continue;
            }
            if trimmed.contains("env::var")
                || trimmed.contains("env!(\"")
                || trimmed.contains("std::env::")
            {
                continue;
            }
            if assign_re.is_match(trimmed) {
                // Mask the value portion to avoid leaking secrets in output.
                let snippet = mask_secret_value(trimmed);
                findings.push((path.to_path_buf(), line_num, snippet));
                continue;
            }
            if payment_key_re.is_match(trimmed)
                || google_re.is_match(trimmed)
                || github_pat_re.is_match(trimmed)
                || jwt_re.is_match(trimmed)
                || slack_token_re.is_match(trimmed)
                || aws_key_re.is_match(trimmed)
            {
                let snippet = mask_secret_value(trimmed);
                findings.push((path.to_path_buf(), line_num, snippet));
            }
        }
    }
    findings
}

/// Parse [dependencies] and [dev-dependencies] from Cargo.toml content.
pub(crate) fn parse_cargo_toml_deps(content: &str) -> HashMap<String, String> {
    let mut deps = HashMap::new();
    if content.is_empty() {
        return deps;
    }
    let value: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(_) => return deps,
    };
    let table = match value.as_table() {
        Some(t) => t,
        None => return deps,
    };
    for section_key in &["dependencies", "dev-dependencies"] {
        if let Some(section) = table.get(*section_key).and_then(|v| v.as_table()) {
            extract_versions_from_section(section, &mut deps);
        }
    }
    if let Some(targets) = table.get("target").and_then(|v| v.as_table()) {
        for target_val in targets.values() {
            if let Some(t) = target_val.as_table() {
                for section_key in &["dependencies", "dev-dependencies"] {
                    if let Some(section) = t.get(*section_key).and_then(|v| v.as_table()) {
                        extract_versions_from_section(section, &mut deps);
                    }
                }
            }
        }
    }
    deps
}

pub(crate) fn extract_versions_from_section(
    section: &toml::map::Map<String, toml::Value>,
    out: &mut HashMap<String, String>,
) {
    for (name, val) in section {
        let version = match val {
            toml::Value::String(v) => v.clone(),
            toml::Value::Table(t) => t
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            _ => String::new(),
        };
        if !version.is_empty() {
            out.entry(name.clone()).or_insert(version);
        }
    }
}

/// Build a compressed directory tree under root.
pub(crate) fn build_project_tree(root: &Path) -> String {
    let root = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return format!("(cannot canonicalize root: {})", root.display()),
    };
    let mut lines = Vec::new();
    for entry in WalkDir::new(&root)
        .max_depth(PROJECT_TREE_MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            !PROJECT_TREE_SKIP.contains(&name)
        })
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let rel = match path.strip_prefix(&root) {
            Ok(r) => r,
            Err(_) => path,
        };
        let depth = rel.components().count();
        if depth == 0 {
            continue;
        }
        let indent = "  ".repeat(depth - 1);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        lines.push(format!("{}{}", indent, name));
    }
    if lines.is_empty() {
        " (empty or no accessible entries)".to_string()
    } else {
        lines.join("\n")
    }
}

/// Summarize package.json: name, scripts, dependencies.
pub(crate) fn summarize_package_json(path: &Path) -> String {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return "package.json not found".to_string(),
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return "package.json invalid JSON".to_string(),
    };
    let obj = match value.as_object() {
        Some(o) => o,
        None => return "package.json not an object".to_string(),
    };
    let mut out = Vec::new();
    if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
        out.push(format!("name: {}", name));
    }
    if let Some(scripts) = obj.get("scripts").and_then(|v| v.as_object()) {
        let keys: Vec<&str> = scripts.keys().map(|k| k.as_str()).collect();
        out.push(format!("scripts: {}", keys.join(", ")));
    }
    if let Some(deps) = obj.get("dependencies").and_then(|v| v.as_object()) {
        let keys: Vec<&str> = deps
            .keys()
            .take(MAX_PACKAGE_DEPS_FOR_SUMMARY)
            .map(|k| k.as_str())
            .collect();
        let more = if deps.len() > MAX_PACKAGE_DEPS_FOR_SUMMARY {
            " ..."
        } else {
            ""
        };
        out.push(format!(
            "dependencies: {} ({} total){}",
            keys.join(", "),
            deps.len(),
            more
        ));
    }
    if out.is_empty() {
        "package.json (no name/scripts/dependencies)".to_string()
    } else {
        out.join("\n")
    }
}
