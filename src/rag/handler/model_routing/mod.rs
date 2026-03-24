//! Multi-model routing: role-based dispatch to Ollama (local) and Gemini (free tier).
//!
//! Model selection strategy (all $0):
//! - Planner: Gemini 2.5 Pro (free, 1M context, best reasoning)
//! - Executor: Gemini 2.5 Flash (free, 1M context, fast, thinking budgets)
//! - Skeptic: Gemini 2.5 Pro (free, different call = fresh perspective on same code)
//!   Falls back to Ollama deepseek-r1:32b if GOOGLE_API_KEY unset
//! - Humanizer: Gemini 2.5 Flash-Lite (free, fastest, cheapest for style passes)
//! - SuperExecutor: Claude Code CLI (escalation for stuck tasks)
//!
//! Env overrides: GEMINI_PLANNER_MODEL, GEMINI_EXECUTOR_MODEL, GEMINI_SKEPTIC_MODEL,
//!                GEMINI_HUMANIZER_MODEL, OLLAMA_HOST (default http://localhost:11434)

use crate::rag::handler::{AgenticHandler, IngestionProvider, VectorStoreProvider};
use reqwest::Client;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};

/// Maximum number of history entries retained by ModelRouter.
const MAX_HISTORY_ENTRIES: usize = 10;

// ---------------------------------------------------------------------------
// Default model IDs (free tier, March 2026)
// ---------------------------------------------------------------------------

/// Best free reasoning model — 1M context, complex decomposition.
const DEFAULT_PLANNER_MODEL: &str = "gemini-2.5-pro-preview-06-05";
/// Fast free model with thinking budgets — 1M context, tool use.
const DEFAULT_EXECUTOR_MODEL: &str = "gemini-2.5-flash-preview-05-20";
/// Same as planner but called separately for adversarial review.
const DEFAULT_SKEPTIC_MODEL: &str = "gemini-2.5-pro-preview-06-05";
/// Smallest/fastest free model — style passes, comment stripping.
const DEFAULT_HUMANIZER_MODEL: &str = "gemini-2.5-flash-lite-preview-06-17";
/// Local fallback skeptic — strong reasoning, fits in VRAM.
const DEFAULT_OLLAMA_SKEPTIC: &str = "qwen2.5-coder:32b";
/// Local fallback planner — distilled R1 reasoning, 8B fits in VRAM.
const DEFAULT_OLLAMA_PLANNER: &str = "deepseek-r1:8b";
/// Local humanizer (GPU) — small, instant.
const DEFAULT_OLLAMA_HUMANIZER: &str = "qwen2.5-coder:7b";

fn ollama_host() -> String {
    env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

fn model_for_role(role: &ModelRole) -> (&'static str, String) {
    match role {
        ModelRole::Planner => {
            let m = env::var("GEMINI_PLANNER_MODEL")
                .unwrap_or_else(|_| DEFAULT_PLANNER_MODEL.to_string());
            ("google", m)
        }
        ModelRole::Executor => {
            let m = env::var("GEMINI_EXECUTOR_MODEL")
                .unwrap_or_else(|_| DEFAULT_EXECUTOR_MODEL.to_string());
            ("google", m)
        }
        ModelRole::Skeptic => {
            let m = env::var("GEMINI_SKEPTIC_MODEL")
                .unwrap_or_else(|_| DEFAULT_SKEPTIC_MODEL.to_string());
            ("google", m)
        }
        ModelRole::Humanizer => {
            let m = env::var("GEMINI_HUMANIZER_MODEL")
                .unwrap_or_else(|_| DEFAULT_HUMANIZER_MODEL.to_string());
            ("google", m)
        }
        ModelRole::SuperExecutor => ("cli", String::new()),
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    /// Task decomposition — needs strong reasoning (Gemini 2.5 Pro).
    Planner,
    /// Code writing — needs speed + tool use (Gemini 2.5 Flash).
    Executor,
    /// Adversarial review — needs different perspective (Gemini 2.5 Pro, separate call).
    Skeptic,
    /// Style cleanup — needs speed, low intelligence (Gemini 2.5 Flash-Lite).
    Humanizer,
    /// Escalation for stuck tasks — Claude Code CLI.
    SuperExecutor,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AIModel {
    Gemini25Pro,
    Gemini25Flash,
    Gemini25FlashLite,
    OllamaLocal,
    ClaudeCli,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum Phase {
    Planning,
    Execution,
    Review,
    Audit,
    Recovery,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RouteTaskParams {
    /// The prompt to send to the routed model.
    pub prompt: String,
    /// Which role to route to (determines model selection).
    pub role: ModelRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RoutingDecision {
    pub model: AIModel,
    pub model_id: String,
    pub provider: String,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// ModelRouter
// ---------------------------------------------------------------------------

pub struct ModelRouter {
    pub client: Client,
    /// Conversation history: (role, prompt, response). Capped at MAX_HISTORY_ENTRIES.
    pub history: Arc<Mutex<Vec<(ModelRole, String, String)>>>,
}

impl ModelRouter {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| Client::new()),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Build a context summary from the last 3 history entries.
    fn build_context_prefix(&self) -> String {
        let guard = match self.history.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.is_empty() {
            return String::new();
        }
        let recent: Vec<_> = guard.iter().rev().take(3).collect();
        let mut prefix = String::from("[Conversation context]\n");
        for (role, prompt, response) in recent.iter().rev() {
            let role_label = match role {
                ModelRole::Planner => "planner",
                ModelRole::Executor => "executor",
                ModelRole::Skeptic => "skeptic",
                ModelRole::Humanizer => "humanizer",
                ModelRole::SuperExecutor => "super_executor",
            };
            // Truncate long entries to keep context compact
            let p_short = if prompt.len() > 200 {
                &prompt[..200]
            } else {
                prompt.as_str()
            };
            let r_short = if response.len() > 200 {
                &response[..200]
            } else {
                response.as_str()
            };
            prefix.push_str(&format!(
                "- [{}] Q: {} | A: {}\n",
                role_label, p_short, r_short
            ));
        }
        prefix.push_str("[End context]\n\n");
        prefix
    }

    /// Push an entry to history, capping at MAX_HISTORY_ENTRIES.
    fn push_history(&self, role: ModelRole, prompt: String, response: String) {
        let mut guard = match self.history.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.push((role, prompt, response));
        if guard.len() > MAX_HISTORY_ENTRIES {
            let excess = guard.len() - MAX_HISTORY_ENTRIES;
            guard.drain(..excess);
        }
    }

    /// Route and execute: pick the model for the role, call it, return the response.
    pub async fn call_model(&self, role: ModelRole, prompt: &str) -> Result<String, String> {
        let (provider, model_id) = model_for_role(&role);

        // Prepend conversation context from recent history
        let context_prefix = self.build_context_prefix();
        let augmented_prompt = if context_prefix.is_empty() {
            prompt.to_string()
        } else {
            format!("{}{}", context_prefix, prompt)
        };

        let result = match provider {
            "google" => {
                // Try Gemini first; fall back to Ollama if no API key
                match self.call_google(&augmented_prompt, &model_id).await {
                    Ok(response) => Ok(response),
                    Err(e) if e.contains("Missing GOOGLE_API_KEY") => {
                        // Fallback to local Ollama
                        let fallback = match role {
                            ModelRole::Planner => DEFAULT_OLLAMA_PLANNER,
                            ModelRole::Skeptic => DEFAULT_OLLAMA_SKEPTIC,
                            ModelRole::Humanizer => DEFAULT_OLLAMA_HUMANIZER,
                            _ => DEFAULT_OLLAMA_PLANNER,
                        };
                        tracing::warn!(
                            "GOOGLE_API_KEY not set, falling back to Ollama model: {}",
                            fallback
                        );
                        self.call_ollama(&augmented_prompt, fallback).await
                    }
                    Err(e) => Err(e),
                }
            }
            "cli" => self.call_cli_agent("claude", &augmented_prompt).await,
            _ => Err(format!("Unknown provider: {}", provider)),
        };

        // Record to history on success
        if let Ok(ref response) = result {
            self.push_history(role, prompt.to_string(), response.clone());
        }

        result
    }

    async fn call_google(&self, prompt: &str, model: &str) -> Result<String, String> {
        let key = env::var("GOOGLE_API_KEY").map_err(|_| "Missing GOOGLE_API_KEY".to_string())?;
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, key
        );
        let res = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "contents": [{"parts": [{"text": prompt}]}]
            }))
            .send()
            .await
            .map_err(|e| format!("Gemini API error: {}", e))?;

        let status = res.status();
        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            let err_msg = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            return Err(format!("Gemini {} error: {} — {}", model, status, err_msg));
        }

        json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                format!(
                    "Failed to parse Gemini response. Raw: {}",
                    serde_json::to_string(&json).unwrap_or_default()
                )
            })
    }

    async fn call_ollama(&self, prompt: &str, model: &str) -> Result<String, String> {
        let host = ollama_host();
        let url = format!("{}/api/generate", host);
        let res = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "model": model,
                "prompt": prompt,
                "stream": false
            }))
            .send()
            .await
            .map_err(|e| format!("Ollama error ({}): {}. Is Ollama running?", host, e))?;

        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        json["response"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Failed to parse Ollama response".to_string())
    }

    async fn call_cli_agent(&self, agent: &str, task: &str) -> Result<String, String> {
        let output = tokio::process::Command::new(agent)
            .args(["-p", task])
            .output()
            .await
            .map_err(|e| format!("CLI agent '{}' error: {}", agent, e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(format!("Agent '{}' failed: {}", agent, err))
        }
    }
}

// ---------------------------------------------------------------------------
// MCP tool
// ---------------------------------------------------------------------------

pub async fn route_task_impl<I, S>(
    _handler: &AgenticHandler<I, S>,
    params: RouteTaskParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let router = ModelRouter::new();
    let response = router
        .call_model(params.role, &params.prompt)
        .await
        .map_err(|e| McpError::internal_error(e, None))?;

    Ok(CallToolResult::success(vec![Content::text(response)]))
}
