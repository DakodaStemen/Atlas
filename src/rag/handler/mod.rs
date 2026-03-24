//! Monolith MCP handler: RAG (query_knowledge, get_related_code, resolve_symbol) + tools.
//! Request flow: client sends initialize → server responds with capabilities; then tools/list (list tools);
//! then tools/call (invoke a tool by name with arguments). Resources: graph://symbol/{name}, rag://index/{path}.
//! Tools: security_audit, module_graph, get_system_status, execute_shell_command,
//! commit_to_memory, save_rule_to_memory, refresh_file_index, verify_integrity, verify_module_tree, read_manifest, fetch_web_markdown,
//! ingest_web_context, research_and_verify, get_ui_blueprint, verify_ui_integrity.
//! Schema parity for Cursor/Antigravity. Constants: MAX_EXTRA, MAX_CALLEES_IN_RELATED, WEB_FALLBACK_MAX_URLS.
//! query_knowledge: RAG → rerank; execute=true returns context for IDE to synthesize. execute_shell_command: allowlist permits only cargo, git, grep, ls, npm (is_command_allowed enforces this).

use crate::rag::chunking::chunk_file;
use crate::rag::dataset_collector::DatasetCollector;
use crate::rag::store::{format_sandbox_response, RagStore, EMPTY_RAG_CONTEXT};
use anyhow::Result as AnyhowResult;
use regex::Regex;
use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    AnnotateAble, CallToolRequestParams, CallToolResult, Content, GetPromptRequestParams,
    GetPromptResult, ListPromptsResult, ListResourceTemplatesResult, ListToolsResult, Meta,
    PaginatedRequestParams, PromptMessage, PromptMessageRole, RawResourceTemplate,
    ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities,
    ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::ErrorData as McpError;
use rmcp::RoleServer;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;
use tokio::task_local;
use tracing::warn;

mod analysis;
mod control_loop;
mod format_response;
mod memory;
mod model_routing;
mod planning;
mod providers;
mod rag;
mod shell;
mod skills;
mod ui;
mod web;

pub use control_loop::{GetLoopStateParams, LoopState, LoopStatus, ManagedLoop};
pub use model_routing::{AIModel, ModelRole, Phase, RouteTaskParams, RoutingDecision};
pub use planning::{Plan, PlanStep, PlanTaskParams};
pub use providers::{DefaultIngestion, DefaultStorage, IngestionProvider, VectorStoreProvider};
pub use shell::{ExecuteShellCommandParams, GetSystemStatusParams, GitCheckpointParams};

pub use analysis::{
    run_secret_scan, AggregateAuditParams, AnalyzeErrorLogParams, GetFileHistoryParams,
    ModuleGraphParams, ProjectPackerParams, ReadManifestParams, ReviewDiffParams,
    ScaffoldReproductionTestParams, ScanSecretsParams, SecurityAuditParams, SkepticReviewParams,
    VerifyIntegrityParams, VerifyModuleTreeParams,
};
pub use memory::{
    ApprovePatternParams, CommitToMemoryParams, LogTrainingRowParams, RefreshFileIndexParams,
    SaveRuleToMemoryParams,
};
pub use rag::{
    symbol_xml, GetCodebaseOutlineParams, GetDocOutlineParams, GetRelatedCodeParams,
    GetRelevantToolsParams, GetSectionParams, InvokeToolParams, QueryKnowledgeParams,
    QueryMasterResearchParams, ResolveSymbolParams,
};
pub use skills::{
    GetSkillContentParams, GetSkillReferenceParams, ListSkillMetadataParams, ValidateSkillParams,
};
pub use ui::{
    allowed_task_types, verify_ui_integrity_check, CompileRulesParams, ForkTerminalParams,
    GetDesignTokensParams, GetToolSelectionGuideParams, GetUiBlueprintParams, SubmitTaskParams,
    VerifyUiIntegrityParams, BUILTIN_TASK_TYPES,
};
pub use web::{
    classify_web_source_type, ingest_web_items_to_rag, parse_verification_agent_response,
    FetchWebMarkdownParams, IngestWebContextParams, ResearchAndVerifyParams, SearchWebParams,
    WebIngestItem, WebSnippetItem,
};

/// Hierarchical/graph search: max extra chunk IDs to add from symbol defines.
const MAX_EXTRA: usize = 10;
/// Max web URLs to fetch full content for when ingesting (e.g. ingest_web_context).
const WEB_FALLBACK_MAX_URLS: usize = 3;

/// Default tool set for list_tools (minimal subset for token savings).
/// Gateway pattern: use get_relevant_tools + invoke_tool for everything else. Set MCP_FULL_TOOLS=1 only for debugging.
const MINIMAL_TOOL_NAMES: &[&str] = &[
    "query_knowledge",
    "get_relevant_tools",
    "invoke_tool",
    "get_loop_state",
    "plan_task",
    "git_checkpoint",
];

/// Tool names that are idempotent (safe for gateway retries). Exposed in list_tools meta.
const IDEMPOTENT_TOOL_NAMES: &[&str] = &[
    "analyze_error_log",
    "fetch_web_markdown",
    "get_codebase_outline",
    "get_design_tokens",
    "get_doc_outline",
    "get_file_history",
    "get_related_code",
    "get_relevant_tools",
    "get_section",
    "get_skill_content",
    "get_skill_reference",
    "get_system_status",
    "get_tool_selection_guide",
    "get_ui_blueprint",
    "list_skill_metadata",
    "module_graph",
    "project_packer",
    "query_knowledge",
    "query_master_research",
    "read_manifest",
    "resolve_symbol",
    "review_diff",
    "search_web",
    "security_audit",
    "validate_skill",
    "validate_tool_params",
    "verify_ui_integrity",
    "get_loop_state",
];

/// Return an MCP error for the client with a generic message; log the full error server-side to avoid exposing paths or PII in client responses.
fn internal_error_sanitized(tool_name: &str, e: &impl std::fmt::Display) -> McpError {
    warn!("{} failed (server): {}", tool_name, e);
    McpError::internal_error(format!("{} failed.", tool_name), None)
}

/// When true, write tools (commit_to_memory, save_rule_to_memory, refresh_file_index, etc.) return a no-op success message.
pub(crate) fn mcp_read_only() -> bool {
    std::env::var("MCP_READ_ONLY")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true"))
        .unwrap_or(false)
}

/// Max chars for LOCAL_RAG and NEW_WEB_CONTENT when building research_and_verify context (kept for future use).
const VERIFICATION_AGENT_MAX_CHARS: usize = 4000;

task_local! {
    pub(crate) static REQUEST_ID: String;
}

/// Returns the current request ID if set (inside a tool call scope), else empty string.
pub(crate) fn current_request_id() -> String {
    REQUEST_ID
        .try_with(|s| s.clone())
        .unwrap_or_else(|_| String::new())
}

fn gen_request_id() -> String {
    let raw = format!(
        "{:016x}{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64,
        rand::random::<u64>()
    );
    blake3::hash(raw.as_bytes())
        .to_hex()
        .chars()
        .take(16)
        .collect()
}

/// Deterministic checksum of the tool list (names + inputSchema) for client-side rug-pull detection.
fn tool_list_checksum(tools: &[rmcp::model::Tool]) -> String {
    let mut entries: Vec<(String, String)> = tools
        .iter()
        .map(|t| {
            let name = t.name.as_ref().to_string();
            let schema = serde_json::to_string(&t.input_schema).unwrap_or_default();
            (name, schema)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let canonical = serde_json::to_string(&entries).unwrap_or_default();
    blake3::hash(canonical.as_bytes())
        .to_hex()
        .chars()
        .take(16)
        .collect()
}

/// Loop guard: per-process ring buffer of (tool_name, args_hash). When the same tool with same args is called
/// MCP_LOOP_GUARD_THRESHOLD times in a row, the next call returns an error and the buffer is cleared.
static LOOP_GUARD_BUFFER: OnceLock<Mutex<VecDeque<(String, String)>>> = OnceLock::new();

/// Session tool call count (process lifetime). When MCP_MAX_TOOL_CALLS_PER_SESSION is set, calls beyond the limit return an error.
static SESSION_TOOL_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

/// Pre-execution validation for path-taking tools. Returns (valid, warnings). Unknown tools or tools without path params return (true, []).
fn validate_tool_params_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    name: &str,
    arguments: Option<&serde_json::Value>,
) -> (bool, Vec<String>)
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let allowed = &handler.store.allowed_roots;
    let first_root = allowed
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let resolve = |path_str: &str| -> PathBuf {
        let path_str = path_str.trim();
        if path_str.is_empty() {
            return first_root.clone();
        }
        let p = PathBuf::from(path_str);
        if p.is_absolute() {
            p.canonicalize().unwrap_or(p)
        } else {
            first_root
                .join(&p)
                .canonicalize()
                .unwrap_or_else(|_| first_root.join(&p))
        }
    };
    let mut warnings = Vec::new();
    let args = match arguments.and_then(|v| v.as_object()) {
        Some(m) => m,
        None => return (true, warnings),
    };
    match name {
        "refresh_file_index" => {
            if let Some(arr) = args.get("paths").and_then(|v| v.as_array()) {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        let resolved = resolve(s);
                        if !crate::rag::path_filter::path_under_allowed(&resolved, allowed, false) {
                            warnings.push(format!("Path not under ALLOWED_ROOTS: {}", s));
                        }
                    }
                }
            }
        }
        "security_audit" | "get_file_history" => {
            if let Some(s) = args.get("path").and_then(|v| v.as_str()) {
                let resolved = resolve(s);
                if !crate::rag::path_filter::path_under_allowed(&resolved, allowed, false) {
                    warnings.push(format!("Path not under ALLOWED_ROOTS: {}", s));
                }
            }
        }
        "module_graph" => {
            if let Some(s) = args.get("workspace_path").and_then(|v| v.as_str()) {
                if !s.trim().is_empty() {
                    let resolved = resolve(s);
                    if !crate::rag::path_filter::path_under_allowed(&resolved, allowed, false) {
                        warnings.push(format!("workspace_path not under ALLOWED_ROOTS: {}", s));
                    }
                }
            }
        }
        "compile_rules" => {
            if let Some(s) = args.get("active_project_path").and_then(|v| v.as_str()) {
                if !s.trim().is_empty() {
                    let resolved = resolve(s);
                    if !crate::rag::path_filter::path_under_allowed(&resolved, allowed, false) {
                        warnings.push(format!(
                            "active_project_path not under ALLOWED_ROOTS: {}",
                            s
                        ));
                    }
                }
            }
        }
        _ => {}
    }
    let valid = warnings.is_empty();
    (valid, warnings)
}

/// Normalized hash of tool arguments (sorted keys) for loop guard. Returns first 16 chars of blake3 hex.
pub(crate) fn loop_guard_args_hash(
    arguments: Option<&serde_json::Map<String, serde_json::Value>>,
) -> String {
    let map = match arguments {
        Some(m) if !m.is_empty() => m,
        _ => return String::new(),
    };
    let mut keys: Vec<_> = map.keys().collect();
    keys.sort();
    let sorted: serde_json::Map<String, serde_json::Value> = keys
        .into_iter()
        .filter_map(|k| map.get(k).map(|v| (k.clone(), v.clone())))
        .collect();
    let s = serde_json::to_string(&sorted).unwrap_or_default();
    blake3::hash(s.as_bytes())
        .to_hex()
        .chars()
        .take(16)
        .collect()
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
/// Params for validate_tool_params (pre-execution feedback: check path/args without executing).
pub struct ValidateToolParamsParams {
    /// Name of the tool to validate (e.g. refresh_file_index, security_audit).
    pub name: String,
    /// Arguments object as would be passed to the tool (for path-taking tools, paths are checked against ALLOWED_ROOTS).
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

/// Default max characters for RAG response (token-budget proxy). Set RAG_MAX_RESPONSE_CHARS=0 to disable.
const DEFAULT_RAG_MAX_RESPONSE_CHARS: usize = 32_000;

/// Single suffix for all truncation (RAG, fetch, shell, skills). Used by truncate_for_budget, truncate_rag_response, web, skills.
pub(crate) const TRUNCATION_SUFFIX: &str = "[TRUNCATED TO SAVE TOKENS]";

/// Read RAG_MAX_RESPONSE_CHARS from env (default DEFAULT_RAG_MAX_RESPONSE_CHARS). 0 = no truncation.
pub(crate) fn read_rag_max_response_chars() -> usize {
    std::env::var("RAG_MAX_RESPONSE_CHARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RAG_MAX_RESPONSE_CHARS)
}

/// Read RAG_MAX_RESPONSE_TOKENS from env. When set and > 0, token-based truncation is used when tokenizer is available.
fn read_rag_max_response_tokens() -> Option<usize> {
    std::env::var("RAG_MAX_RESPONSE_TOKENS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&n| n > 0)
}

/// Truncate string to max_chars (character count); append TRUNCATION_SUFFIX when truncated. 0 = no limit.
pub(crate) fn truncate_for_budget(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return s.to_string();
    }
    let nchars = s.chars().count();
    if nchars <= max_chars {
        return s.to_string();
    }
    let take = max_chars.saturating_sub(TRUNCATION_SUFFIX.chars().count());
    format!(
        "{}{}",
        s.chars().take(take).collect::<String>(),
        TRUNCATION_SUFFIX
    )
}

/// Truncate RAG context by token count when RAG_MAX_RESPONSE_TOKENS is set and embedder has a tokenizer; else by RAG_MAX_RESPONSE_CHARS. Appends TRUNCATION_SUFFIX after the threshold.
pub(crate) fn truncate_rag_response(store: &RagStore, text: &str) -> String {
    let max_tokens = match read_rag_max_response_tokens() {
        Some(n) => n,
        None => {
            return truncate_for_budget(text, read_rag_max_response_chars());
        }
    };
    let text_tokens = match store.embedder.count_tokens(text) {
        Some(n) => n,
        None => {
            return truncate_for_budget(text, read_rag_max_response_chars());
        }
    };
    let suffix_tokens = store.embedder.count_tokens(TRUNCATION_SUFFIX).unwrap_or(8);
    let max_content_tokens = max_tokens.saturating_sub(suffix_tokens);
    if text_tokens <= max_content_tokens {
        return text.to_string();
    }
    let nchars = text.chars().count();
    let mut lo: usize = 0;
    let mut hi = nchars;
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let prefix: String = text.chars().take(mid).collect();
        let ok = store
            .embedder
            .count_tokens(&prefix)
            .is_none_or(|c| c <= max_content_tokens);
        if ok {
            lo = mid;
        } else {
            hi = mid.saturating_sub(1);
        }
    }
    let truncated: String = text.chars().take(lo).collect();
    format!("{}{}", truncated, TRUNCATION_SUFFIX)
}

/// Build _meta JSON line for RAG-style responses (chunks_returned/sections_returned + tokens_estimated, optional cost_avoided_usd).
pub(crate) fn format_rag_meta(key: &str, key_val: usize, tokens_estimated: usize) -> String {
    let mut obj = format!(
        r#" "{}": {}, "tokens_estimated": {}"#,
        key, key_val, tokens_estimated
    );
    if let Some(cost_per_1m) = read_rag_cost_per_1m_tokens_usd() {
        let cost_avoided = (tokens_estimated as f64 / 1_000_000.0) * cost_per_1m;
        obj.push_str(&format!(r#", "cost_avoided_usd": {:.6}"#, cost_avoided));
    }
    format!("\n\n_meta: {{{}}}", obj)
}

/// Read RAG_COST_PER_1M_INPUT_TOKENS_USD from env. When set, _meta includes cost_avoided_usd.
fn read_rag_cost_per_1m_tokens_usd() -> Option<f64> {
    std::env::var("RAG_COST_PER_1M_INPUT_TOKENS_USD")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&x| x >= 0.0)
}

/// Redact path/username and API-key patterns in shell output so logs and training data
/// do not leak local paths, usernames, or secrets (e.g. sk-..., AIza...).
pub fn sanitize_shell_output(s: &str) -> String {
    static WINDOWS_USER: OnceLock<Regex> = OnceLock::new();
    static UNIX_HOME: OnceLock<Regex> = OnceLock::new();
    static OPENAI_KEY: OnceLock<Regex> = OnceLock::new();
    static GOOGLE_API_KEY: OnceLock<Regex> = OnceLock::new();
    static AWS_ACCESS_KEY: OnceLock<Regex> = OnceLock::new();
    static ANTHROPIC_KEY: OnceLock<Regex> = OnceLock::new();
    static GITHUB_TOKEN: OnceLock<Regex> = OnceLock::new();
    static PEM_BLOCK: OnceLock<Regex> = OnceLock::new();
    /// Redact path segments that look like secret filenames (e.g. secret_key.txt) so exports do not leak them.
    static SECRET_FILENAME: OnceLock<Regex> = OnceLock::new();
    // Match any drive letter (A-Z), not just C:, so users on D:, E:, etc. are also redacted (M-3).
    let win = WINDOWS_USER
        .get_or_init(|| Regex::new(r"(?i)[A-Z]:\\Users\\[^\\]+").expect("WINDOWS_USER regex"));
    let unix = UNIX_HOME.get_or_init(|| Regex::new(r"/home/[^/\s]+").expect("UNIX_HOME regex"));
    let openai =
        OPENAI_KEY.get_or_init(|| Regex::new(r"sk-[a-zA-Z0-9]{20,}").expect("OPENAI_KEY regex"));
    let google = GOOGLE_API_KEY
        .get_or_init(|| Regex::new(r"AIza[0-9A-Za-z\-_]{35}").expect("GOOGLE_API_KEY regex"));
    let aws = AWS_ACCESS_KEY
        .get_or_init(|| Regex::new(r"AKIA[0-9A-Z]{16}").expect("AWS_ACCESS_KEY regex"));
    let anthropic = ANTHROPIC_KEY
        .get_or_init(|| Regex::new(r"sk-ant-[a-zA-Z0-9\-_]{20,}").expect("ANTHROPIC_KEY regex"));
    let github = GITHUB_TOKEN.get_or_init(|| {
        Regex::new(r"(ghp_|gho_|github_pat_)[a-zA-Z0-9_]{10,}").expect("GITHUB_TOKEN regex")
    });
    let pem =
        PEM_BLOCK.get_or_init(|| Regex::new(r"-----BEGIN [A-Z ]+-----").expect("PEM_BLOCK regex"));
    let secret_fn = SECRET_FILENAME.get_or_init(|| {
        Regex::new(
            r"(?i)(secret|password|credential|token|api.?key).*\.(txt|json|env|yaml|yml|toml)",
        )
        .expect("SECRET_FILENAME regex")
    });
    let t = win.replace_all(s, "[REDACTED]");
    let t = unix.replace_all(&t, "[REDACTED]");
    let t = openai.replace_all(&t, "[REDACTED]");
    let t = google.replace_all(&t, "[REDACTED]");
    let t = aws.replace_all(&t, "[REDACTED]");
    let t = anthropic.replace_all(&t, "[REDACTED]");
    let t = github.replace_all(&t, "[REDACTED]");
    let t = pem.replace_all(&t, "[REDACTED]");
    secret_fn.replace_all(&t, "[REDACTED]").into_owned()
}

/// Handler generic over ingestion and storage providers for testability.
#[derive(Clone)]
pub struct AgenticHandler<I, S> {
    pub store: Arc<RagStore>,
    pub dataset_collector: Option<Arc<std::sync::Mutex<DatasetCollector>>>,
    /// When set, commit_to_memory can spawn background ingest; manifest path for RAG re-index.
    pub ingest_manifest_path: Option<PathBuf>,
    /// When set, get_tool_selection_guide reads from this path; else first allowed root/docs/TOOL_SELECTION_GUIDE.md.
    pub tool_selection_guide_path: Option<PathBuf>,
    /// When set, get_design_tokens reads from this dir; else first allowed root/docs/design_tokens.
    pub design_tokens_dir: Option<PathBuf>,
    /// When set, compile_rules reads global rules from this dir. From RULES_VAULT or GLOBAL_RULES_DIR.
    pub global_rules_dir: Option<PathBuf>,
    /// When set, propose_vault_rule can append to vault 00_Meta/Rules. From VAULT_DIR or HOLLOW_VAULT.
    pub vault_dir: Option<PathBuf>,
    pub ingestion: I,
    pub storage: S,
    pub(crate) tool_router: ToolRouter<AgenticHandler<I, S>>,
}

/// Builds the tool router for the default handler type. Used in [`AgenticHandler::new_with_collector`].
fn build_tool_router() -> ToolRouter<AgenticHandler<DefaultIngestion, DefaultStorage>> {
    AgenticHandler::<DefaultIngestion, DefaultStorage>::tool_router()
}

impl AgenticHandler<DefaultIngestion, DefaultStorage> {
    /// Construct with default filesystem ingestion and DB storage.
    pub fn new(store: Arc<RagStore>) -> Self {
        Self::new_with_collector(store, None, None, None, None, None, None)
    }
    /// Construct with optional dataset collector, manifest path, tool selection guide path, design tokens dir, global rules dir, and vault dir.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_collector(
        store: Arc<RagStore>,
        dataset_collector: Option<Arc<std::sync::Mutex<DatasetCollector>>>,
        ingest_manifest_path: Option<PathBuf>,
        tool_selection_guide_path: Option<PathBuf>,
        design_tokens_dir: Option<PathBuf>,
        global_rules_dir: Option<PathBuf>,
        vault_dir: Option<PathBuf>,
    ) -> Self {
        let ingestion = DefaultIngestion;
        let storage = DefaultStorage::new(store.db.clone(), store.embedder.clone());
        Self {
            store: store.clone(),
            dataset_collector,
            ingest_manifest_path,
            tool_selection_guide_path,
            design_tokens_dir,
            global_rules_dir,
            vault_dir,
            ingestion,
            storage,
            tool_router: build_tool_router(),
        }
    }
}

#[tool_router]
impl<I, S> AgenticHandler<I, S>
where
    I: IngestionProvider + Send + Sync + Clone + 'static,
    S: VectorStoreProvider + Send + Sync + Clone + 'static,
{
    /// Build handler with custom ingestion and storage (e.g. for tests).
    #[allow(clippy::too_many_arguments)]
    pub fn with_providers(
        store: Arc<RagStore>,
        ingestion: I,
        storage: S,
        dataset_collector: Option<Arc<std::sync::Mutex<DatasetCollector>>>,
        ingest_manifest_path: Option<PathBuf>,
        tool_selection_guide_path: Option<PathBuf>,
        design_tokens_dir: Option<PathBuf>,
        global_rules_dir: Option<PathBuf>,
        vault_dir: Option<PathBuf>,
        tool_router: ToolRouter<AgenticHandler<I, S>>,
    ) -> Self {
        Self {
            store,
            dataset_collector,
            ingest_manifest_path,
            tool_selection_guide_path,
            design_tokens_dir,
            global_rules_dir,
            vault_dir,
            ingestion,
            storage,
            tool_router,
        }
    }

    /// Orchestration: read content via ingestion, chunk, optionally embed via store, save via storage. Used for testing with mocks.
    pub fn process_ingestion(&self, path: &Path) -> AnyhowResult<u32> {
        let content = self.ingestion.read_content(path)?;
        let content = content.trim();
        if content.is_empty() {
            return Ok(0);
        }
        let source = path.to_string_lossy().to_string();
        let chunks = chunk_file(content, &source);
        if chunks.is_empty() {
            return Ok(0);
        }
        let embeddings = if self.store.embedder.is_available() {
            let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
            self.store.embedder.embed_batch(&texts).ok()
        } else {
            None
        };
        self.storage.delete_by_source(&source)?;
        self.storage
            .save_chunks(&source, &chunks, embeddings.as_deref())?;
        Ok(chunks.len() as u32)
    }

    #[tool(
        name = "query_knowledge",
        description = "Search the codebase: hierarchical (summaries then chunks), graph walk, reranking. execute=true: RAG context for IDE to synthesize. reasoning=true: fewer chunks, you answer. outline_only=true: return chunk id, source, name per hit (no full text); use for exploratory browse then get_section(id) or full query for content. section_first=true: return outline + instruction to call get_section(id) for document-heavy queries. If 'No relevant information found', do not invent—ask user or refine query."
    )]
    async fn query_knowledge(
        &self,
        params: Parameters<QueryKnowledgeParams>,
    ) -> Result<CallToolResult, McpError> {
        rag::query_knowledge_core(self, params.0).await
    }

    #[tool(
        name = "query_master_research",
        description = "Search only the synthesized master research document for technical details, architectural patterns, or domain logic. Run synthesis (e.g. NotebookLM or manual merge) and rag-mcp ingest-from-jsonl before relying on this tool; if the master document is not indexed, results will be empty."
    )]
    async fn query_master_research(
        &self,
        params: Parameters<QueryMasterResearchParams>,
    ) -> Result<CallToolResult, McpError> {
        rag::query_master_research_impl(self, params.0).await
    }

    #[tool(
        name = "get_related_code",
        description = "Get the code chunk(s) that define the given symbol and chunks that import it. Use for exact navigation by symbol name (e.g. PaymentService) without semantic search. Pass max_references to cap ref chunks (saves tokens); default from GET_RELATED_CODE_MAX_REFERENCES or 25."
    )]
    async fn get_related_code(
        &self,
        params: Parameters<GetRelatedCodeParams>,
    ) -> Result<CallToolResult, McpError> {
        rag::get_related_code_impl(self, params.0).await
    }

    #[tool(
        name = "resolve_symbol",
        description = "Return the exact code block that defines the given symbol (jump-to-definition). Use when you need the implementation of a specific function or class, not references."
    )]
    async fn resolve_symbol(
        &self,
        params: Parameters<ResolveSymbolParams>,
    ) -> Result<CallToolResult, McpError> {
        rag::resolve_symbol_impl(self, params.0).await
    }

    #[tool(
        name = "get_doc_outline",
        description = "List section ids and titles for a document (token-efficient). Use before get_section to browse structure then fetch only needed sections. Only documents chunked by headings (e.g. .md with #) have sections; others return empty."
    )]
    async fn get_doc_outline(
        &self,
        params: Parameters<GetDocOutlineParams>,
    ) -> Result<CallToolResult, McpError> {
        rag::get_doc_outline_impl(self, params.0).await
    }

    #[tool(
        name = "get_section",
        description = "Fetch full content of one section by its chunk id (from get_doc_outline). Token-efficient: returns only that section."
    )]
    async fn get_section(
        &self,
        params: Parameters<GetSectionParams>,
    ) -> Result<CallToolResult, McpError> {
        rag::get_section_impl(self, params.0).await
    }

    #[tool(
        name = "security_audit",
        description = "Run Semgrep security scan on a path. Deterministic findings; use when the LLM must know if code is safe. Requires Semgrep CLI in PATH."
    )]
    async fn security_audit(
        &self,
        params: Parameters<SecurityAuditParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::security_audit_impl(self, params.0).await
    }

    #[tool(
        name = "module_graph",
        description = "Rust module structure: text tree (cargo-modules) or Mermaid diagram. Params: workspace_path (optional), format: 'text' | 'mermaid'. Use for refactors and topology."
    )]
    async fn module_graph(
        &self,
        params: Parameters<ModuleGraphParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::module_graph_impl(self, params.0).await
    }

    #[tool(
        name = "get_system_status",
        description = "Resource awareness: CPU load, RAM (total/used), and optionally GPU VRAM via nvidia-smi. Call before heavy or complex commands to avoid overload; VRAM > 90% triggers a critical warning."
    )]
    async fn get_system_status(
        &self,
        params: Parameters<GetSystemStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        shell::get_system_status_impl(self, params).await
    }

    /// Single command only (no shell). Only cargo, git, grep, ls, npm are allowed (allowlist); runs in project root.
    #[tool(
        name = "execute_shell_command",
        description = "Execute terminal commands in project root. Only cargo, git, grep, ls, npm are allowed (security allowlist). Path args are resolved against the project root and must stay under ALLOWED_ROOTS. Destructive git commands are blocked unless bypass_hitl is true and MCP_SHELL_BYPASS_HITL=1. Redirects (>, >>) into the hub are rejected (vault boundary)."
    )]
    async fn execute_shell_command(
        &self,
        params: Parameters<ExecuteShellCommandParams>,
    ) -> Result<CallToolResult, McpError> {
        shell::execute_shell_command_impl(self, params).await
    }

    #[tool(
        name = "commit_to_memory",
        description = "Append a timestamped lesson to docs/lessons_learned.md and trigger background RAG re-index. Use after fixing a bug or making an architectural decision. Writes to the first allowed root's docs/ (project root when using a single ALLOWED_ROOTS entry)."
    )]
    async fn commit_to_memory(
        &self,
        params: Parameters<CommitToMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        if mcp_read_only() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Read-only mode: write disabled (MCP_READ_ONLY).",
            )]));
        }
        memory::commit_to_memory_impl(self, params.0).await
    }

    #[tool(
        name = "log_training_row",
        description = "Append one row to training.jsonl for Ouroboros. Use the exact task line as query (e.g. [DOCS] Document X). Server skips low-value query/response."
    )]
    async fn log_training_row(
        &self,
        params: Parameters<LogTrainingRowParams>,
    ) -> Result<CallToolResult, McpError> {
        if mcp_read_only() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Read-only mode: write disabled (MCP_READ_ONLY).",
            )]));
        }
        memory::log_training_row_impl(self, params.0).await
    }

    #[tool(
        name = "approve_pattern",
        description = "Golden Vibe Flywheel: Approve a successful code pattern. Saves the pattern to docs/golden_set.md and logs it as a high-quality example to the training dataset. Use this when you've hit upon a great solution you want the model to remember."
    )]
    async fn approve_pattern(
        &self,
        params: Parameters<ApprovePatternParams>,
    ) -> Result<CallToolResult, McpError> {
        if mcp_read_only() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Read-only mode: write disabled (MCP_READ_ONLY).",
            )]));
        }
        memory::approve_pattern_impl(self, params.0).await
    }

    #[tool(
        name = "auto_approve_pattern",
        description = "Propose a code pattern for golden set. Compares to approved patterns; if similar or reasonable first (100-4000 chars), adds to golden_set.md and training. Call after verify_integrity passes."
    )]
    async fn auto_approve_pattern(
        &self,
        params: Parameters<ApprovePatternParams>,
    ) -> Result<CallToolResult, McpError> {
        if mcp_read_only() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Read-only mode: write disabled (MCP_READ_ONLY).",
            )]));
        }
        memory::auto_approve_pattern_impl(self, params.0).await
    }

    #[tool(
        name = "refresh_file_index",
        description = "Re-ingest the given file paths into the RAG index (parse, chunk, embed). Call after editing files so the agent's memory is up to date. Paths must be under ALLOWED_ROOTS."
    )]
    async fn refresh_file_index(
        &self,
        params: Parameters<RefreshFileIndexParams>,
    ) -> Result<CallToolResult, McpError> {
        if mcp_read_only() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Read-only mode: write disabled (MCP_READ_ONLY).",
            )]));
        }
        memory::refresh_file_index_impl(self, params.0).await
    }

    #[tool(
        name = "save_rule_to_memory",
        description = "Append a rule or guideline to docs/agent_rules.md and re-index it so query_knowledge (RECALL) can retrieve it. Use to persist high-signal rules the agent should follow. Same docs root as commit_to_memory (first allowed root)."
    )]
    async fn save_rule_to_memory(
        &self,
        params: Parameters<SaveRuleToMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        if mcp_read_only() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Read-only mode: write disabled (MCP_READ_ONLY).",
            )]));
        }
        memory::save_rule_to_memory_impl(self, params.0).await
    }

    #[tool(
        name = "propose_vault_rule",
        description = "Append a rule to the Vault 00_Meta/Rules (e.g. agent_rules.md). Requires VAULT_DIR or HOLLOW_VAULT and ALLOW_VAULT_RULE_PROPOSAL=true. Use to evolve global memory from the IDE; optionally run Synapse after to refresh .cursorrules."
    )]
    async fn propose_vault_rule(
        &self,
        params: Parameters<SaveRuleToMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        if mcp_read_only() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Read-only mode: write disabled (MCP_READ_ONLY).",
            )]));
        }
        memory::propose_vault_rule_impl(self, params.0).await
    }

    #[tool(
        name = "verify_integrity",
        description = "Gatekeeper: run syntax (cargo check), tests (cargo test), and linter (cargo clippy) in one go. Returns clean JSON pass/fail so the agent can decide without parsing raw output. Use after code changes instead of manually running cargo check and cargo test."
    )]
    async fn verify_integrity(
        &self,
        params: Parameters<VerifyIntegrityParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::verify_integrity_impl(self, params.0).await
    }

    #[tool(
        name = "verify_module_tree",
        description = "Verify that all Rust modules under src/ are reachable from lib.rs or main.rs. Call after creating a new file under src/ to ensure it is wired (e.g. mod auth; in the parent). Returns phantom modules if any are not reachable."
    )]
    async fn verify_module_tree(
        &self,
        params: Parameters<VerifyModuleTreeParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::verify_module_tree_impl(self, params.0).await
    }

    #[tool(
        name = "read_manifest",
        description = "Read Cargo.toml and return dependency crate names and versions (dependencies + dev-dependencies). Use before suggesting code that uses an external crate to avoid version drift; prefer compiler errors and current manifest over RAG/memory."
    )]
    async fn read_manifest(
        &self,
        params: Parameters<ReadManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::read_manifest_impl(self, params.0).await
    }

    #[tool(
        name = "project_packer",
        description = "Generate a compressed tree view of the project and read key configs (Cargo.toml, package.json). Gives the model a mental map of the codebase without wasting tokens on ls -R."
    )]
    async fn project_packer(
        &self,
        params: Parameters<ProjectPackerParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::project_packer_impl(self, params.0).await
    }

    #[tool(
        name = "scan_secrets",
        description = "Scan the workspace for likely hardcoded secrets (key/token/secret/password assignments to long strings, sk_live_/sk_test_, AIza). Returns JSON list of path, line, snippet. Run before commit_to_memory to satisfy Tribunal; use std::env::var() for secrets instead of hardcoding."
    )]
    async fn scan_secrets(
        &self,
        params: Parameters<ScanSecretsParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::scan_secrets_impl(self, params.0).await
    }

    #[tool(
        name = "get_file_history",
        description = "Return per-line git blame or recent log for a file so you can see why a line exists before deleting or refactoring (Chesterton's fence). Path must be under ALLOWED_ROOTS."
    )]
    async fn get_file_history(
        &self,
        params: Parameters<GetFileHistoryParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::get_file_history_impl(self, params.0).await
    }

    #[tool(
        name = "route_task",
        description = "Multi-model routing: determine which AI model is optimal for a given control loop phase (planning, execution, review, audit, recovery). Returns model selection with confidence and fallback. Override per-phase via MCP_ROUTE_PLANNING=gemini etc."
    )]
    async fn route_task(
        &self,
        params: Parameters<RouteTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        model_routing::route_task_impl(self, params.0).await
    }

    #[tool(
        name = "get_loop_state",
        description = "Query the current autonomous control loop state: iteration count, status (running/stagnated/converged), progress score, and full history."
    )]
    async fn get_loop_state(
        &self,
        params: Parameters<control_loop::GetLoopStateParams>,
    ) -> Result<CallToolResult, McpError> {
        // ManagedLoop intercepts when wrapped; bare handler returns default-state summary.
        control_loop::get_loop_state_handler(self, params.0).await
    }

    #[tool(
        name = "get_metrics",
        description = "Return operational metrics: latency histograms, cache hit/miss rates, tool call counts."
    )]
    async fn get_metrics(&self) -> Result<CallToolResult, McpError> {
        let tool_snap = crate::metrics::TOOL_LATENCY.snapshot();
        let total = crate::metrics::TOOL_CALLS_TOTAL.get();
        let errors = crate::metrics::TOOL_ERRORS.get();
        let cache_hits = crate::metrics::CACHE_HITS.get();
        let cache_misses = crate::metrics::CACHE_MISSES.get();
        let rerank_hits = crate::rerank::RERANK_HITS.load(std::sync::atomic::Ordering::Relaxed);
        let rerank_misses = crate::rerank::RERANK_MISSES.load(std::sync::atomic::Ordering::Relaxed);

        let cache_total = cache_hits + cache_misses;
        let cache_rate = if cache_total > 0 {
            cache_hits as f64 / cache_total as f64 * 100.0
        } else {
            0.0
        };
        let rerank_total = rerank_hits + rerank_misses;
        let rerank_rate = if rerank_total > 0 {
            rerank_hits as f64 / rerank_total as f64 * 100.0
        } else {
            0.0
        };
        let error_rate = if total > 0 {
            errors as f64 / total as f64 * 100.0
        } else {
            0.0
        };

        let report = format!(
            "## MCP Server Metrics\n\n\
             **Tool Calls:** {} total, {} errors ({:.1}%)\n\
             **Latency:** {}\n\
             **Semantic Cache:** {} hits, {} misses ({:.1}% hit rate)\n\
             **Reranker:** {} hits, {} misses ({:.1}% hit rate)",
            total,
            errors,
            error_rate,
            tool_snap,
            cache_hits,
            cache_misses,
            cache_rate,
            rerank_hits,
            rerank_misses,
            rerank_rate,
        );
        Ok(CallToolResult::success(vec![Content::text(report)]))
    }

    #[tool(
        name = "plan_task",
        description = "Task decomposition: break a complex user objective into a structured Markdown plan with verifiable substeps, success criteria, and constraints."
    )]
    async fn plan_task(
        &self,
        params: Parameters<planning::PlanTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        planning::plan_task_impl(self, params.0).await
    }

    #[tool(
        name = "git_checkpoint",
        description = "Create or inspect Git checkpoints. 'save' commits staged changes; 'revert' runs git reset --hard HEAD~1 only if GIT_CHECKPOINT_REVERT_ALLOW=1 on the server; 'status' lists recent checkpoint commits."
    )]
    async fn git_checkpoint(
        &self,
        params: Parameters<shell::GitCheckpointParams>,
    ) -> Result<CallToolResult, McpError> {
        shell::git_checkpoint_impl(self, params.0).await
    }

    #[tool(
        name = "aggregate_audit",
        description = "Run a comprehensive audit (cargo check + test + clippy + scan_secrets + verify_module_tree) and return a structured JSON report with an overall 0.0-1.0 confidence score."
    )]
    async fn aggregate_audit(
        &self,
        params: Parameters<analysis::AggregateAuditParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::aggregate_audit_impl(self, params.0).await
    }

    #[tool(
        name = "skeptic_review",
        description = "Adversarial code review: produce an adversarial critique of a code diff using RAG-backed patterns and lessons learned. Returns JSON with requires_retry: true if slop or errors are found."
    )]
    async fn skeptic_review(
        &self,
        params: Parameters<analysis::SkepticReviewParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::skeptic_review_impl(self, params.0).await
    }

    #[tool(
        name = "submit_task",
        description = "Enqueue an async task to _tasks/inbox. Task types: research, research_ingest, ingest, refresh_file_index, verify-integrity. Payload is type-specific (e.g. query, path, paths, workspace_path, jsonl_path). The task runner processes inbox and writes results to _tasks/outbox."
    )]
    async fn submit_task(
        &self,
        params: Parameters<SubmitTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        if mcp_read_only() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Read-only mode: write disabled (MCP_READ_ONLY).",
            )]));
        }
        ui::submit_task_impl(self, params.0).await
    }

    #[tool(
        name = "get_relevant_tools",
        description = "Tool-RAG: return tool names most relevant to a natural-language query (semantic similarity over name+description). Use to limit context: call with the user message, then inject only these tools into the prompt. top_k default 5 (or GET_RELEVANT_TOOLS_TOP_K). Pass include_descriptions: true for gateway mode (returns [{ name, description }] per tool). When embedder unavailable, returns all tools."
    )]
    async fn get_relevant_tools(
        &self,
        params: Parameters<GetRelevantToolsParams>,
    ) -> Result<CallToolResult, McpError> {
        rag::get_relevant_tools_impl(self, params.0).await
    }

    #[tool(
        name = "invoke_tool",
        description = "Gateway mode: run a tool by name with the given arguments. Call after get_relevant_tools; uses same context so only 2–3 tools need to be in context instead of 40+."
    )]
    async fn invoke_tool(
        &self,
        params: Parameters<InvokeToolParams>,
    ) -> Result<CallToolResult, McpError> {
        // Redirect is handled in call_tool; this path is unreachable when client calls invoke_tool.
        let _ = params;
        Err(McpError::internal_error(
            "invoke_tool is handled by gateway redirect; this should not be reached.",
            None,
        ))
    }

    #[tool(
        name = "validate_tool_params",
        description = "Pre-execution feedback: validate tool name and arguments (e.g. paths under ALLOWED_ROOTS) without executing. Returns { \"valid\": bool, \"warnings\": string[] }. Use to let the agent correct path/args before calling the real tool."
    )]
    async fn validate_tool_params(
        &self,
        params: Parameters<ValidateToolParamsParams>,
    ) -> Result<CallToolResult, McpError> {
        let (valid, warnings) =
            validate_tool_params_impl(self, &params.0.name, params.0.arguments.as_ref());
        let out = serde_json::json!({ "valid": valid, "warnings": warnings });
        Ok(CallToolResult::success(vec![Content::text(
            out.to_string(),
        )]))
    }

    #[tool(
        name = "fetch_web_markdown",
        description = "Fetches a URL and returns a sanitized Markdown version of the page content. No JavaScript is executed. Use this for documentation research."
    )]
    async fn fetch_web_markdown(
        &self,
        params: Parameters<FetchWebMarkdownParams>,
    ) -> Result<CallToolResult, McpError> {
        web::fetch_web_markdown_impl(self, params.0).await
    }

    #[tool(
        name = "search_web",
        description = "Server-side web search. Returns URLs (and titles) for the given topic. Requires TAVILY_API_KEY or SERPER_API_KEY. Use with research_and_verify(topic, urls) to compare and ingest. Params: topic (required), limit (optional, default 5)."
    )]
    async fn search_web(
        &self,
        params: Parameters<SearchWebParams>,
    ) -> Result<CallToolResult, McpError> {
        web::search_web_impl(self, params.0).await
    }

    #[tool(
        name = "ingest_web_context",
        description = "Persist web content into RAG. Pass snippets (url + content); they are stored with last_verified_date so query_knowledge results show how old the knowledge is. No API key required."
    )]
    async fn ingest_web_context(
        &self,
        params: Parameters<IngestWebContextParams>,
    ) -> Result<CallToolResult, McpError> {
        web::ingest_web_context_impl(self, params.0).await
    }

    #[tool(
        name = "research_and_verify",
        description = "Lookup RAG for a topic, then fetch the given URLs and ingest into RAG. Use after web search: pass topic and 1–3 high-signal URLs. No API key required."
    )]
    async fn research_and_verify(
        &self,
        params: Parameters<ResearchAndVerifyParams>,
    ) -> Result<CallToolResult, McpError> {
        web::research_and_verify_impl(self, params.0).await
    }

    #[tool(
        name = "get_ui_blueprint",
        description = "Return a pre-vetted Tailwind/React layout snippet per DESIGN_AXIOMS (3-column horizontal on desktop, responsive collapse). Use when generating dashboard or multi-column UI. blueprint_type: dashboard, form, or settings."
    )]
    async fn get_ui_blueprint(
        &self,
        params: Parameters<GetUiBlueprintParams>,
    ) -> Result<CallToolResult, McpError> {
        ui::get_ui_blueprint_impl(self, params.0).await
    }

    #[tool(
        name = "verify_ui_integrity",
        description = "Linter for design: check UI snippet against DESIGN_AXIOMS. Flags stacking red flags (e.g. w-full without flex/grid), forbidden shadows (shadow-md+), and suggests refactor. All UI code must pass before completion."
    )]
    async fn verify_ui_integrity(
        &self,
        params: Parameters<VerifyUiIntegrityParams>,
    ) -> Result<CallToolResult, McpError> {
        ui::verify_ui_integrity_impl_tool(self, params.0).await
    }

    #[tool(
        name = "analyze_error_log",
        description = "Analyze error output using RAG context and lessons_learned; suggest root cause and concrete fix. Pass error_output; optionally recent_errors for recurring errors (same as prompt)."
    )]
    async fn analyze_error_log(
        &self,
        params: Parameters<AnalyzeErrorLogParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::analyze_error_log_impl(self, params.0).await
    }

    #[tool(
        name = "scaffold_reproduction_test",
        description = "TDD: Before fixing a logic bug, write a test that fails. Returns context and instructions to scaffold that reproduction test. Pass bug_description; optionally error_output (same as prompt)."
    )]
    async fn scaffold_reproduction_test(
        &self,
        params: Parameters<ScaffoldReproductionTestParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::scaffold_reproduction_test_impl(self, params.0).await
    }

    #[tool(
        name = "review_diff",
        description = "Audit a code diff with the local LLM: security, unwrap(), and bad practices. Use before committing. Pass diff (unified diff or changed code). Pass mode: 'short' for security-only audit. Same behavior as prompt."
    )]
    async fn review_diff(
        &self,
        params: Parameters<ReviewDiffParams>,
    ) -> Result<CallToolResult, McpError> {
        analysis::review_diff_impl(self, params.0).await
    }

    #[tool(
        name = "get_tool_selection_guide",
        description = "Returns the static situation→tool table. Pass outline_only: true for outline only; section: <id> for one section (dont, core_vs_niche, situation_tool). Call when unsure which tool fits; use get_relevant_tools for query-driven selection."
    )]
    async fn get_tool_selection_guide(
        &self,
        params: Parameters<GetToolSelectionGuideParams>,
    ) -> Result<CallToolResult, McpError> {
        ui::get_tool_selection_guide_impl(self, params.0).await
    }

    #[tool(
        name = "get_design_tokens",
        description = "Read design tokens (colors, typography, component specs) from DESIGN_TOKENS_DIR or docs/design/data. Pass token_category (e.g. colors, typography). Tries {category}.csv/.json and title-case (e.g. Colors.csv). Complements get_ui_blueprint and verify_ui_integrity."
    )]
    async fn get_design_tokens(
        &self,
        params: Parameters<GetDesignTokensParams>,
    ) -> Result<CallToolResult, McpError> {
        ui::get_design_tokens_impl(self, params.0).await
    }

    #[tool(
        name = "fork_terminal",
        description = "Run a command in a new terminal/process (same allowlist as execute_shell_command: cargo, git, grep, ls, npm). Use for long-running or interactive commands (e.g. npm run dev, cargo run) so the user sees live output and MCP does not block."
    )]
    async fn fork_terminal(
        &self,
        params: Parameters<ForkTerminalParams>,
    ) -> Result<CallToolResult, McpError> {
        ui::fork_terminal_impl(self, params.0).await
    }

    #[tool(
        name = "compile_rules",
        description = "Merge global rules (from RULES_VAULT or GLOBAL_RULES_DIR: Standards, Rules, Workflows) and project .context/ into .cursorrules and optionally GEMINI.md, CLAUDE.md. Requires RULES_VAULT or GLOBAL_RULES_DIR to be set. Pass active_project_path (target project root)."
    )]
    async fn compile_rules(
        &self,
        params: Parameters<CompileRulesParams>,
    ) -> Result<CallToolResult, McpError> {
        ui::compile_rules_impl(self, params.0).await
    }

    #[tool(
        name = "list_skill_metadata",
        description = "List skills (id, name, description ~100 words). Use for procedure, pipeline, MCP quality, or security checklist; then get_skill_content(skill_id) for full SKILL.md. No full content returned here. Requires RULES_VAULT or GLOBAL_RULES_DIR with 02_Skills/ or skills/."
    )]
    async fn list_skill_metadata(
        &self,
        params: Parameters<ListSkillMetadataParams>,
    ) -> Result<CallToolResult, McpError> {
        let _ = params;
        skills::list_skill_metadata_impl(self).await
    }

    #[tool(
        name = "get_skill_content",
        description = "Fetch full content of a skill by skill_id (from list_skill_metadata). Use after list_skill_metadata when you need the full procedure, pipeline, or quality checklist. Truncated at 5k words if longer."
    )]
    async fn get_skill_content(
        &self,
        params: Parameters<GetSkillContentParams>,
    ) -> Result<CallToolResult, McpError> {
        skills::get_skill_content_impl(self, params.0.skill_id).await
    }

    #[tool(
        name = "get_skill_reference",
        description = "Fetch a file under a skill's references/ (on-demand). Pass skill_id and path (e.g. philosophy.md). Truncated at 5k words if longer."
    )]
    async fn get_skill_reference(
        &self,
        params: Parameters<GetSkillReferenceParams>,
    ) -> Result<CallToolResult, McpError> {
        skills::get_skill_reference_impl(self, params.0.skill_id, params.0.path).await
    }

    #[tool(
        name = "validate_skill",
        description = "Validate a skill file: frontmatter (name/title, description), optional required headings (## Purpose, ## When to use), max size. Returns { valid, errors, warnings }."
    )]
    async fn validate_skill(
        &self,
        params: Parameters<ValidateSkillParams>,
    ) -> Result<CallToolResult, McpError> {
        skills::validate_skill_impl(self, params.0.skill_id).await
    }

    #[tool(
        name = "get_codebase_outline",
        description = "Compressed structural outline of the codebase (file path + symbol names from AST). Use before deep search for a token-efficient mental map. Params: workspace_path (optional), max_items (default 2000)."
    )]
    async fn get_codebase_outline(
        &self,
        params: Parameters<GetCodebaseOutlineParams>,
    ) -> Result<CallToolResult, McpError> {
        rag::get_codebase_outline_impl(self, params.0).await
    }
}

/// Per-tool timeout in seconds. Known slow tools get generous limits;
/// everything else defaults to `MCP_TOOL_TIMEOUT_SECS` env var or 120s.
pub(crate) fn tool_timeout_secs(name: &str) -> u64 {
    match name {
        "ingest_directory" | "research_and_verify" => 600,
        "fetch_web_markdown" | "ingest_web_context" => 300,
        _ => std::env::var("MCP_TOOL_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120),
    }
}

// Manual implementation of ServerHandler to intercept calls (default handler type).
impl rmcp::ServerHandler for AgenticHandler<DefaultIngestion, DefaultStorage> {
    /// get_info.
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "RECALL first: query_knowledge every turn (2-5 keywords) before anything else. \
Pipeline: RECALL → RESEARCH → work → refresh_file_index (files changed) → scan_secrets → LOG (log_training_row) → EVOLUTION (commit_to_memory) → verify_integrity → auto_approve_pattern. \
Read STATE.md at session start. Use /rag:pause to save state, /rag:resume to restore. \
Full rules + pitfalls: docs/setup/AGENTIC_OPERATOR_RULE.md. No hardcoded secrets; use std::env::var(). \
Loop guard: same tool+args blocked after N calls (MCP_LOOP_GUARD_THRESHOLD, default 5); use different args or approach if blocked. \
For autonomous loops use STATE.md (or workflow_state.md) for phase/position; read at cycle start, update at cycle end. \
Default is minimal (5 tools) for token savings; set MCP_FULL_TOOLS=1 for full list. Use get_relevant_tools (optionally with include_descriptions: true) then invoke_tool for other tools."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            ..Default::default()
        }
    }

    /// Returns MCP tool list. Default: minimal set for token savings (gateway: get_relevant_tools + invoke_tool).
    /// Set MCP_FULL_TOOLS=1 in env to return all tools (for debugging only).
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let request_id = gen_request_id();
        let out = REQUEST_ID
            .scope(request_id.clone(), async {
                let mut tools = self.tool_router.list_all();
                let full_tools = std::env::var("MCP_FULL_TOOLS")
                    .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true"))
                    .unwrap_or(false);
                if !full_tools {
                    tools.retain(|t| MINIMAL_TOOL_NAMES.contains(&t.name.as_ref()));
                }
                let checksum = tool_list_checksum(&tools);
                let idempotent_list: Vec<serde_json::Value> = IDEMPOTENT_TOOL_NAMES
                    .iter()
                    .filter(|n| tools.iter().any(|t| t.name.as_ref() == **n))
                    .map(|s| serde_json::Value::String((*s).to_string()))
                    .collect();
                let mut meta_obj = serde_json::Map::new();
                meta_obj.insert(
                    "tool_list_checksum".to_string(),
                    serde_json::Value::String(checksum),
                );
                meta_obj.insert(
                    "idempotent_tools".to_string(),
                    serde_json::Value::Array(idempotent_list),
                );
                let meta = Some(Meta(meta_obj));
                Ok(ListToolsResult {
                    tools,
                    next_cursor: None,
                    meta,
                })
            })
            .await;
        tracing::info!(
            target: "mcp_timing",
            request_id = %request_id,
            method = "list_tools",
            "request completed"
        );
        out
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let name = request.name.clone();
        let start = Instant::now();
        let request_id = gen_request_id();
        // Gateway mode: invoke_tool(name, arguments) redirects to the inner tool with same context.
        let (request, name_for_log) = if name.as_ref() == "invoke_tool" {
            let m = match request.arguments.as_ref() {
                Some(m) => m,
                None => {
                    let elapsed_ms = start.elapsed().as_millis();
                    tracing::info!(target: "mcp_timing", request_id = %request_id, tool = "invoke_tool", elapsed_ms = %elapsed_ms, "tool call completed");
                    return Err(McpError::invalid_params(
                        "invoke_tool requires arguments: { name: string, arguments?: object }",
                        None,
                    ));
                }
            };
            let inner_name = match m.get("name").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => {
                    let elapsed_ms = start.elapsed().as_millis();
                    tracing::info!(target: "mcp_timing", request_id = %request_id, tool = "invoke_tool", elapsed_ms = %elapsed_ms, "tool call completed");
                    return Err(McpError::invalid_params(
                        "invoke_tool requires arguments.name (string)",
                        None,
                    ));
                }
            };
            let inner_args: rmcp::model::JsonObject = m
                .get("arguments")
                .and_then(|v| {
                    // Happy path: client sent an inline object.
                    v.as_object().cloned().or_else(|| {
                        // Fallback: some clients serialise the arguments object as a JSON
                        // string (occurs when the schema exposes serde_json::Value with no
                        // type hint and the client treats it as opaque text).
                        v.as_str()
                            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                            .and_then(|parsed| match parsed {
                                serde_json::Value::Object(map) => Some(map),
                                _ => None,
                            })
                    })
                })
                .unwrap_or_else(serde_json::Map::new);
            let inner_request = CallToolRequestParams {
                meta: None,
                name: std::borrow::Cow::Owned(inner_name.clone()),
                arguments: Some(inner_args),
                task: None,
            };
            (inner_request, inner_name)
        } else {
            (request, name.to_string())
        };

        // Argument-aware loop guard: block after N identical (tool, args) calls.
        if std::env::var("MCP_LOOP_GUARD_DISABLED")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true"))
            .unwrap_or(false)
        {
            // Guard disabled, skip.
        } else {
            let threshold = std::env::var("MCP_LOOP_GUARD_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(5)
                .clamp(2, 20);
            let args_hash = loop_guard_args_hash(request.arguments.as_ref());
            let key = (name_for_log.clone(), args_hash);
            let buffer = LOOP_GUARD_BUFFER.get_or_init(|| Mutex::new(VecDeque::new()));
            if let Ok(mut guard) = buffer.lock() {
                guard.push_back(key.clone());
                if guard.len() >= threshold {
                    let tail: Vec<_> = guard.iter().rev().take(threshold).cloned().collect();
                    let all_same = tail.len() == threshold
                        && tail.iter().all(|(n, a)| n == &key.0 && a == &key.1);
                    if all_same {
                        guard.clear();
                        drop(guard);
                        return Err(McpError::invalid_params(
                            format!(
                                "Loop guard: same tool with same arguments called {} times in a row. \
                                 Try a different approach or different arguments.",
                                threshold
                            ),
                            None,
                        ));
                    }
                }
                while guard.len() > threshold {
                    guard.pop_front();
                }
            }
        }

        let session_limit = std::env::var("MCP_MAX_TOOL_CALLS_PER_SESSION")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        if session_limit > 0 {
            let prev = SESSION_TOOL_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
            if prev >= session_limit {
                return Err(McpError::invalid_params(
                    "Session tool call limit reached (MCP_MAX_TOOL_CALLS_PER_SESSION).",
                    None,
                ));
            }
        }

        let params_str = request
            .arguments
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default())
            .unwrap_or_default();
        let tool_timeout = tool_timeout_secs(&name_for_log);
        let tool_context =
            rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        let out = match tokio::time::timeout(
            std::time::Duration::from_secs(tool_timeout),
            REQUEST_ID.scope(request_id.clone(), self.tool_router.call(tool_context)),
        )
        .await
        {
            Ok(result) => result,
            Err(_elapsed) => {
                tracing::error!(
                    target: "mcp_timing",
                    request_id = %request_id,
                    tool = %name_for_log,
                    timeout_secs = tool_timeout,
                    "tool call TIMED OUT"
                );
                Err(McpError::internal_error(
                    format!("Tool '{}' timed out after {}s", name_for_log, tool_timeout),
                    None,
                ))
            }
        };
        let elapsed_ms = start.elapsed().as_millis();
        // Record metrics for the in-process registry.
        crate::metrics::TOOL_CALLS_TOTAL.inc();
        crate::metrics::TOOL_LATENCY.record(elapsed_ms as u64);
        if out.is_err() {
            crate::metrics::TOOL_ERRORS.inc();
        }
        // Request-scoped ID for session/turn correlation (P1 observability).
        tracing::info!(
            target: "mcp_timing",
            request_id = %request_id,
            tool = %name_for_log,
            elapsed_ms = %elapsed_ms,
            "tool call completed"
        );
        // Optional audit log (MCP_AUDIT_LOG_PATH): one JSONL line per tool call.
        if let Ok(log_path) = std::env::var("MCP_AUDIT_LOG_PATH") {
            let params_hash = blake3::hash(params_str.as_bytes()).to_hex();
            let params_hash_short = params_hash.chars().take(16).collect::<String>();
            let error_field = out.as_ref().err().map(|e| e.to_string());
            let line = serde_json::json!({
                "request_id": request_id,
                "tool": name_for_log,
                "params_hash": params_hash_short,
                "elapsed_ms": elapsed_ms,
                "error": error_field
            });
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
            {
                use std::io::Write;
                let _ = writeln!(f, "{}", line);
                let _ = f.flush();
            }
        }
        out
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        let request_id = gen_request_id();
        let out = REQUEST_ID
            .scope(request_id.clone(), async {
                let templates = vec![
                    RawResourceTemplate {
                        uri_template: "graph://symbol/{name}".into(),
                        name: "Symbol".into(),
                        title: None,
                        description: Some(
                            "Browsable symbol: definition plus references with graph links.".into(),
                        ),
                        mime_type: Some("text/xml".into()),
                        icons: None,
                    }
                    .no_annotation(),
                    RawResourceTemplate {
                        uri_template: "rag://index/{path}".into(),
                        name: "Indexed file chunks".into(),
                        title: None,
                        description: Some(
                            "Direct access to indexed chunks for a specific file.".into(),
                        ),
                        mime_type: Some("text/plain".into()),
                        icons: None,
                    }
                    .no_annotation(),
                ];
                Ok(ListResourceTemplatesResult {
                    resource_templates: templates,
                    next_cursor: None,
                    meta: None,
                })
            })
            .await;
        tracing::info!(
            target: "mcp_timing",
            request_id = %request_id,
            method = "list_resource_templates",
            "request completed"
        );
        out
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let request_id = gen_request_id();
        let uri_for_log = request.uri.to_string();
        let out = REQUEST_ID
            .scope(request_id.clone(), async {
                let uri = &request.uri;
                if uri.starts_with("graph://symbol/") {
                    let name = uri.trim_start_matches("graph://symbol/");
                    let name =
                        urlencoding::decode(name).unwrap_or(std::borrow::Cow::Borrowed(name));
                    let name = name.trim();
                    if name.contains('/') || name.contains('\\') {
                        return Err(McpError::invalid_params("Invalid symbol name.", None));
                    }
                    let xml = rag::symbol_xml(&self.store, name);
                    let text = truncate_rag_response(&self.store, &xml);
                    return Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("text/xml".into()),
                            text,
                            meta: None,
                        }],
                    });
                }
                if uri.starts_with("rag://index/") {
                    let path = uri.trim_start_matches("rag://index/");
                    let path =
                        urlencoding::decode(path).unwrap_or(std::borrow::Cow::Borrowed(path));
                    let path = path.trim();
                    if !self.store.path_under_allowed(path) {
                        return Err(McpError::invalid_params(
                            format!("Access denied: {} is not in ALLOWED_ROOTS", path),
                            None,
                        ));
                    }
                    let rows = self.store.get_chunks_by_source(path).map_err(
                        |e: crate::rag::db::RagDbError| {
                            McpError::internal_error(e.to_string(), None)
                        },
                    )?;
                    let text = truncate_rag_response(
                        &self.store,
                        &format_sandbox_response(&rows, &self.store.allowed_roots),
                    );
                    return Ok(ReadResourceResult {
                        contents: vec![ResourceContents::TextResourceContents {
                            uri: uri.clone(),
                            mime_type: Some("text/plain".into()),
                            text,
                            meta: None,
                        }],
                    });
                }
                Err(McpError::invalid_params(
                    "Unknown resource URI scheme.",
                    None,
                ))
            })
            .await;
        tracing::info!(
            target: "mcp_timing",
            request_id = %request_id,
            method = "read_resource",
            uri = %uri_for_log,
            "request completed"
        );
        out
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let request_id = gen_request_id();
        let out = REQUEST_ID
            .scope(request_id.clone(), async {
                let prompts = vec![
                    rmcp::model::Prompt {
                        name: "analyze_architecture".into(),
                title: None,
                description: Some("Pre-loads the LLM with high-level architectural context found in the index.".into()),
                arguments: Some(vec![]),
                icons: None,
                meta: None,
            },
            rmcp::model::Prompt {
                name: "refactor_component".into(),
                title: None,
                description: Some("Pre-loads the definition and references for a symbol so you can refactor it with full context.".into()),
                arguments: Some(vec![rmcp::model::PromptArgument {
                    name: "symbol_name".into(),
                    title: None,
                    description: None,
                    required: Some(true),
                }]),
                icons: None,
                meta: None,
            },
            rmcp::model::Prompt {
                name: "debug_symbol".into(),
                title: None,
                description: Some("Pre-loads the symbol and its references to debug issues related to it.".into()),
                arguments: Some(vec![rmcp::model::PromptArgument {
                    name: "symbol_name".into(),
                    title: None,
                    description: None,
                    required: Some(true),
                }]),
                icons: None,
                meta: None,
            },
            rmcp::model::Prompt {
                name: "explain_codebase".into(),
                title: None,
                description: Some("High-level overview of the codebase: what it does, main modules, and how they fit together.".into()),
                arguments: Some(vec![]),
                icons: None,
                meta: None,
            },
            rmcp::model::Prompt {
                name: "analyze_error_log".into(),
                title: None,
                description: Some("Analyze error output using RAG context and lessons_learned; suggest root cause and concrete fix.".into()),
                arguments: Some(vec![
                    rmcp::model::PromptArgument {
                        name: "error_output".into(),
                        title: None,
                        description: Some("Raw error or build/test output to analyze.".into()),
                        required: Some(true),
                    },
                    rmcp::model::PromptArgument {
                        name: "recent_errors".into(),
                        title: None,
                        description: Some("Optional: concatenated recent error outputs from the last 2-3 attempts; used to detect oscillation (same error recurring).".into()),
                        required: Some(false),
                    },
                ]),
                icons: None,
                meta: None,
            },
            rmcp::model::Prompt {
                name: "review_diff".into(),
                title: None,
                description: Some("Audit a code diff with the local LLM: security, unwrap(), and bad practices. Use before committing changes.".into()),
                arguments: Some(vec![rmcp::model::PromptArgument {
                    name: "diff".into(),
                    title: None,
                    description: Some("Unified diff or changed code to audit.".into()),
                    required: Some(true),
                }]),
                icons: None,
                meta: None,
            },
            rmcp::model::Prompt {
                name: "scaffold_reproduction_test".into(),
                title: None,
                description: Some("TDD: Before fixing a logic bug, you MUST write a test that fails. This prompt returns context and instructions to scaffold that reproduction test.".into()),
                arguments: Some(vec![
                    rmcp::model::PromptArgument {
                        name: "bug_description".into(),
                        title: None,
                        description: Some("What is wrong (e.g. 'function returns wrong value when input is negative').".into()),
                        required: Some(true),
                    },
                    rmcp::model::PromptArgument {
                        name: "error_output".into(),
                        title: None,
                        description: Some("Optional: raw error or failure output.".into()),
                        required: Some(false),
                    },
                ]),
                icons: None,
                meta: None,
            },
        ];
                Ok(ListPromptsResult {
                    prompts,
                    next_cursor: None,
                    meta: None,
                })
            })
            .await;
        tracing::info!(
            target: "mcp_timing",
            request_id = %request_id,
            method = "list_prompts",
            "request completed"
        );
        out
    }

    async fn get_prompt(
        &self,
        params: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let request_id = gen_request_id();
        let name_for_log = params.name.to_string();
        let out = REQUEST_ID
            .scope(request_id.clone(), async {
                let name = params.name.as_str();
                let arguments = params.arguments.as_ref();
                let messages = match name {
            "analyze_architecture" => {
                let rows = self
                    .store
                    .hybrid_search(
                        "architecture system design patterns overview structure README",
                        5,
                        None,
                    )
                    .map_err(|e: crate::rag::db::RagDbError| {
                        McpError::internal_error(e.to_string(), None)
                    })?;
                let context = truncate_rag_response(
                    &self.store,
                    &format_sandbox_response(&rows, &self.store.allowed_roots),
                );
                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        r#"You are the Principal Software Architect.
Review the provided context below, which contains high-level documentation and structural code snippets from the repository.

{}

Based on this, please provide:
1. A high-level summary of the system architecture.
2. Key design patterns identified.
3. Observations on code organization."#,
                        context
                    ),
                )]
            }
            "refactor_component" => {
                let symbol_name = arguments
                    .and_then(|a| a.get("symbol_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let symbol_content = symbol_xml(&self.store, &symbol_name);
                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        r#"Refactor the component '{}'. Below is its definition and where it is used (with graph links). Preserve behavior and improve structure, tests, or clarity as needed.

{}"#,
                        symbol_name, symbol_content
                    ),
                )]
            }
            "debug_symbol" => {
                let symbol_name = arguments
                    .and_then(|a| a.get("symbol_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let symbol_content = symbol_xml(&self.store, &symbol_name);
                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        r#"Debug issues related to '{}'. Below is its definition and references. Identify possible causes of bugs, race conditions, or misuse.

{}"#,
                        symbol_name, symbol_content
                    ),
                )]
            }
            "explain_codebase" => {
                let rows = self
                    .store
                    .hybrid_search(
                        "overview main modules entry points README architecture",
                        5,
                        None,
                    )
                    .map_err(|e: crate::rag::db::RagDbError| {
                        McpError::internal_error(e.to_string(), None)
                    })?;
                let context = truncate_rag_response(
                    &self.store,
                    &format_sandbox_response(&rows, &self.store.allowed_roots),
                );
                vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        r#"Explain this codebase at a high level. Use the context below.

{}

Provide: 1) What the project does. 2) Main modules or layers. 3) How they connect."#,
                        context
                    ),
                )]
            }
            "analyze_error_log" => {
                let error_output = arguments
                    .and_then(|a| a.get("error_output"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let recent_errors = arguments
                    .and_then(|a| a.get("recent_errors"))
                    .and_then(|v| v.as_str());
                let message =
                    analysis::build_analyze_error_log_text(self, error_output, recent_errors)?;
                vec![PromptMessage::new_text(PromptMessageRole::User, message)]
            }
            "scaffold_reproduction_test" => {
                let bug_description = arguments
                    .and_then(|a| a.get("bug_description"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let error_output = arguments
                    .and_then(|a| a.get("error_output"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let message = analysis::build_scaffold_reproduction_test_text(
                    self,
                    bug_description,
                    error_output,
                )?;
                vec![PromptMessage::new_text(PromptMessageRole::User, message)]
            }
            "review_diff" => {
                let diff = arguments
                    .and_then(|a| a.get("diff"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let (user_content, audit_opt) =
                    analysis::build_review_diff_audit(self, diff, None).await?;
                if let Some(audit_text) = audit_opt {
                    vec![
                        PromptMessage::new_text(PromptMessageRole::User, user_content),
                        PromptMessage::new_text(PromptMessageRole::Assistant, audit_text),
                    ]
                } else {
                    vec![PromptMessage::new_text(
                        PromptMessageRole::User,
                        user_content,
                    )]
                }
            }
            _ => return Err(McpError::invalid_params("Unknown prompt.", None)),
        };
                Ok(GetPromptResult {
                    description: None,
                    messages,
                })
            })
            .await;
        tracing::info!(
            target: "mcp_timing",
            request_id = %request_id,
            method = "get_prompt",
            name = %name_for_log,
            "request completed"
        );
        out
    }
}

#[cfg(test)]
mod tests;
