//! Implementation of plan_task: RAG-backed task decomposition.

use super::PlanTaskParams;
use crate::rag::handler::{
    read_rag_max_response_chars, truncate_for_budget, truncate_rag_response, AgenticHandler,
    IngestionProvider, VectorStoreProvider, MAX_EXTRA,
};
use crate::rag::store::format_sandbox_response;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;

/// Build a plan by searching RAG for relevant context, then producing a structured plan.
pub async fn plan_task_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    params: PlanTaskParams,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let objective = params.objective.trim();
    if objective.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "plan_task requires a non-empty objective.",
        )]));
    }

    let max_steps = params.max_steps.unwrap_or(10).min(20);
    let constraints = params.constraints.unwrap_or_default();

    // Search RAG for relevant context about the objective
    let mut rows = handler
        .store
        .hierarchical_search(objective, handler.store.rerank_candidates, MAX_EXTRA)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    rows = handler
        .store
        .rerank_results(objective, rows, handler.store.rerank_top_k);
    rows = handler.store.mmr_rerank(rows, handler.store.rerank_top_k);
    rows.truncate(handler.store.rerank_top_k);

    // Also search for lessons learned related to this objective
    let extra_rows = handler
        .store
        .hierarchical_search(
            &format!("{} lessons_learned best practice", objective),
            handler.store.rerank_candidates,
            MAX_EXTRA,
        )
        .ok()
        .unwrap_or_default();
    let reranked = handler
        .store
        .rerank_results(objective, extra_rows, 5);
    let mut seen: std::collections::HashSet<String> =
        rows.iter().map(|r| r.id.clone()).collect();
    for r in reranked {
        if seen.insert(r.id.clone()) {
            rows.push(r);
        }
    }

    let context = truncate_rag_response(
        &handler.store,
        &format_sandbox_response(&rows, &handler.store.allowed_roots),
    );

    let constraints_section = if constraints.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n<constraints>\n{}\n</constraints>",
            constraints
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{}. {}", i + 1, c))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let prompt = format!(
        r#"You are a planning engine. Decompose the following objective into a concrete execution plan.

<objective>
{objective}
</objective>{constraints_section}

<retrieved_context>
{context}
</retrieved_context>

Rules:
1. Maximum {max_steps} steps.
2. Each step must have a clear action, an optional tool name, and a testable success criterion.
3. Order steps by dependency (prerequisites first).
4. Include constraints that apply across all steps.
5. Estimate complexity as "low" (1-3 steps), "medium" (4-7 steps), or "high" (8+ steps).
6. Be concrete — reference specific files, functions, or tools from the context when possible.
7. Do NOT include steps for "verify" or "test" unless the objective explicitly requires new tests.

Respond with a JSON object matching this schema:
{{
  "objective": "...",
  "steps": [{{ "step": 1, "action": "...", "tool": "optional_tool_name", "success_criterion": "..." }}],
  "constraints": ["..."],
  "success_criteria": ["..."],
  "complexity": "low|medium|high"
}}"#,
    );

    let text = truncate_for_budget(&prompt, read_rag_max_response_chars());
    Ok(CallToolResult::success(vec![Content::text(text)]))
}
