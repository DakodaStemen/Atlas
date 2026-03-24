//! Prompt-building and RAG-backed analysis tools: analyze_error_log, scaffold_reproduction_test, review_diff.
//!
//! **Prompt size (typical inputs):** For ~500-char error_output and ~200-char bug_description,
//! build_analyze_error_log_text and build_scaffold_reproduction_test_text produce prompts that
//! depend on RAG context (often 5–15k chars). review_diff user_content is ~100 chars + diff length.
//! All three tool responses are truncated by truncate_for_budget(..., read_rag_max_response_chars())
//! before return (see TOKEN_OPTIMAL_CHECKLIST caps).

use super::super::{
    read_rag_max_response_chars, truncate_for_budget, truncate_rag_response, AgenticHandler,
    IngestionProvider, VectorStoreProvider, MAX_EXTRA,
};
use super::{
    AnalyzeErrorLogParams, ReviewDiffParams, ScaffoldReproductionTestParams, SkepticReviewParams,
};
use crate::rag::store::format_sandbox_response;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;

/// Shared RAG context for analysis prompts: hierarchical_search → rerank → mmr → truncate. Reduces duplication between analyze_error_log and scaffold_reproduction_test.
fn rag_context_for_prompt<I, S>(
    handler: &AgenticHandler<I, S>,
    primary_query: &str,
    extra_queries: &[(&str, usize)],
) -> Result<String, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let mut rows = handler
        .store
        .hierarchical_search(primary_query, handler.store.rerank_candidates, MAX_EXTRA)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    rows = handler
        .store
        .rerank_results(primary_query, rows, handler.store.rerank_top_k);
    rows = handler.store.mmr_rerank(rows, handler.store.rerank_top_k);
    rows.truncate(handler.store.rerank_top_k);
    let mut seen = std::collections::HashSet::new();
    for r in &rows {
        seen.insert(r.id.clone());
    }
    for (query, max_k) in extra_queries {
        let extra_rows = handler
            .store
            .hierarchical_search(query, handler.store.rerank_candidates, MAX_EXTRA)
            .ok()
            .unwrap_or_default();
        let reranked =
            handler
                .store
                .rerank_results(query, extra_rows, handler.store.rerank_top_k.min(*max_k));
        for r in reranked {
            if seen.insert(r.id.clone()) {
                rows.push(r);
            }
        }
    }
    let context = truncate_rag_response(
        &handler.store,
        &format_sandbox_response(&rows, &handler.store.allowed_roots),
    );
    Ok(context)
}

/// Builds the analyze_error_log prompt: RAG context + error output + optional recent_errors. Returns single message text.
pub fn build_analyze_error_log_text<I, S>(
    handler: &AgenticHandler<I, S>,
    error_output: &str,
    recent_errors: Option<&str>,
) -> Result<String, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let error_output = error_output.trim();
    let context = rag_context_for_prompt(
        handler,
        error_output,
        &[("lessons_learned debugging error fix", 5)],
    )?;
    let recent_errors_section = recent_errors
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            format!(
                "\n\n<recent_errors>\nRecurring error (e.g. after revert). Do NOT suggest the same fix again.\n{}\n</recent_errors>",
                s
            )
        })
        .unwrap_or_default();
    Ok(format!(
        r#"You are an expert debugger. Use the retrieved context and past lessons.

1. Identify root cause. 2. If similar error in lessons_learned, apply that fix. 3. Suggest concrete code change. 4. If context shows this error was recently "fixed" then reverted, suggest a different fix or human intervention.

<retrieved_context>
{}

</retrieved_context>

<error_output>
{}
</error_output>{}"#,
        context, error_output, recent_errors_section
    ))
}

/// Builds the scaffold_reproduction_test prompt: RAG context + bug description. Returns single message text.
pub fn build_scaffold_reproduction_test_text<I, S>(
    handler: &AgenticHandler<I, S>,
    bug_description: &str,
    error_output: &str,
) -> Result<String, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let bug_description = bug_description.trim();
    let error_output = error_output.trim();
    let query = if error_output.is_empty() {
        bug_description.to_string()
    } else {
        format!("{} {}", bug_description, error_output)
    };
    let context = rag_context_for_prompt(
        handler,
        &query,
        &[("unit test example #[test] test case", 5)],
    )?;
    let error_section = if error_output.is_empty() {
        String::new()
    } else {
        format!("<error_output>\n{}\n</error_output>\n", error_output)
    };
    Ok(format!(
        r#"RULE: Before fixing this logic bug you MUST write a minimal test that FAILS (reproduces the bug). Do not modify production code until the test is in place and fails.

<bug_description>
{}
</bug_description>
{}
<relevant_context>
{}

</relevant_context>

Steps: 1) Write a test that reproduces the bug. 2) Run the test and confirm it fails. 3) Then fix the code so the test passes."#,
        bug_description,
        error_section,
        context
    )
    .trim()
    .to_string())
}

/// Shared logic for review_diff prompt/tool. Returns (user_content, audit_text_opt).
/// When mode == Some("short"), uses a minimal security-only prompt.
pub async fn build_review_diff_audit<I, S>(
    _handler: &AgenticHandler<I, S>,
    diff: &str,
    mode: Option<&str>,
) -> Result<(String, Option<String>), McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let diff = diff.trim();
    let user_content = if mode == Some("short") {
        format!(
            r#"Audit this diff for security issues and unwrap()/expect(). Reply APPROVE or REQUEST_CHANGES and a brief list of issues.

<diff>
{}
</diff>"#,
            diff
        )
    } else {
        format!(
            r#"Audit the following code diff. Check for: 1) Security issues. 2) Use of unwrap() or expect() that could panic. 3) Bad practices (e.g. ignoring errors, hardcoded secrets). Reply with a short verdict (APPROVE / REQUEST_CHANGES) and a bullet list of issues, or "No issues found" if clean.

<diff>
{}
</diff>"#,
            diff
        )
    };
    Ok((user_content, None))
}

pub async fn analyze_error_log_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: AnalyzeErrorLogParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let raw = build_analyze_error_log_text(
        handler,
        &params.error_output,
        params.recent_errors.as_deref(),
    )?;
    let text = truncate_for_budget(&raw, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn scaffold_reproduction_test_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ScaffoldReproductionTestParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let raw = build_scaffold_reproduction_test_text(
        handler,
        &params.bug_description,
        params.error_output.as_deref().unwrap_or(""),
    )?;
    let text = truncate_for_budget(&raw, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

pub async fn review_diff_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: ReviewDiffParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let (user_content, audit_opt) =
        build_review_diff_audit(handler, &params.diff, params.mode.as_deref()).await?;
    let raw = if let Some(audit) = audit_opt {
        format!("{}\n\n--- Audit ---\n{}", user_content, audit)
    } else {
        user_content
    };
    let text = truncate_for_budget(&raw, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

/// Enhanced skeptic review: combines review_diff with RAG-backed pattern matching
/// and lessons learned to produce a confidence-scored critique.
pub async fn skeptic_review_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: SkepticReviewParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let diff = params.diff.trim();
    let objective = params.objective.as_deref().unwrap_or("").trim();

    if diff.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "skeptic_review requires a non-empty diff.",
        )]));
    }

    // Search for relevant anti-patterns and lessons learned
    let query = if objective.is_empty() {
        format!(
            "code review security anti-pattern {}",
            &diff[..diff.len().min(200)]
        )
    } else {
        format!("{} code review anti-pattern lessons_learned", objective)
    };

    let context = rag_context_for_prompt(
        handler,
        &query,
        &[
            ("lessons_learned bug fix regression", 5),
            ("golden_set approved pattern", 3),
        ],
    )?;

    let objective_section = if objective.is_empty() {
        String::new()
    } else {
        format!("\n\n<objective>\n{}\n</objective>", objective)
    };

    let prompt = format!(
        r#"You are a skeptical senior engineer performing an adversarial code review.
Your job is to find flaws, incorrect assumptions, edge cases, and risks.
ASSUME THE SOLUTION IS WRONG UNTIL PROVEN OTHERWISE.

Review this diff and produce a structured critique:{objective_section}

<diff>
{diff}
</diff>

<known_patterns_and_lessons>
{context}
</known_patterns_and_lessons>

Respond with a JSON object:
{{
  "issues": [
    {{
      "severity": "critical|high|medium|low",
      "category": "security|logic|performance|style|assumption",
      "description": "what is wrong",
      "suggestion": "how to fix it"
    }}
  ],
  "risk_level": "low|medium|high|critical",
  "confidence": 0.0-1.0,
  "requires_retry": true/false,
  "summary": "one-line verdict"
}}"#,
    );

    let text = truncate_for_budget(&prompt, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}
