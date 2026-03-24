//! UI, task queue, and rules tools: submit_task, get_ui_blueprint,
//! verify_ui_integrity, get_tool_selection_guide, get_design_tokens, fork_terminal, compile_rules.

use super::{
    read_rag_max_response_chars, truncate_for_budget, AgenticHandler, IngestionProvider,
    VectorStoreProvider,
};
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------- Params ----------

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// SubmitTaskParams.
pub struct SubmitTaskParams {
    pub task_type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GetUiBlueprintParams.
pub struct GetUiBlueprintParams {
    #[serde(default)]
    pub blueprint_type: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// VerifyUiIntegrityParams.
pub struct VerifyUiIntegrityParams {
    #[serde(default)]
    pub snippet: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GetToolSelectionGuideParams. Default (section=None, outline_only=None) = full doc. outline_only=true = outline only; section=id = single section.
pub struct GetToolSelectionGuideParams {
    /// Return only this section (slug id, e.g. "dont", "core_vs_niche", "situation_tool"). Ignored if outline_only is true.
    #[serde(default)]
    pub section: Option<String>,
    /// If true and section is None, return outline (section ids + titles) and hint to call with section for full content.
    #[serde(default)]
    pub outline_only: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// GetDesignTokensParams.
pub struct GetDesignTokensParams {
    #[serde(default)]
    pub token_category: String,
    #[serde(default)]
    pub base_path: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// ForkTerminalParams.
pub struct ForkTerminalParams {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub bypass_hitl: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// CompileRulesParams.
pub struct CompileRulesParams {
    #[serde(default)]
    pub active_project_path: String,
}

/// One section of the tool selection guide: id (slug), title (heading text), content (raw markdown).
#[derive(Clone, Debug)]
struct GuideSection {
    id: String,
    title: String,
    content: String,
}

/// Parse TOOL_SELECTION_GUIDE markdown into sections by ## headings. Returns (intro_opt, sections).
fn parse_guide_sections(content: &str) -> (Option<String>, Vec<GuideSection>) {
    let mut sections = Vec::new();
    let parts: Vec<&str> = content.split("\n## ").collect();
    let (intro, rest) = if parts.is_empty() {
        (None, &[] as &[&str])
    } else if parts.len() == 1 {
        (Some(parts[0].trim().to_string()), &[] as &[&str])
    } else {
        (Some(parts[0].trim().to_string()), &parts[1..])
    };
    for block in rest {
        let first_line = block.lines().next().unwrap_or("").trim();
        let title = first_line
            .strip_prefix('#')
            .map(str::trim)
            .unwrap_or(first_line)
            .to_string();
        let slug: String = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("_");
        let body = block
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
        let content = if body.is_empty() {
            format!("## {}", title)
        } else {
            format!("## {}\n\n{}", title, body)
        };
        sections.push(GuideSection {
            id: slug,
            title,
            content,
        });
    }
    (intro, sections)
}

/// Synapse manifest: optional list of paths (relative to rules root / project) to include when compiling rules. When present, compile_rules uses only these files instead of scanning Standards/Rules/Workflows.
#[derive(Clone, Debug, Deserialize)]
struct SynapseManifest {
    #[serde(default)]
    global: Vec<String>,
    #[serde(default)]
    project: Option<Vec<String>>,
}

// ---------- Constants and helpers ----------

/// Built-in task types for submit_task.
pub const BUILTIN_TASK_TYPES: &[&str] = &[
    "research",
    "research_ingest",
    "ingest",
    "refresh_file_index",
    "verify-integrity",
    "data-clean",
];

/// Returns the list of allowed task types.
pub fn allowed_task_types() -> Vec<String> {
    let mut types: Vec<String> = BUILTIN_TASK_TYPES
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    if let Ok(extra) = std::env::var("TASK_TYPES_EXTRA") {
        for s in extra.split(',') {
            let t = s.trim().to_string();
            if !t.is_empty() && !types.contains(&t) {
                types.push(t);
            }
        }
    }
    types
}

const UI_BLUEPRINT_DASHBOARD: &str = r#"<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
  <div class="flex flex-col lg:flex-row gap-4 border border-slate-200 rounded-lg bg-white">
    <aside class="w-full lg:w-64 flex-shrink-0 border-b lg:border-b-0 lg:border-r border-slate-200 p-4">
      <!-- Sidebar -->
    </aside>
    <main class="flex-1 min-w-0 p-4">
      <!-- Main content -->
    </main>
    <aside class="w-full lg:w-72 flex-shrink-0 border-t lg:border-t-0 lg:border-l border-slate-200 p-4">
      <!-- Context / Inspector -->
    </aside>
  </div>
</div>"#;

const UI_BLUEPRINT_FORM: &str = r#"<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
  <div class="border border-slate-200 rounded-lg p-6 bg-white">
    <!-- Form content -->
    <button type="submit" class="mt-4 px-4 py-2 bg-indigo-600 text-white rounded border border-slate-200 shadow-sm">Submit</button>
  </div>
</div>"#;

const UI_BLUEPRINT_SETTINGS: &str = r#"<div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
  <div class="flex flex-col lg:flex-row gap-4 border border-slate-200 rounded-lg bg-white">
    <aside class="w-full lg:w-56 flex-shrink-0 border-b lg:border-b-0 lg:border-r border-slate-200 p-4">
      <!-- Settings nav -->
    </aside>
    <main class="flex-1 min-w-0 p-4">
      <!-- Settings panel -->
    </main>
    <aside class="w-full lg:w-64 flex-shrink-0 border-t lg:border-t-0 lg:border-l border-slate-200 p-4 hidden xl:block">
      <!-- Help / hints -->
    </aside>
  </div>
</div>"#;

const FORBIDDEN_SHADOWS: &[&str] = &["shadow-md", "shadow-lg", "shadow-xl", "shadow-2xl"];

fn verify_ui_integrity_impl(snippet: &str) -> Vec<String> {
    let mut violations = Vec::new();
    for &shadow in FORBIDDEN_SHADOWS {
        if snippet.contains(shadow) {
            violations.push(format!(
                "Forbidden shadow: {} (use shadow-sm at most per DESIGN_AXIOMS)",
                shadow
            ));
        }
    }
    let has_flex_row = snippet.contains("flex-row") || snippet.contains("flex-row ");
    let has_grid =
        snippet.contains("grid") && (snippet.contains("grid-cols") || snippet.contains("lg:grid"));
    let has_horizontal = has_flex_row || has_grid;
    let w_full_count = snippet.matches("w-full").count();
    if w_full_count >= 2 && !has_horizontal {
        violations.push(
            "Stacking red flag: multiple w-full without horizontal layout (add flex flex-row or grid with columns for desktop per DESIGN_AXIOMS)".to_string(),
        );
    }
    violations
}

/// Public entry: returns (pass, violations). Used by verify_ui_integrity MCP tool.
pub fn verify_ui_integrity_check(snippet: &str) -> (bool, Vec<String>) {
    let violations = verify_ui_integrity_impl(snippet);
    (violations.is_empty(), violations)
}

// ---------- Impls ----------

pub async fn submit_task_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: SubmitTaskParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    use chrono::Utc;
    let task_type = params.task_type.trim();
    if task_type.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "submit_task requires a non-empty task_type (research, research_ingest, ingest, refresh_file_index, verify-integrity).",
        )]));
    }
    let allowed = allowed_task_types();
    if !allowed.iter().any(|t| t == task_type) {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "submit_task: invalid task_type '{}'. Allowed: {}.",
            task_type,
            allowed.join(", "),
        ))]));
    }
    let root = match handler.store.allowed_roots.first() {
        Some(r) => r.clone(),
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "submit_task: no allowed root (ALLOWED_ROOTS); cannot write to _tasks/inbox.",
            )]));
        }
    };
    let inbox_dir = root.join("_tasks").join("inbox");
    let payload = if params.payload.is_object() {
        params.payload
    } else {
        serde_json::json!({})
    };
    let created_at = Utc::now().to_rfc3339();
    let id = format!(
        "{}_{}",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        std::process::id()
    );
    let task_json = serde_json::json!({
        "id": id,
        "type": task_type,
        "payload": payload,
        "created_at": created_at,
    });
    if let Err(e) = std::fs::create_dir_all(&inbox_dir) {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "submit_task: failed to create inbox dir {}: {}",
            inbox_dir.display(),
            e,
        ))]));
    }
    let filename = format!("{}.json", id);
    let file_path = inbox_dir.join(&filename);
    match std::fs::write(
        &file_path,
        serde_json::to_string_pretty(&task_json).unwrap_or_else(|_| task_json.to_string()),
    ) {
        Ok(()) => Ok(CallToolResult::success(vec![Content::text(format!(
            "submit_task: wrote {} (type={})",
            file_path.display(),
            task_type,
        ))])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
            "submit_task: failed to write {}: {}",
            file_path.display(),
            e,
        ))])),
    }
}

pub async fn get_ui_blueprint_impl<I, S>(
    _handler: &AgenticHandler<I, S>,
    params: GetUiBlueprintParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let t = params.blueprint_type.trim().to_lowercase();
    let snippet = match t.as_str() {
        "form" => UI_BLUEPRINT_FORM,
        "settings" => UI_BLUEPRINT_SETTINGS,
        _ => UI_BLUEPRINT_DASHBOARD,
    };
    Ok(CallToolResult::success(vec![Content::text(
        snippet.to_string(),
    )]))
}

pub async fn verify_ui_integrity_impl_tool<I, S>(
    _handler: &AgenticHandler<I, S>,
    params: VerifyUiIntegrityParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let snippet = params.snippet.trim();
    let (pass, violations) = verify_ui_integrity_check(snippet);
    let json = serde_json::json!({
        "pass": pass,
        "violations": violations
    });
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&json)
            .unwrap_or_else(|_| format!("{{\"pass\":{}, \"violations\": []}}", pass)),
    )]))
}

pub async fn get_tool_selection_guide_impl<I, S>(
    _handler: &AgenticHandler<I, S>,
    params: GetToolSelectionGuideParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let path = _handler.tool_selection_guide_path.clone().or_else(|| {
        _handler
            .store
            .allowed_roots
            .first()
            .map(|r| r.join("docs").join("TOOL_SELECTION_GUIDE.md"))
    });
    let path = match path {
        Some(p) => p,
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "get_tool_selection_guide: no allowed root and TOOL_SELECTION_GUIDE_PATH not set.",
            )]));
        }
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "get_tool_selection_guide: could not read {}: {}",
                path.display(),
                e
            ))]));
        }
    };
    let (intro_opt, sections) = parse_guide_sections(&content);

    if params.outline_only == Some(true) && params.section.is_none() {
        let mut lines = Vec::new();
        if let Some(intro) = intro_opt {
            let first = intro.lines().next().unwrap_or("").trim();
            if !first.is_empty() {
                lines.push(format!("intro: {}", first));
            }
        }
        for s in &sections {
            lines.push(format!("{}: {}", s.id, s.title));
        }
        lines.push(String::new());
        lines.push("Call get_tool_selection_guide(section: <id>) for full content.".to_string());
        let outline = lines.join("\n");
        return Ok(CallToolResult::success(vec![Content::text(outline)]));
    }

    if let Some(ref id) = params.section {
        let id_trim = id.trim().to_lowercase();
        if let Some(s) = sections.iter().find(|s| s.id == id_trim) {
            let truncated = truncate_for_budget(&s.content, read_rag_max_response_chars());
            return Ok(CallToolResult::success(vec![Content::text(truncated)]));
        }
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "get_tool_selection_guide: no section '{}'. Known: {}",
            id,
            sections
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))]));
    }

    Ok(CallToolResult::success(vec![Content::text(
        truncate_for_budget(&content, read_rag_max_response_chars()),
    )]))
}

pub async fn get_design_tokens_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: GetDesignTokensParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let category = params.token_category.trim();
    if category.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_design_tokens requires a non-empty token_category (e.g. colors, typography).",
        )]));
    }
    let base = if let Some(ref bp) = params.base_path {
        let p = PathBuf::from(bp.trim());
        if !crate::rag::path_filter::path_under_allowed(&p, &handler.store.allowed_roots, false) {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "get_design_tokens: base_path '{}' is outside the allowed workspace roots.",
                bp
            ))]));
        }
        p
    } else {
        handler
            .design_tokens_dir
            .clone()
            .or_else(|| {
                handler
                    .store
                    .allowed_roots
                    .first()
                    .map(|r| r.join("docs").join("design").join("data"))
            })
            .unwrap_or_else(|| PathBuf::from("docs/design/data"))
    };
    let safe_name = category.replace(['/', '\\'], "_");
    // Title-case variant for repos that use e.g. Colors.csv (first letter uppercase)
    let title_name = safe_name
        .chars()
        .next()
        .map(|c| {
            c.to_uppercase()
                .chain(safe_name.chars().skip(1))
                .collect::<String>()
        })
        .unwrap_or_else(|| safe_name.clone());
    let candidates = [
        base.join(format!("{}.csv", safe_name)),
        base.join(format!("{}.csv", &title_name)),
        base.join(format!("{}.json", safe_name)),
        base.join(format!("{}.json", &title_name)),
    ];
    let path = match candidates.iter().find(|p| p.exists()) {
        Some(p) => p.clone(),
        None => {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "get_design_tokens: no file found for category '{}' at {} (tried {}.csv, {}.csv, {}.json, {}.json)",
                category,
                base.display(),
                safe_name,
                title_name,
                safe_name,
                title_name
            ))]));
        }
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
            "get_design_tokens: could not read {}: {}",
            path.display(),
            e
        ))])),
    }
}

pub async fn fork_terminal_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ForkTerminalParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    use super::shell;
    let command = params.command.trim();
    if command.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "fork_terminal requires a non-empty command.",
        )]));
    }
    if !shell::is_command_allowed(command) {
        return Ok(CallToolResult::success(vec![Content::text(
            shell::COMMAND_REJECTION_MESSAGE,
        )]));
    }
    if command.contains('\n') || command.contains('\r') {
        return Ok(CallToolResult::success(vec![Content::text(
            "Command rejected: newlines not allowed.",
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
    if argv[0].eq_ignore_ascii_case("git") && !shell::is_git_args_safe(&argv) {
        return Ok(CallToolResult::success(vec![Content::text(
            shell::GIT_DESTRUCTIVE_REJECT_MESSAGE.to_string(),
        )]));
    }
    if argv[0].eq_ignore_ascii_case("cargo") && !shell::is_cargo_args_safe(&argv) {
        return Ok(CallToolResult::success(vec![Content::text(
            "Command rejected: cargo subcommand not allowed (e.g. uninstall).",
        )]));
    }
    if argv[0].eq_ignore_ascii_case("npm") && !shell::is_npm_args_safe(&argv) {
        return Ok(CallToolResult::success(vec![Content::text(
            "Command rejected: npm subcommand not allowed (e.g. uninstall).",
        )]));
    }
    let project_root = handler
        .store
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let cwd = params
        .working_dir
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.clone());
    if !crate::rag::path_filter::path_under_allowed(&cwd, &handler.store.allowed_roots, false) {
        return Ok(CallToolResult::success(vec![Content::text(
            "fork_terminal: working_dir must be under an allowed workspace root (ALLOWED_ROOTS).",
        )]));
    }
    #[cfg(windows)]
    let title = params
        .title
        .clone()
        .unwrap_or_else(|| "MCP fork_terminal".to_string());
    let result = tokio::task::spawn_blocking(move || {
        #[cfg(windows)]
        {
            std::process::Command::new("cmd")
                .args([
                    "/c",
                    "start",
                    title.as_str(),
                    "/d",
                    cwd.to_string_lossy().as_ref(),
                    "cmd",
                    "/k",
                ])
                .arg(&argv[0])
                .args(&argv[1..])
                .spawn()
                .map(|_| ())
        }
        #[cfg(not(windows))]
        {
            std::process::Command::new(&argv[0])
                .args(&argv[1..])
                .current_dir(&cwd)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .map(|_| ())
        }
    })
    .await
    .map_err(|e| McpError::internal_error(format!("fork_terminal spawn: {}", e), None))?;
    match result {
        Ok(_) => Ok(CallToolResult::success(vec![Content::text(
            "fork_terminal: command started in a new terminal.",
        )])),
        Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
            "fork_terminal: failed to start: {}",
            e
        ))])),
    }
}

pub async fn compile_rules_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: CompileRulesParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let global_dir = match &handler.global_rules_dir {
        Some(d) if d.exists() => d.clone(),
        Some(_) => {
            return Ok(CallToolResult::success(vec![Content::text(
                "compile_rules: RULES_VAULT/GLOBAL_RULES_DIR is set but the path does not exist.",
            )]));
        }
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "compile_rules: Set RULES_VAULT or GLOBAL_RULES_DIR to use this tool.",
            )]));
        }
    };
    let project_path = params.active_project_path.trim();
    if project_path.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "compile_rules requires a non-empty active_project_path (target project root).",
        )]));
    }
    let project_root = PathBuf::from(project_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(project_path));
    if !crate::rag::path_filter::path_under_allowed(
        &project_root,
        &handler.store.allowed_roots,
        false,
    ) {
        return Ok(CallToolResult::success(vec![Content::text(
            "compile_rules: active_project_path must be under an allowed workspace root (ALLOWED_ROOTS).",
        )]));
    }
    if !project_root.exists() {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "compile_rules: project path does not exist: {}",
            project_root.display()
        ))]));
    }

    let mut sections: Vec<String> = vec!["# Compiled rules\n".to_string()];

    // Optional synapse manifest: if present, compile only listed files (paths relative to global_dir / project).
    let manifest_path = std::env::var("SYNAPSE_MANIFEST_PATH")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| Some(global_dir.join("synapse_manifest.json")));
    let use_manifest = manifest_path.as_ref().and_then(|p| {
        if p.exists() {
            let content = std::fs::read_to_string(p).ok()?;
            let manifest: SynapseManifest = serde_json::from_str(&content).ok()?;
            Some(manifest)
        } else {
            None
        }
    });

    if let Some(manifest) = use_manifest {
        for rel in &manifest.global {
            let full = global_dir.join(rel);
            if full.exists() {
                if let Ok(content) = std::fs::read_to_string(&full) {
                    sections.push(format!("\n### {}\n{}\n", rel.replace('\\', "/"), content));
                }
            }
        }
        if let Some(project) = &manifest.project {
            for rel in project {
                let full = project_root.join(rel);
                if full.exists() {
                    if let Ok(content) = std::fs::read_to_string(&full) {
                        sections.push(format!("\n### {}\n{}\n", rel.replace('\\', "/"), content));
                    }
                }
            }
        }
    } else {
        for sub in ["Standards", "Rules", "Workflows"] {
            let sub_dir = global_dir.join(sub);
            if sub_dir.exists() {
                sections.push(format!("\n## --- {} ---\n", sub));
                let mut files: Vec<_> = walkdir::WalkDir::new(&sub_dir)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .is_some_and(|ext| ext == "md" || ext == "mdc")
                    })
                    .map(|e| e.path().to_path_buf())
                    .collect();
                files.sort();
                for f in files {
                    if let Ok(content) = std::fs::read_to_string(&f) {
                        let rel = f.strip_prefix(&sub_dir).unwrap_or(&f);
                        sections.push(format!("\n### {}\n{}\n", rel.display(), content));
                    }
                }
            }
        }
    }
    let context_dir = project_root.join(".context");
    if context_dir.exists() {
        sections.push("\n## --- Project context ---\n".to_string());
        let mut files: Vec<_> = std::fs::read_dir(&context_dir)
            .map_err(|e| {
                McpError::internal_error(format!("compile_rules read .context: {}", e), None)
            })?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.path())
            .collect();
        files.sort();
        for f in files {
            if let Ok(content) = std::fs::read_to_string(&f) {
                let name = f.file_name().unwrap_or_default().to_string_lossy();
                sections.push(format!("\n### {}\n{}\n", name, content));
            }
        }
    }
    let merged = sections.join("\n");
    let cursorrules_path = project_root.join(".cursorrules");
    std::fs::write(&cursorrules_path, &merged).map_err(|e| {
        McpError::internal_error(format!("compile_rules write .cursorrules: {}", e), None)
    })?;
    let mut written = vec![cursorrules_path.display().to_string()];
    let gemini_path = project_root.join("GEMINI.md");
    let claude_path = project_root.join("CLAUDE.md");
    if let Err(e) = std::fs::write(&gemini_path, &merged) {
        tracing::warn!("compile_rules: failed to write GEMINI.md: {}", e);
    } else {
        written.push(gemini_path.display().to_string());
    }
    if let Err(e) = std::fs::write(&claude_path, &merged) {
        tracing::warn!("compile_rules: failed to write CLAUDE.md: {}", e);
    } else {
        written.push(claude_path.display().to_string());
    }
    Ok(CallToolResult::success(vec![Content::text(format!(
        "compile_rules: wrote {}",
        written.join(", ")
    ))]))
}
