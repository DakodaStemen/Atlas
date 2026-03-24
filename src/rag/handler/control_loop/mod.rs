//! Multi-agent control loop: ManagedLoop wraps AgenticHandler to provide autonomous
//! execution with convergence monitoring, stagnation detection, and structured audit
//! integration. The loop tracks progress toward objectives and can halt on stagnation
//! or divergence.

use crate::rag::handler::analysis::SkepticReviewParams;
use crate::rag::handler::model_routing::{ModelRole, ModelRouter};
use crate::rag::handler::{AgenticHandler, DefaultIngestion, DefaultStorage};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
    ListPromptsResult, ListResourceTemplatesResult, ListToolsResult, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResult, ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Configuration constants (overridable via env)
// ---------------------------------------------------------------------------

/// Default max iterations before forced halt.
const DEFAULT_MAX_ITERATIONS: u32 = 20;
/// Consecutive iterations with no score improvement before declaring stagnation.
const DEFAULT_STAGNATION_THRESHOLD: u32 = 3;
/// Minimum audit score (0.0-1.0) to consider the task "done".
const DEFAULT_CONVERGENCE_THRESHOLD: f32 = 0.95;
/// Max consecutive failures on the same tool+args before triggering circuit breaker.
const DEFAULT_DEAD_END_THRESHOLD: u32 = 2;

fn read_max_iterations() -> u32 {
    std::env::var("MCP_MAX_LOOP_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_ITERATIONS)
}

fn read_stagnation_threshold() -> u32 {
    std::env::var("MCP_STAGNATION_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_STAGNATION_THRESHOLD)
}

fn read_convergence_threshold() -> f32 {
    std::env::var("MCP_CONVERGENCE_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_CONVERGENCE_THRESHOLD)
}

// ---------------------------------------------------------------------------
// Loop state
// ---------------------------------------------------------------------------

/// Status of the control loop.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LoopStatus {
    /// Not yet started.
    Idle,
    /// Actively executing iterations.
    Running,
    /// Converged: audit score exceeded threshold.
    Converged,
    /// Halted: max iterations reached.
    MaxIterationsReached,
    /// Halted: no improvement for N consecutive iterations.
    Stagnated,
    /// Halted: explicit stop requested.
    Stopped,
    /// Halted: unrecoverable error.
    Failed,
}

impl std::fmt::Display for LoopStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopStatus::Idle => write!(f, "idle"),
            LoopStatus::Running => write!(f, "running"),
            LoopStatus::Converged => write!(f, "converged"),
            LoopStatus::MaxIterationsReached => write!(f, "max_iterations_reached"),
            LoopStatus::Stagnated => write!(f, "stagnated"),
            LoopStatus::Stopped => write!(f, "stopped"),
            LoopStatus::Failed => write!(f, "failed"),
        }
    }
}

/// A single iteration record.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IterationRecord {
    /// 1-indexed iteration number.
    pub iteration: u32,
    /// Tool that was called.
    pub tool_name: String,
    /// Whether the tool call succeeded.
    pub success: bool,
    /// Audit score after this iteration (0.0-1.0), if available.
    pub score: Option<f32>,
    /// Duration of the tool call in milliseconds.
    pub duration_ms: u64,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Full state of a managed control loop.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LoopState {
    /// Current iteration count.
    pub current_iteration: u32,
    /// Maximum allowed iterations.
    pub max_iterations: u32,
    /// Current loop status.
    pub status: LoopStatus,
    /// Latest audit/progress score (0.0 to 1.0).
    pub last_score: f32,
    /// Peak score achieved during this run.
    pub best_score: f32,
    /// Number of consecutive iterations with no score improvement.
    pub stagnation_count: u32,
    /// Stagnation threshold before halting.
    pub stagnation_threshold: u32,
    /// Convergence threshold (score above this = done).
    pub convergence_threshold: f32,
    /// History of all iterations.
    pub history: Vec<IterationRecord>,
    /// Objective being pursued (if set).
    pub objective: Option<String>,
    /// Timestamp when the loop started.
    pub started_at: Option<String>,
    /// Tracks repeated failures by file/tool to detect dead-ends.
    /// Key: (tool_name, args_hash), Value: consecutive failure count.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[schemars(skip)]
    pub dead_end_tracker: HashMap<String, u32>,
}

impl Default for LoopState {
    fn default() -> Self {
        Self {
            current_iteration: 0,
            max_iterations: read_max_iterations(),
            status: LoopStatus::Idle,
            last_score: 0.0,
            best_score: 0.0,
            stagnation_count: 0,
            stagnation_threshold: read_stagnation_threshold(),
            convergence_threshold: read_convergence_threshold(),
            history: Vec::new(),
            objective: None,
            started_at: None,
            dead_end_tracker: HashMap::new(),
        }
    }
}

impl LoopState {
    /// Update the score and check for stagnation / convergence.
    pub fn update_score(&mut self, new_score: f32) {
        // Use epsilon to avoid float comparison false positives on identical scores.
        const EPSILON: f32 = 1e-6;
        if new_score > self.best_score + EPSILON {
            self.best_score = new_score;
            self.stagnation_count = 0;
        } else {
            self.stagnation_count += 1;
        }
        self.last_score = new_score;

        if self.last_score >= self.convergence_threshold {
            self.status = LoopStatus::Converged;
        } else if self.stagnation_count >= self.stagnation_threshold {
            self.status = LoopStatus::Stagnated;
        }
    }

    /// Record a completed iteration.
    pub fn record_iteration(&mut self, tool_name: &str, success: bool, duration_ms: u64) {
        let record = IterationRecord {
            iteration: self.current_iteration,
            tool_name: tool_name.to_string(),
            success,
            score: Some(self.last_score),
            duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.history.push(record);
    }

    /// Check if the loop should continue.
    pub fn should_continue(&self) -> bool {
        matches!(self.status, LoopStatus::Running | LoopStatus::Idle)
    }

    /// Record a tool failure and check for dead-end.
    /// Returns Some(message) if dead-end detected.
    pub fn record_failure(&mut self, tool_name: &str, args_summary: &str) -> Option<String> {
        let key = format!("{}::{}", tool_name, args_summary);
        let count = self.dead_end_tracker.entry(key).or_insert(0);
        *count += 1;
        let threshold = std::env::var("MCP_DEAD_END_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_DEAD_END_THRESHOLD);
        if *count >= threshold {
            Some(format!(
                "DEAD-END DETECTED: '{}' has failed {} times with similar arguments. \
                 You are NOT allowed to retry this approach. You MUST: \
                 (1) use get_related_code to find a different approach, \
                 (2) decompose the problem differently, or \
                 (3) escalate to human.",
                tool_name, count
            ))
        } else {
            None
        }
    }

    /// Clear dead-end tracker for a tool (call after success).
    pub fn clear_dead_end(&mut self, tool_name: &str) {
        self.dead_end_tracker
            .retain(|k, _| !k.starts_with(&format!("{}::", tool_name)));
    }

    /// Get a summary string for logging/responses.
    pub fn summary(&self) -> String {
        format!(
            "iteration={}/{}, status={}, score={:.3} (best={:.3}), stagnation={}/{}",
            self.current_iteration,
            self.max_iterations,
            self.status,
            self.last_score,
            self.best_score,
            self.stagnation_count,
            self.stagnation_threshold,
        )
    }
}

// ---------------------------------------------------------------------------
// ManagedLoop
// ---------------------------------------------------------------------------

/// Snapshot of git status for ghost file detection.
#[derive(Clone, Debug, Default)]
pub struct GitSnapshot {
    pub untracked: std::collections::HashSet<String>,
    pub modified: std::collections::HashSet<String>,
}

/// ManagedLoop wraps an AgenticHandler to provide autonomous control loop features.
/// It intercepts tool calls to track progress, detect stagnation, and enforce convergence.
pub struct ManagedLoop {
    pub handler: Arc<AgenticHandler<DefaultIngestion, DefaultStorage>>,
    pub state: std::sync::Mutex<LoopState>,
    pub router: Arc<ModelRouter>,
}

impl ManagedLoop {
    pub fn new(handler: Arc<AgenticHandler<DefaultIngestion, DefaultStorage>>) -> Self {
        Self {
            handler,
            state: std::sync::Mutex::new(LoopState::default()),
            router: Arc::new(ModelRouter::new()),
        }
    }

    /// Reset the loop state for a new task.
    pub fn reset(&self, max_iterations: u32) {
        if let Ok(mut state) = self.state.lock() {
            *state = LoopState {
                max_iterations,
                started_at: Some(chrono::Utc::now().to_rfc3339()),
                status: LoopStatus::Running,
                ..LoopState::default()
            };
        }
    }

    /// Set the objective being pursued.
    pub fn set_objective(&self, objective: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.objective = Some(objective.to_string());
        }
    }

    /// Manually update the progress score (e.g., after an external audit).
    pub fn update_score(&self, score: f32) {
        if let Ok(mut state) = self.state.lock() {
            state.update_score(score);
        }
    }

    /// Stop the loop gracefully.
    pub fn stop(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.status = LoopStatus::Stopped;
        }
    }

    /// Get a snapshot of the current state.
    pub fn snapshot(&self) -> LoopState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Check if a tool name is an audit tool (used to auto-extract scores).
    fn is_audit_tool(tool_name: &str) -> bool {
        matches!(
            tool_name,
            "verify_integrity" | "aggregate_audit" | "security_audit" | "scan_secrets"
        )
    }

    /// Capture current git status for ghost file detection.
    fn capture_git_snapshot_sync(project_root: &std::path::Path) -> GitSnapshot {
        let output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(project_root)
            .output();
        let mut snapshot = GitSnapshot::default();
        if let Ok(o) = output {
            let stdout = String::from_utf8_lossy(&o.stdout);
            for line in stdout.lines() {
                if line.len() < 4 {
                    continue;
                }
                let status = &line[..2];
                let file = line[3..].trim().to_string();
                if status.starts_with("??") {
                    snapshot.untracked.insert(file);
                } else {
                    snapshot.modified.insert(file);
                }
            }
        }
        snapshot
    }

    /// Detect new untracked files that appeared between two snapshots.
    fn detect_ghost_files(before: &GitSnapshot, after: &GitSnapshot) -> Vec<String> {
        after
            .untracked
            .difference(&before.untracked)
            .cloned()
            .collect()
    }

    /// Check if a tool is likely to modify the filesystem.
    fn is_fs_modifying_tool(tool_name: &str) -> bool {
        matches!(
            tool_name,
            "execute_shell_command"
                | "git_checkpoint"
                | "commit_to_memory"
                | "save_rule_to_memory"
                | "refresh_file_index"
                | "propose_vault_rule"
                | "compile_rules"
        )
    }

    /// Physical revert of current changes via `git stash`, gated behind
    /// `MCP_AUTO_REVERT_ENABLED=1`.  When the env var is unset or not "1",
    /// the revert is skipped and `Err` carries the rejection message so the
    /// caller can surface it without destroying work.
    async fn auto_revert(&self, project_root: &std::path::Path) -> Result<(), String> {
        let enabled = std::env::var("MCP_AUTO_REVERT_ENABLED")
            .map(|v| v == "1")
            .unwrap_or(false);

        if !enabled {
            tracing::warn!(
                target: "control_loop",
                "Skeptic rejected code but MCP_AUTO_REVERT_ENABLED is not set — skipping auto-revert. \
                 Set MCP_AUTO_REVERT_ENABLED=1 to enable automatic stashing of rejected changes."
            );
            return Err("Auto-revert disabled (MCP_AUTO_REVERT_ENABLED != 1). \
                        Skeptic rejected the change but no revert was performed."
                .to_string());
        }

        let output = tokio::process::Command::new("git")
            .args([
                "stash",
                "push",
                "-m",
                "auto-revert: skeptic rejected change",
            ])
            .current_dir(project_root)
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            tracing::info!(
                target: "control_loop",
                "Skeptic-rejected changes stashed. Recover with: git stash pop"
            );
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Try to extract a score from an audit tool result.
    fn extract_score_from_result(result: &CallToolResult) -> Option<f32> {
        for content in &result.content {
            if let Some(text) = content.as_text() {
                // Try to parse JSON with a "score" field
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(text.text.as_ref()) {
                    if let Some(score) = val.get("score").and_then(|s| s.as_f64()) {
                        return Some(score as f32);
                    }
                }
                // Check for pass/fail in verify_integrity output
                if text.text.contains("\"pass\": true") || text.text.contains("\"pass\":true") {
                    return Some(1.0);
                }
                if text.text.contains("\"pass\": false") || text.text.contains("\"pass\":false") {
                    return Some(0.0);
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// GetLoopStateParams — for the get_loop_state tool
// ---------------------------------------------------------------------------

#[derive(Clone, Default, Serialize, Deserialize, JsonSchema)]
/// GetLoopStateParams.
pub struct GetLoopStateParams {
    /// If true, include full iteration history. Default false (summary only).
    #[serde(default)]
    pub include_history: Option<bool>,
}

/// MCP tool: query the current control loop state.
/// When called via ManagedLoop, returns real state.
/// When called directly on AgenticHandler, returns default idle state.
pub async fn get_loop_state_impl(
    managed: &ManagedLoop,
    params: GetLoopStateParams,
) -> Result<CallToolResult, McpError> {
    let state = managed.snapshot();
    let include_history = params.include_history.unwrap_or(false);

    let output = if include_history {
        serde_json::to_string_pretty(&state)
    } else {
        // Summary without full history
        let summary = serde_json::json!({
            "current_iteration": state.current_iteration,
            "max_iterations": state.max_iterations,
            "status": state.status,
            "last_score": state.last_score,
            "best_score": state.best_score,
            "stagnation_count": state.stagnation_count,
            "stagnation_threshold": state.stagnation_threshold,
            "convergence_threshold": state.convergence_threshold,
            "objective": state.objective,
            "started_at": state.started_at,
            "total_iterations_completed": state.history.len(),
        });
        serde_json::to_string_pretty(&summary)
    };

    let text = output.map_err(|e| McpError::internal_error(format!("JSON: {}", e), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

/// Handler-compatible adapter: returns default loop state when not in managed mode.
pub async fn get_loop_state_handler<I, S>(
    _handler: &crate::rag::handler::AgenticHandler<I, S>,
    params: GetLoopStateParams,
) -> Result<CallToolResult, McpError>
where
    I: crate::rag::handler::IngestionProvider + Send + Sync,
    S: crate::rag::handler::VectorStoreProvider + Send + Sync,
{
    let state = LoopState::default();
    let include_history = params.include_history.unwrap_or(false);
    let output = if include_history {
        serde_json::to_string_pretty(&state)
    } else {
        let summary = serde_json::json!({
            "current_iteration": state.current_iteration,
            "max_iterations": state.max_iterations,
            "status": state.status,
            "last_score": state.last_score,
            "best_score": state.best_score,
            "stagnation_count": state.stagnation_count,
            "objective": state.objective,
            "note": "Not in managed loop mode. State reflects defaults."
        });
        serde_json::to_string_pretty(&summary)
    };
    let text = output.map_err(|e| McpError::internal_error(format!("JSON: {}", e), None))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

// ---------------------------------------------------------------------------
// ServerHandler impl
// ---------------------------------------------------------------------------

#[allow(clippy::manual_async_fn)]
impl ServerHandler for ManagedLoop {
    fn get_info(&self) -> ServerInfo {
        let mut info = self.handler.get_info();
        if let Some(ref mut instructions) = info.instructions {
            let state = self.snapshot();
            let managed_instr = format!(
                "\nManaged Loop ACTIVE ({}). Convergence tracking enabled. \
                 Score: {:.3}, Iterations: {}/{}, Stagnation: {}/{}.",
                state.status,
                state.last_score,
                state.current_iteration,
                state.max_iterations,
                state.stagnation_count,
                state.stagnation_threshold,
            );
            instructions.push_str(&managed_instr);
        }
        info
    }

    fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        self.handler.list_tools(request, context)
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let tool_name = request.name.to_string();

            // Gateway interception: get_loop_state
            if tool_name == "get_loop_state" {
                let params: GetLoopStateParams = match request.arguments {
                    Some(args) => {
                        serde_json::from_value(serde_json::Value::Object(args)).unwrap_or_default()
                    }
                    None => GetLoopStateParams::default(),
                };
                return get_loop_state_impl(self, params).await;
            }

            let args_hash = crate::rag::handler::loop_guard_args_hash(request.arguments.as_ref());
            let start = Instant::now();

            // 1. Pre-check: should we continue?
            {
                let mut state = self.state.lock().map_err(|_| {
                    McpError::internal_error("ManagedLoop state lock poisoned", None)
                })?;

                if state.status == LoopStatus::Idle {
                    state.status = LoopStatus::Running;
                    state.started_at = Some(chrono::Utc::now().to_rfc3339());
                }

                state.current_iteration += 1;

                if state.current_iteration > state.max_iterations {
                    state.status = LoopStatus::MaxIterationsReached;
                    let summary = state.summary();
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "Managed Loop HALTED: max iterations exceeded. {}\n\
                         Action: Decompose the task further, switch strategy, or escalate to human.",
                        summary
                    ))]));
                }

                if !state.should_continue() {
                    let summary = state.summary();
                    return Ok(CallToolResult::success(vec![Content::text(format!(
                        "Managed Loop HALTED: {}. {}\n\
                         Action: Reset the loop with a new objective or escalate.",
                        state.status, summary
                    ))]));
                }
            }

            // 2. Ghost File Guard: capture pre-snapshot for fs-modifying tools
            let is_fs_tool = Self::is_fs_modifying_tool(&tool_name);
            let project_root = std::env::var("ALLOWED_ROOTS").unwrap_or_else(|_| ".".to_string());
            let project_root =
                std::path::PathBuf::from(project_root.split(':').next().unwrap_or("."));
            let pre_snapshot = if is_fs_tool {
                Some(Self::capture_git_snapshot_sync(&project_root))
            } else {
                None
            };

            // 3. Execute the tool (with per-tool timeout)
            let tool_timeout = super::tool_timeout_secs(&tool_name);
            let result = match tokio::time::timeout(
                std::time::Duration::from_secs(tool_timeout),
                self.handler.call_tool(request, context),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => {
                    tracing::error!(
                        target: "mcp_timing",
                        tool = %tool_name,
                        timeout_secs = tool_timeout,
                        "tool call TIMED OUT in control loop"
                    );
                    Err(McpError::internal_error(
                        format!("Tool '{}' timed out after {}s", tool_name, tool_timeout),
                        None,
                    ))
                }
            };
            let duration_ms = start.elapsed().as_millis() as u64;
            let success = result.is_ok();

            // 4. THE SKEPTIC PROTOCOL: Adversarial Review
            let mut skeptic_rejected = false;
            let mut skeptic_feedback = String::new();

            if is_fs_tool && success {
                let git_diff = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    tokio::process::Command::new("git")
                        .args(["diff", "HEAD"])
                        .current_dir(&project_root)
                        .output(),
                )
                .await
                .map_err(|_| {
                    McpError::internal_error("Git diff timed out after 30s".to_string(), None)
                })?
                .map_err(|e| McpError::internal_error(format!("Git diff error: {}", e), None))?;

                let diff_text = String::from_utf8_lossy(&git_diff.stdout).to_string();
                if !diff_text.is_empty() {
                    let objective = {
                        let state = self.state.lock().map_err(|_| {
                            McpError::internal_error("ManagedLoop state lock poisoned", None)
                        })?;
                        state.objective.clone()
                    };

                    // Use the existing skeptic logic to build the prompt
                    let skeptic_params = SkepticReviewParams {
                        diff: diff_text,
                        objective,
                    };

                    let skeptic_prompt_res = crate::rag::handler::analysis::skeptic_review_impl(
                        &self.handler,
                        skeptic_params,
                    )
                    .await?;

                    if let Some(prompt_text) = skeptic_prompt_res.content[0].as_text() {
                        // Call the routed model (Gemini 1.5 Pro)
                        let review_json = self
                            .router
                            .call_model(ModelRole::Skeptic, &prompt_text.text)
                            .await
                            .unwrap_or_else(|e| format!(r#"{{"requires_retry": false, "summary": "Skeptic API failed: {}"}}"#, e));

                        // Parse verdict
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&review_json) {
                            if v["requires_retry"].as_bool().unwrap_or(false) {
                                skeptic_rejected = true;
                                skeptic_feedback = v["summary"]
                                    .as_str()
                                    .unwrap_or("Skeptic rejected code without summary")
                                    .to_string();
                                // Attempt revert (gated by MCP_AUTO_REVERT_ENABLED)
                                match self.auto_revert(&project_root).await {
                                    Ok(()) => {
                                        skeptic_feedback.push_str(
                                            " | Changes stashed (recover with: git stash pop).",
                                        );
                                    }
                                    Err(msg) => {
                                        skeptic_feedback.push_str(&format!(" | {}", msg));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 5. Post-execution: update state
            if let Ok(ref res) = result {
                if skeptic_rejected {
                    let mut contents = res.content.clone();
                    contents.push(Content::text(format!(
                        "\n\nSKEPTIC REJECTED CHANGE: {}\nPlease rethink your approach.",
                        skeptic_feedback
                    )));
                    return Ok(CallToolResult {
                        content: contents,
                        ..res.clone()
                    });
                }

                let mut state = self.state.lock().map_err(|_| {
                    McpError::internal_error("ManagedLoop state lock poisoned", None)
                })?;

                // Auto-extract score from audit tools
                if Self::is_audit_tool(&tool_name) {
                    if let Some(score) = Self::extract_score_from_result(res) {
                        state.update_score(score);
                    }
                }

                state.record_iteration(&tool_name, success, duration_ms);
                // Clear dead-end tracker on success
                state.clear_dead_end(&tool_name);

                // Check if we just hit stagnation
                if state.status == LoopStatus::Stagnated {
                    let summary = state.summary();
                    // Append stagnation warning to result
                    let mut contents = res.content.clone();
                    contents.push(Content::text(format!(
                        "\n\nSTAGNATION DETECTED: {}. Consider: (1) switch strategy, \
                         (2) decompose further, (3) try different model, (4) escalate to human.",
                        summary
                    )));
                    return Ok(CallToolResult {
                        content: contents,
                        ..res.clone()
                    });
                }

                // Ghost File Guard: detect new untracked files after fs-modifying tools
                let ghost_files = if is_fs_tool {
                    if let Some(ref before) = pre_snapshot {
                        let after = Self::capture_git_snapshot_sync(&project_root);
                        let ghosts = Self::detect_ghost_files(before, &after);
                        if ghosts.is_empty() {
                            None
                        } else {
                            Some(ghosts)
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(ref ghosts) = ghost_files {
                    let mut contents = res.content.clone();
                    contents.push(Content::text(format!(
                        "\n\nGHOST FILE WARNING: New untracked files detected after tool call: {:?}. \
                         Review and clean up if unintended.",
                        ghosts
                    )));
                    return Ok(CallToolResult {
                        content: contents,
                        ..res.clone()
                    });
                }
            } else if let Ok(mut state) = self.state.lock() {
                // Tool failed — record failure and check dead-end circuit breaker
                state.record_iteration(&tool_name, false, duration_ms);
                if let Some(dead_end_msg) = state.record_failure(&tool_name, &args_hash) {
                    return Err(McpError::internal_error(
                        format!(
                            "{} | {}",
                            result
                                .as_ref()
                                .err()
                                .map(|e| e.to_string())
                                .unwrap_or_default(),
                            dead_end_msg
                        ),
                        None,
                    ));
                }
            }

            result
        }
    }

    fn list_resource_templates(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListResourceTemplatesResult, McpError>> + Send + '_ {
        self.handler.list_resource_templates(request, context)
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        self.handler.read_resource(request, context)
    }

    fn list_prompts(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        self.handler.list_prompts(request, context)
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        self.handler.get_prompt(request, context)
    }
}
