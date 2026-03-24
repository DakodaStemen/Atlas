# Monolith MCP Tools — Full Reference

Every tool: what it does, when to use it, and what value it provides. This server exposes 31 tools via MCP; this doc is the single source of truth.

---

## Meta & tool discovery

### get_relevant_tools

**What it does:** Returns tool names most relevant to a natural-language query using semantic similarity over tool names and descriptions (Tool-RAG). Optional `top_k` (default 15). When the embedder is unavailable, returns all tools.

**When to use it:** When you want to limit context: call with the user message, then inject only these tool names into the prompt. Use for query-driven tool selection.

**Value:** Reduces noise and token use by suggesting only the tools that match the user’s intent. For a static decision table, use `get_tool_selection_guide` instead when it is available.

---

## RAG & code navigation

### query_knowledge

**What it does:** Semantic search over the indexed codebase: hierarchical (summaries then chunks), graph walk, and cross-encoder reranking. Optional: `reasoning=true` (fewer, focused chunks for you to answer from) or `execute=true` (returns RAG context for the IDE to synthesize the answer).

**When to use it:** “How does X work?”, “Where is Y?”, “Explain the auth flow”—any question that needs search over docs + code. Primary way to use RAG.

**Value:** Main way to answer “how/where” from the codebase without reading everything. If the index is stale, use `refresh_file_index` on the relevant files first. When the result is “No relevant information found in the index.”, do not invent an answer; ask the user for clarification.

### query_master_research

**What it does:** Search only the synthesized “Godly” master research document for technical details, architectural patterns, or domain logic.

**When to use it:** When you have produced a master research document and run `rag-mcp ingest-from-jsonl` so it is indexed (see [GODLY_RAG_WORKFLOW.md](../../docs/setup/GODLY_RAG_WORKFLOW.md)). Use for canonical research artifact lookup.

**Value:** Dedicated search over one canonical research doc; avoids mixing with general workspace RAG.

### get_related_code

**What it does:** Returns the code chunk(s) that **define** the given symbol plus chunks that **import/reference** it. Definition + call sites in one call.

**When to use it:** You have a symbol name (e.g. `PaymentService`, `handle_request`) and want its definition and who uses it.

**Value:** Exact navigation by symbol name; no free-text search. Use instead of `query_knowledge` when you already know the symbol.

### resolve_symbol

**What it does:** Returns the exact code block that **defines** the symbol (jump-to-definition). AST-based; no references.

**When to use it:** “Where is X defined?” when you only need the definition.

**Value:** Fast, precise definition lookup. If you need call sites too, use `get_related_code` instead.

---

## Build, test, and structure

### execute_shell_command

**What it does:** Runs a shell command in the project root. Only allowlisted programs: by default `cargo`, `git`, `grep`, `ls`, `npm`; set `EXECUTE_SHELL_ALLOWLIST` (comma-separated) to extend. Rejects redirects to paths outside the workspace. Blocks destructive or arbitrary commands.

**When to use it:** “Run the tests,” “run the build,” “git status,” “grep for X,” list dirs, run npm scripts.

**Value:** Safe way for the agent to run terminal commands. For “did the build pass?” use `verify_integrity` instead. For long-running or interactive commands (e.g. `npm run dev`, `cargo run`) use `fork_terminal` so the user sees live output without blocking MCP.

### verify_integrity

**What it does:** Runs `cargo check`, `cargo test`, and `cargo clippy` in one go and returns a structured pass/fail payload (no parsing of raw cargo output).

**When to use it:** After code changes: “Did I break anything?” Single gatekeeper for “is the repo green?”

**Value:** One call instead of multiple `execute_shell_command`; consistent JSON result for the agent to act on.

### verify_module_tree

**What it does:** Checks that every Rust module under `src/` is reachable from `lib.rs` or `main.rs` (no orphaned `mod`). Reports phantom modules if any.

**When to use it:** Right after adding a new `.rs` file under `src/`.

**Value:** Catches “I added a file but forgot `mod x`” before it causes confusing errors.

### read_manifest

**What it does:** Reads `Cargo.toml` and returns dependency and dev-dependency crate names and versions.

**When to use it:** Before suggesting code that uses an external crate, or before adding a dependency.

**Value:** Avoids version drift (e.g. suggesting `serde` 2.0 when the project uses 1.0). Prefer over RAG for “what crates/versions are in use?”

**When to use it:** Before a refactor: “What depends on module X?” or when you need a topology view.

### module_graph

**What it does:** Rust module structure: text (cargo-modules) or Mermaid diagram. Params: workspace_path (optional), format: "text" or "mermaid" (default "mermaid").

**When to use it:** Before a refactor or for topology view. Use format "text" for cargo-modules output; "mermaid" for the diagram.

**Value:** Single tool for both structure views; use for refactors and onboarding.

---

## Memory and learning

### commit_to_memory

**What it does:** Appends a timestamped lesson to `docs/lessons_learned.md` and triggers a background RAG re-index so future answers can use it. Writes to the first allowed root’s `docs/`.

**When to use it:** After fixing a non-obvious bug or making an architectural decision you want the agent to remember (e.g. “we tried X and it failed because Y”).

**Value:** Builds institutional memory; reduces repeated mistakes. Use consistently for high payoff.

### log_training_row

**What it does:** Appends one row to `training.jsonl` for the Ouroboros/fine-tuning pipeline. Use the exact task line as query (e.g. [DOCS] Document X). Server may skip low-value rows.

**When to use it:** After a successful non-trivial response when you have a pipeline that consumes `training.jsonl`.

**Value:** Feeds the training flywheel; irrelevant if you don’t use that pipeline.

### approve_pattern

**What it does:** Saves a code pattern to the golden set (e.g. `docs/golden_set.md`) and logs it as a high-quality example for training.

**When to use it:** When you’ve written a pattern (error handling, API usage, etc.) that should be the standard.

**Value:** Documents “this is the right way”; most useful if you use the golden set or a training pipeline.

### auto_approve_pattern

**What it does:** Automatically proposes a code pattern for the golden set using pattern recognition and similarity to already approved patterns. If it matches or is a reasonable first pattern (e.g. 100–4000 chars), adds to golden set and training.

**When to use it:** After `verify_integrity` passes and you want the agent to suggest adding the change as a pattern. Use sparingly.

**Value:** Reduces manual approval when you trust the heuristic; prefer manual `approve_pattern` for critical patterns.

### save_rule_to_memory

**What it does:** Appends a rule or guideline to `docs/agent_rules.md` and triggers RAG re-index so `query_knowledge` (RECALL) can retrieve it. Same docs root as `commit_to_memory` (first allowed root).

**When to use it:** When you want to persist high-signal rules the agent should follow (e.g. “always use std::env::var() for secrets”).

**Value:** Keeps rules in RAG without a separate vault; complements `commit_to_memory` for lessons.

---

## Index and refresh

### refresh_file_index

**What it does:** Re-ingests the given file paths into the RAG index (parse, chunk, embed). Paths must be under ALLOWED_ROOTS.

**When to use it:** After editing files so the agent’s memory is up to date. Call once per turn with the list of modified paths; skip if no changes.

**Value:** Keeps RAG in sync with the codebase without a full re-ingest.

---

## Security and history

### security_audit

**What it does:** Runs Semgrep security scan on a path. Returns deterministic findings. Requires Semgrep CLI in PATH.

**When to use it:** When the LLM must know if code is safe (e.g. before recommending a dependency or pattern).

**Value:** Automated security check; use with `scan_secrets` before commits.

### scan_secrets

**What it does:** Scans the workspace for likely hardcoded secrets (e.g. key/token/secret/password assignments to long strings, `sk_` prefixes, `AIza`). Returns a JSON list of path, line, snippet.

**When to use it:** Before committing or before `commit_to_memory`. When the user says “commit” or “save to memory.”

**Value:** Catches accidental secret commits; use env vars / secret managers instead of hardcoding.

### get_file_history

**What it does:** Returns per-line git blame or recent log for a file under ALLOWED_ROOTS so you can see why a line exists.

**When to use it:** Before deleting or refactoring code when the change is risky or the code looks odd (“Chesterton’s fence”).

**Value:** Avoids removing code that was added for a good reason; supports safe refactors.

---

## Task queue

### submit_task

**What it does:** Enqueues an async task to `_tasks/inbox` under the first allowed root. Task types: `research`, `ingest`, `refresh_file_index`, `verify-integrity`. Payload is type-specific JSON (e.g. query, path, paths, workspace_path). Writes a JSON file; the task runner (external) processes inbox and writes results to `_tasks/outbox`.

**When to use it:** When you want work done by a background task runner (e.g. research, full ingest, verify-integrity) without blocking the IDE.

**Value:** Typed, validated task contract; easy to automate. Extensible via `TASK_TYPES_EXTRA` or task manifest when configured.

---

## Web and RAG growth

### project_packer

**What it does:** Generates a compressed tree view of the project and reads key configs (Cargo.toml, package.json). Gives the model a mental map without wasting tokens on full directory listing.

**When to use it:** At the start of a session or when the agent needs a quick layout of the codebase.

**Value:** Fast onboarding and context; use before deep RAG when structure matters.

### fetch_web_markdown

**What it does:** Fetches a URL and returns a sanitized Markdown version of the page. No JavaScript execution. SSRF-safe.

**When to use it:** When you need the content of a doc page, API reference, or blog as markdown.

**Value:** Brings external docs into the conversation. For long-term use, follow with `ingest_web_context` so it’s in RAG.

### search_web

**What it does:** Server-side web search. Returns a list of URLs (and titles) for the given topic. Uses Tavily if `TAVILY_API_KEY` is set, otherwise Serper if `SERPER_API_KEY` is set. If neither is set, returns a message asking to set one.

**When to use it:** When you want the server to find URLs for a topic (e.g. “MCP protocol spec”) so you can pass them to `research_and_verify(topic, urls)` for compare-and-ingest. Optional: reduces reliance on the IDE’s web search.

**Value:** One-call URL discovery; pair with `research_and_verify` for the full research flow. Params: `topic` (required), `limit` (optional, default 5).

### ingest_web_context

**What it does:** Persists web content into RAG. You pass snippets (url + content); they’re stored so future `query_knowledge` can retrieve them. No API key required.

**When to use it:** After `fetch_web_markdown` (or when you have content from elsewhere) when that content should become part of project knowledge.

**Value:** Grows the RAG index from the web; makes that content answerable via `query_knowledge` later.

### research_and_verify

**What it does:** Lookup RAG for a topic, then fetch the given URLs and ingest them into RAG. Pass topic and 1–3 high-signal URLs.

**When to use it:** After web search when you want to verify and optionally ingest new content. Phase 1.5 URL-based research flow.

**Value:** Reduces duplicate or low-value ingest; keeps RAG quality high.

---

## System and model

### get_system_status

**What it does:** Returns CPU load, RAM (total/used), and optionally GPU VRAM and utilization via `nvidia-smi`. Emits a critical warning if VRAM > 90%.

**When to use it:** Before heavy or complex commands (e.g. large build) to avoid overloading the machine.

**Value:** Resource awareness; helps avoid running big jobs when the system is already under load.

---

## UI and design

### get_ui_blueprint

**What it does:** Returns a pre-vetted Tailwind/React layout snippet per DESIGN_AXIOMS (e.g. 3-column horizontal on desktop, responsive collapse). `blueprint_type`: dashboard, form, or settings.

**When to use it:** When generating dashboard or multi-column UI and you want a consistent, approved layout.

**Value:** Avoids ad-hoc layouts that violate design rules; use with `verify_ui_integrity` before completion.

### verify_ui_integrity

**What it does:** Linter for design: checks a UI snippet against DESIGN_AXIOMS. Flags stacking red flags (e.g. w-full without flex/grid), forbidden shadows (shadow-md+), and suggests refactor.

**When to use it:** Before marking UI work complete. All UI code should pass this check.

**Value:** Enforces design consistency; use with `get_ui_blueprint` for layout and `get_design_tokens` (when available) for tokens.

---

## Error and diff workflow

### analyze_error_log

**What it does:** Analyzes error output using RAG context and `lessons_learned`; suggests root cause and a concrete fix. Accepts `error_output` and optional `recent_errors` for recurring issues.

**When to use it:** When the error is non-obvious or recurring. For simple “unknown type” errors, the compiler is often enough.

**Value:** Connects errors to past lessons and codebase context; good for subtle or recurring failures.

### scaffold_reproduction_test

**What it does:** TDD helper: returns context and instructions to scaffold a test that reproduces a bug. Accepts `bug_description` and optional `error_output`.

**When to use it:** Before fixing a logic bug when you want a failing test first.

**Value:** Ensures a regression test exists before changing behavior; supports TDD.

### review_diff

**What it does:** Sends a code diff (unified diff or changed code) to the local LLM for review: security, `unwrap()`, and bad practices. Returns the review; you then approve or request changes.

**When to use it:** Before committing, as an extra check. Not a replacement for human review.

**Value:** Automated second pass on safety and style before commit.

---

## New tools (from optimization plan)

*The following tools are added by the optimization plan; entries will be updated when implemented.*

### get_tool_selection_guide

**What it does:** Returns the static situation→tool table (and “Don’t” / “Core vs niche” sections) as text. No arguments. Reads from `TOOL_SELECTION_GUIDE_PATH` or `docs/TOOL_SELECTION_GUIDE.md` under the first allowed root.

**When to use it:** When you’re unsure which tool matches the user’s request. Call it first, then pick the tool from the table. Use `get_relevant_tools` for query-driven selection.

**Value:** Avoids guessing among many tools; one call gives the full decision guide. Complements `get_relevant_tools`.

### get_design_tokens

**What it does:** Reads design tokens (e.g. colors, typography, component specs) from a configurable path (`DESIGN_TOKENS_DIR` or default `docs/design/data`) and returns the content for the given `token_category` (e.g. "colors", "typography"). Categories map to filenames: the tool looks for `{category}.csv` and `{category}.json`, and also tries title-case (e.g. `Colors.csv`) for case-sensitive filesystems. Optional `base_path` to override.

**When to use it:** When you need structured design system data (colors, typography, spacing) for UI work. Complements `get_ui_blueprint` and `verify_ui_integrity`.

**Value:** Design-system-driven UIs without hardcoding tokens; single source of truth for tokens when the path is configured.

### fork_terminal

**What it does:** Runs a command in a **new** terminal/process using the same allowlist as `execute_shell_command` (default: cargo, git, grep, ls, npm; extend via `EXECUTE_SHELL_ALLOWLIST`). Does not block MCP stdio. Optional `working_dir` and `title` (e.g. for window title on Windows).

**When to use it:** For long-running or interactive commands (e.g. `npm run dev`, `cargo run`) so the user sees live output in a separate window.

**Value:** Keeps the IDE responsive; use instead of `execute_shell_command` when the command is long-running or interactive.

---

### compile_rules

**What it does:** Merges global rules (from RULES_VAULT or GLOBAL_RULES_DIR: Standards, Rules, Workflows markdown) and the active project’s `.context/` into a single document and writes it to `active_project_path/.cursorrules`, and optionally to `GEMINI.md` and `CLAUDE.md` in the project root. Rust-only; no Python.

**When to use it:** When RULES_VAULT or GLOBAL_RULES_DIR is set and the user wants to sync rules into a project (e.g. after vault rule updates or when switching projects).

**Value:** One source of truth for global rules; consistent injection across IDEs without running a separate Synapse script.

---

*End of reference. See also: TOOL_SELECTION_GUIDE.md, IMPLEMENTATION_PLAN_OPTIMIZATIONS.md, OPERATIONAL_RUNBOOK.md.*
