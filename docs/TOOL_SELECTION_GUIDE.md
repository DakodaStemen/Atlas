# Tool Selection Guide — Monolith MCP

When you’re unsure which tool matches the user’s request, **call this guide first** (or use `get_tool_selection_guide` / `get_relevant_tools`). Do not guess among 31+ tools.

---

## Don’t

- **Don’t guess.** If the intent is ambiguous, call `get_tool_selection_guide` (returns this doc) or `get_relevant_tools` (query-based top-k) and pick from the result.
- **Don’t use `execute_shell_command` for “did the build pass?”** — use `verify_integrity` instead.
- **Don’t use `query_knowledge` when you already have a symbol name** — use `get_related_code` or `resolve_symbol`.
- **Don’t invent answers when RAG returns “No relevant information found in the index.”** Ask the user for clarification or suggest indexing.
- **Don’t run long-running or interactive commands (e.g. `npm run dev`) with `execute_shell_command`** — use `fork_terminal` so the user sees live output and MCP doesn’t block.
- **Don’t commit or call `commit_to_memory` without running `scan_secrets` first.**

---

## Core vs niche

**Core (use often):**  
query_knowledge, get_related_code, resolve_symbol, execute_shell_command, verify_integrity, refresh_file_index, commit_to_memory, project_packer, read_manifest, get_system_status, scan_secrets, fetch_web_markdown, ingest_web_context, analyze_error_log, review_diff.

**Niche (use when the situation matches):**  
query_master_research, save_rule_to_memory, submit_task, get_relevant_tools, get_tool_selection_guide, research_and_verify, get_ui_blueprint, verify_ui_integrity, get_design_tokens, fork_terminal, module_graph, verify_module_tree, security_audit, get_file_history, log_training_row, approve_pattern, auto_approve_pattern, scaffold_reproduction_test, compile_rules.

---

## Situation → Tool

| User intent / situation | Tool(s) to use |
|-------------------------|----------------|
| “How does X work?” / “Where is Y?” / explain something in the codebase | query_knowledge |
| Search only the synthesized master research doc | query_master_research |
| I have a symbol name; I want its definition | resolve_symbol |
| I have a symbol name; I want definition + who uses it | get_related_code |
| Run tests, build, git status, grep, npm script | execute_shell_command |
| Long-running or interactive command (dev server, cargo run) | fork_terminal |
| “Did the build/tests pass?” after code change | verify_integrity |
| Just added a new .rs file under src/ | verify_module_tree |
| “What crates/versions does this project use?” | read_manifest |
| Refactor / module structure (text or Mermaid) | module_graph |
| Get a mental map of the project | project_packer |
| Remember a lesson or architectural decision | commit_to_memory |
| Persist a rule the agent should follow | save_rule_to_memory |
| Add to golden set / training (manual) | approve_pattern |
| Suggest adding last change as pattern (after verify_integrity) | auto_approve_pattern |
| Log a training row for Ouroboros | log_training_row |
| I edited files; keep RAG in sync | refresh_file_index |
| Enqueue background task (research, ingest, verify-integrity) | submit_task |
| “Which tools should I use for this request?” | get_tool_selection_guide or get_relevant_tools |
| Fetch a URL as Markdown | fetch_web_markdown |
| Save web content into RAG | ingest_web_context |
| Research topic + verify URLs then maybe ingest | research_and_verify |
| Check CPU/RAM/GPU before heavy work | get_system_status |
| Need a dashboard/form/settings layout snippet | get_ui_blueprint |
| Check UI snippet against design rules | verify_ui_integrity |
| Get design tokens (colors, typography, etc.) | get_design_tokens |
| Analyze error output / recurring errors | analyze_error_log |
| Write a failing test before fixing a bug | scaffold_reproduction_test |
| Audit diff before commit | review_diff |
| Security scan (Semgrep) on path | security_audit |
| Check for hardcoded secrets | scan_secrets |
| Why does this line exist? (before refactor/delete) | get_file_history |
| Sync global rules + project context to .cursorrules | compile_rules (when RULES_VAULT / GLOBAL_RULES_DIR set) |
| Need a step-by-step procedure or skill (MCP quality, pipeline, security) | list_skill_metadata then get_skill_content(skill_id) |

---

## Token savings (symbol-first, like jCodeMunch)

Same idea as [jCodeMunch / “MCP Tools That Cut Claude Token Costs by 99%”](https://www.youtube.com/watch?v=vzCy44o3JwA): **index once, retrieve only what’s needed** instead of whole files.

| Goal | Use this | Avoid this |
|------|----------|------------|
| Definition of a known symbol | **resolve_symbol**(symbol_name) | Reading the whole file or query_knowledge |
| Definition + who uses it | **get_related_code**(symbol_name) | Reading multiple full files |
| One section of a long doc | **get_doc_outline** then **get_section**(section_id) | Pulling the full document |
| Answer from RAG when you’ll synthesize | **query_knowledge**(…, reasoning=true) | execute=true when you don’t need the server to answer |

Result: hundreds of tokens per symbol lookup instead of thousands per file. Keep **refresh_file_index** after edits so symbol index stays accurate.

---

## Context rot (Chase AI: “The Secret Poison…”)

[Chase AI: The Secret Poison Killing Your Claude Code Performance](https://www.youtube.com/watch?v=-xHprsdG4ME) and the [Chroma context-rot study](https://research.trychroma.com/context-rot) show: **the more the context window is filled, the worse the model performs** (roughly past ~100k–120k tokens). It’s not just cost—it’s quality.

| Mitigation | What to do |
|------------|------------|
| **Task size** | One session = discrete, small tasks. Use a PRD and break work into atomic steps (Ralph loop / GSD style). |
| **Session management** | Long chats: ask for a summary, start a new chat/session with that summary so the window is fresh. Use autocompact (e.g. Claude Code) if available. |
| **Fewer tokens per turn** | Symbol-first (above): resolve_symbol / get_related_code / get_section so each turn sends less. Less in window = less rot. |
| **MCPs** | Anthropic notes MCP tool definitions are heavy. Enable only the MCPs you need for the current task; don’t leave everything on. Use get_relevant_tools when you need guidance. |

Same goal as token savings: keep context small and focused so the model stays effective.

---

*Full details: [MCP_Tools_Reference.md](MCP_Tools_Reference.md).*
