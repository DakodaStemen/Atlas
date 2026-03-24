---
name: agent-tool-patterns
description: Agent architecture patterns covering tool calling, memory systems, safety guardrails, multi-agent orchestration, SDK patterns (TypeScript), agentic pipeline design, state/session management, and zero-waste context optimization. Use when building, configuring, or debugging AI agent systems.
domain: ai-engineering
tags: [agent, tool-calling, memory, guardrails, multi-agent, orchestration, pipeline, state-management, zero-waste]
triggers: agent, tool calling, agent memory, agent safety, guardrails, multi-agent orchestration, agentic pipeline, state management, session resume, zero-waste, breakdown plan, work queue
---

# Agent and Tool Patterns

## 1. Agentic Pipeline Design

### Pipeline Order

1. **RECALL**: `query_knowledge` with 2-5 keywords. Always first, no exceptions.
2. **TOOL-RAG**: `get_relevant_tools` to discover optimal tool subset before planning.
3. **RESEARCH**: `search_web` + `fetch_web_markdown` for APIs/libs/best practices.
4. **INVESTIGATION**: Codebase search, file reads. `get_file_history` before deleting code (Chesterton's fence).
5. **EXECUTION**: Implement changes. No TODOs, stubs, or placeholders.
6. **POST-WORK**: `refresh_file_index` → `scan_secrets` → `review_diff` → `log_training_row` → `commit_to_memory` → `verify_integrity` → `auto_approve_pattern`.
7. **ON FAILURE**: `analyze_error_log` → fix → retry up to 3x, then escalate.

### Loop Guard

- Track tool call count per turn. Alert when approaching limits (e.g., 20 calls without progress).
- Detect repeated identical tool calls (same tool, same args). Break after 3 repetitions.
- 5+ turns without success: Fresh Eyes — summarize error, restart clean.

## 2. Tool Calling Patterns

### Tool Design

- Each tool does one thing well. Clear name, description, and parameter schema.
- Return structured data (JSON), not natural language. Include error codes, not just error messages.
- Validate inputs before execution. Return partial results with status rather than failing silently.

### Error Handling

- Distinguish retriable errors (network timeout, rate limit) from permanent errors (invalid input, permission denied).
- Implement exponential backoff for retriable errors: 1s, 2s, 4s, max 3 retries.
- Log tool call context (args, timing, result status) for debugging.
- Cascade failures gracefully: if a tool fails, the agent should report what it knows rather than completely failing.

### Tool Selection

- Use `get_relevant_tools` to narrow from full tool catalog. Present only relevant tools to the LLM.
- Group related tools logically. Provide usage examples in tool descriptions.

## 3. Agent Memory Systems

### Short-Term Memory

- Conversation context within a session. Managed by the LLM's context window.
- Summarize long conversations to fit context limits. Preserve key decisions and user preferences.

### Long-Term Memory

- Persistent storage across sessions. Use `commit_to_memory` after significant learnings.
- Store: user preferences, project conventions, past decisions, error patterns.
- Retrieve via semantic search at session start.

### Working Memory

- Task-specific state during multi-step operations. Track: current step, intermediate results, pending actions.
- Use STATE.md or equivalent for persistence across context window boundaries.
- `/rag:pause` to save state, `/rag:resume` to restore.

### Memory Hygiene

- Run `scan_secrets` before `commit_to_memory` — never store secrets.
- Timestamp all memories. Set expiration for time-sensitive information.
- Periodically review and prune stale memories.

## 4. Agent Safety and Guardrails

### Input Guardrails

- Classify user intent before processing. Block or redirect harmful/out-of-scope requests.
- Validate tool arguments against schemas. Reject overly broad or destructive operations.

### Output Guardrails

- Scan responses for PII, secrets, or sensitive data before returning.
- Verify factual claims against retrieved context (grounding).
- Apply content filters for harmful or inappropriate content.

### Operational Guardrails

- No destructive SQL or bulk file delete without explicit user approval.
- No `cargo add` without `cargo search` first.
- No editing failing tests — fix source code only.
- No refactor of file >500 lines or function >50 lines without Breakdown Plan + approval.
- No TODO/FIXME/unimplemented!/stubs — deliver complete solutions.

### Rate Limiting and Cost Control

- Track token usage per session and per user. Set alerts on anomalous consumption.
- Implement circuit breakers for external API calls.
- Use cheaper models for classification/routing, expensive models for generation.

## 5. Multi-Agent Orchestration

### Patterns

- **Router**: Single agent classifies task and routes to specialist agents. Low overhead, clear responsibilities.
- **Pipeline**: Agents process sequentially (researcher → analyst → writer). Each agent's output is next agent's input.
- **Supervisor**: Coordinator agent delegates tasks and aggregates results. Can reassign on failure.
- **Debate**: Multiple agents propose solutions, then critique each other. Converge on best answer.

### Communication

- Pass structured data between agents, not natural language. Define clear input/output contracts.
- Use work queues for async orchestration. Track task status (pending, in-progress, done, failed).

### Evaluation Agents

- **Blind Comparator**: Compares two outputs without knowing which skill produced them. Prevents bias.
- **Grader**: Evaluates execution transcripts against expectations. Grades outputs AND critiques the evals.
- **Post-hoc Analyzer**: After comparison, explains WHY the winner won. Generates improvement suggestions.

## 6. Agent SDK Patterns (TypeScript)

### Basic Agent

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";
const result = await query({
  system: "You are a helpful assistant.",
  messages: [{ role: "user", content: userMessage }],
  tools: [/* tool definitions */],
  maxTokens: 4096
});
```

### Hooks

- **PreToolUse**: Validate/modify tool calls before execution. Block dangerous operations.
- **PostToolUse**: Process/filter tool results. Add context or transform output.
- **Stop**: Final validation before returning to user. Check completeness.
- **SessionStart**: Initialize state, load memory, set up environment.

### State Management

- Use `STATE.md` for persistent task state across sessions.
- Structure: current task, completed steps, pending actions, key decisions.
- Update state after each significant action. Resume from state on session start.

## 7. Breakdown Plan Pattern

### When Required

- File >500 lines needs refactoring.
- Function >50 lines needs restructuring.
- Cross-cutting change affects >3 files.

### Plan Structure

1. Current state analysis (what exists, what's wrong).
2. Target state (what it should look like).
3. Step-by-step migration plan with rollback points.
4. Risk assessment for each step.

### Execution

- Get user approval before starting. Store plan in `docs/BREAKDOWN_PLAN_*.md`.
- Execute one step at a time. Verify after each step. Checkpoint with git after each successful step.

## Checklist

- [ ] RECALL-first pipeline enforced on every turn
- [ ] Tool-RAG used before building multi-tool plans
- [ ] Loop guard configured (max calls, repetition detection)
- [ ] Tool error handling with retry and graceful degradation
- [ ] Memory system: short-term, long-term, working memory configured
- [ ] Secrets scanning before memory commits
- [ ] Safety guardrails: input validation, output scanning, operational limits
- [ ] Multi-agent contracts defined (input/output schemas)
- [ ] Breakdown plan required for large refactors
- [ ] State persistence for multi-session tasks
