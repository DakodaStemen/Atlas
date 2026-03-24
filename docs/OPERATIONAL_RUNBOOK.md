# Operational Runbook: rag-mcp (Monolith)

Scannable guide for operating the `rag-mcp` server in production or day-to-day use.

---

## 0. Consistency checklist (run perfectly, every time)

Use this before relying on Cursor + pipeline + tri-model so everything runs the same way every time.

| Check | Why |
|-------|-----|
| **One MCP entry only** | Cursor MCP config has exactly one `monolith` entry (no duplicate key). Duplicates double the backend and bloat the tool list. |
| **Absolute paths in MCP env** | Set `DATA_DIR` and `ALLOWED_ROOTS` to **absolute** paths in the MCP server `env`. Cursor spawns the server with an undefined or workspace-dependent cwd; relative paths (e.g. `./data`) can point to the wrong place. |
| **ORT_DYLIB_PATH set** | Required for semantic search (Nomic embeddings). If unset, search is FTS-only; if the DB was built with embeddings and you later run without ORT, the server can exit with "embedding dimension mismatch". Use the same env for ingest and serve. |
| **RAG index exists** | Run `rag-mcp ingest <path>` at least once (path under ALLOWED_ROOTS). Then `rag-mcp verify-retrieval` to confirm retrieval returns chunks. |
| **Release build** | `cargo build --release` from `monolith/`; point Cursor at `target/release/rag-mcp.exe` (full path). |
| **Rule points at one file** | Cursor "Rules for AI" / project rules use **only** `.cursor/rules/agentic-operator.mdc`. No extra always-applied rules. |
| **Tri-model: CLIs in PATH** | To run `scripts/orchestrator/run_orchestrator.ps1`, ensure `claude` and `gemini` are in PATH (Anthropic CLI, Google Gemini CLI). Otherwise CODE/SCAN/TEST and AUDIT/DOCS/RESEARCH lanes fail. |
| **Queue file present** | `.cursor/work_queue.md` exists when using the orchestrator or Ouroboros. First three lines control run limit when using "run N" style. |
| **security_audit (optional)** | If the agent uses `security_audit`, Semgrep CLI must be in PATH. |
| **Kill orphans before rebuild** | Before `cargo build --release`, run `taskkill /F /IM rag-mcp.exe` (or `scripts/ops/kill_orphan_mcp.ps1 -Force` from repo root) so the new binary is used when Cursor reconnects. |

**Optional for consistency:** `MCP_AUDIT_LOG_PATH` (e.g. `monolith/data/mcp_audit.jsonl`) to log every tool call; rotate the file yourself. For read-only or auditor-only use, `MCP_READ_ONLY=true` prevents memory/index writes.

---

## 1. Quick Start

Three steps to get the server running:

1. **Set environment variables** (minimum: ensure `ALLOWED_ROOTS` includes your repo; for semantic search set `ORT_DYLIB_PATH`). See [Environment Variables](#2-environment-variables).
2. **Run ingest** so the RAG index exists:  
   `rag-mcp ingest <path>`  
   Use a path under your allowed roots (e.g. current directory or repo root). Run `rag-mcp audit` first if this is a fresh install.
3. **Run serve** (default command):  
   `rag-mcp serve`  
   The server uses stdio; Cursor (or another MCP client) spawns it. Do not run it in a separate terminal and expect the client to connect—the client must start the process.

**First-time check:** After ingest, run `rag-mcp verify-retrieval` to confirm the same retrieval path used by `query_knowledge` returns non-empty chunks.

---

## 2. Environment Variables

Single reference for all env vars. Defaults and examples are from the running code; "Optional" means the server works without it.

| Variable | Purpose | Default / example | Optional |
|----------|---------|-------------------|----------|
| **ALLOWED_ROOTS** | Comma-separated paths for ingest, `refresh_file_index`, `get_file_history`. | cwd; on Windows `%USERPROFILE%\Desktop\Work` added. Example: `C:\MCP,D:\Other` | Yes |
| **DATA_DIR** | Local data directory. RAG DB: `DATA_DIR/rag.db`; manifest: `DATA_DIR/rag_manifest.json`. | `./data` when not using vault | Yes |
| **VAULT_DIR** / **HOLLOW_VAULT** | When set, RAG DB and training live here (vault mode). | (none) | Yes |
| **ORT_DYLIB_PATH** | Path to ONNX Runtime DLL/shared library. Enables Nomic embeddings; unset = FTS-only. | (none) | Yes (FTS-only without it) |
| **PRISM_ROOT** | Single workspace root when vault set or for compatibility; used if `ALLOWED_ROOTS` unset. | (none) | Yes |
| **GOLDEN_SET_DIR** | When set, `auto_approve_pattern` and training write here. | (none) | Yes |
| **TOOL_SELECTION_GUIDE_PATH** | Path for `get_tool_selection_guide`; else first allowed root `docs/TOOL_SELECTION_GUIDE.md`. | (none) | Yes |
| **DESIGN_TOKENS_DIR** | Dir for `get_design_tokens`; else first allowed root `docs/design/data`. | (none) | Yes |
| **RULES_VAULT** / **GLOBAL_RULES_DIR** | When set, `compile_rules` reads global rules from this dir. | (none) | Yes |
| **TASK_TYPES_EXTRA** | Comma-separated extra task types for `submit_task`. | (none) | Yes |
| **JANITOR_INTERVAL_HOURS** | Hours between background janitor cycles. | `24` | Yes |
| **JANITOR_WEB_PRUNE_DAYS** | Prune web chunks older than N days in janitor `ingest-web`. | `30` | Yes |
| **JANITOR_TRIM_KEEP_LAST** | Keep last N lines of `training.jsonl` when janitor trims. | `100000` | Yes |
| **JANITOR_REVIEW_LESSONS_DAYS** | Run `review-lessons` every N days (stamp: `data_dir/.last_review_lessons`). `0` = disable. | `7` | Yes |
| **JANITOR_WITH_SERVER** | Truthy = server spawns background janitor on startup. | (none) | Yes |
| **RAG_MAX_RESPONSE_CHARS** | Max chars for RAG context in responses; `0` = no truncation. | `32000` | Yes |
| **EXECUTE_SHELL_MAX_OUTPUT_CHARS** | Max chars for `execute_shell_command` stdout+stderr; `0` = no truncation. | `16000` | Yes |
| **EXECUTE_SHELL_TIMEOUT_SECS** | Max seconds for `execute_shell_command`; `0` = no timeout (not recommended). | `120` | Yes |
| **EXECUTE_SHELL_ALLOWLIST** | Comma-separated program names for shell/terminal (e.g. `cargo,git,grep,ls,npm,pytest`). Unset = built-in list. | `cargo,git,grep,ls,npm` | Yes |
| **HUMANIZER_URL** | When set, POST lesson/training text to this URL before writing (e.g. humanize API). | (none) | Yes |
| **TAVILY_API_KEY** | When set, `search_web` tool uses Tavily search API. | (none) | Yes |
| **SERPER_API_KEY** | When set, `search_web` tool uses Serper search API (alternative to Tavily). | (none) | Yes |
| **RUST_LOG** | Tracing filter (e.g. `info`, `rag_mcp=debug`, `mcp_timing=info`). | (none) | Yes |
| **SEMANTIC_CACHE_ENABLED** | When `1` or `true`, cache `query_knowledge` responses by semantic similarity to reduce cost/latency. | (off) | Yes |
| **SEMANTIC_CACHE_MAX_ROWS** | Max rows in semantic cache table; prune on startup. | `500` | Yes |
| **SEMANTIC_CACHE_TTL_SECS** | Delete cache rows older than this (seconds). | `86400` (24h) | Yes |
| **SEMANTIC_CACHE_SIMILARITY_THRESHOLD** | Cosine similarity threshold for cache hit (0.85–0.95). | `0.90` | Yes |
| **MCP_AUDIT_LOG_PATH** | When set, append one JSONL line per tool call (request_id, tool, params_hash, elapsed_ms, error). Operator responsible for rotation. | (none) | Yes |
| **MCP_FULL_TOOLS** | When `1` or `true`, `tools/list` returns all tools. **Default is off:** the server returns only 3 tools (query_knowledge, get_relevant_tools, invoke_tool) for token savings. Other tools are available via `get_relevant_tools` and `invoke_tool`. Set `MCP_FULL_TOOLS=1` only when you need the full tool list in the client UI. | (off) | Yes |
| **MCP_MAX_TOOL_CALLS_PER_SESSION** | Max tool calls per process (session). When set to a positive integer (e.g. 200), the server returns an error after that many calls; `0` = disabled. Prevents runaway sessions. | `0` (disabled) | Yes |
| **RESPONSE_FORMAT** | `json` (default) or `compact` for RAG/skill responses (compact = single-line `{"t": content}`). | `json` | Yes |
| **OTEL_EXPORTER_OTLP_ENDPOINT** | When the server is built with `--features otel`, traces are exported to this OTLP endpoint (e.g. collector or Jaeger). | `http://localhost:4317` | Yes (only when `otel` feature enabled) |

---

## 3. Core Commands

| Command | Description |
|---------|-------------|
| **serve** | (Default.) Run the MCP server on STDIO. Used by Cursor or other MCP clients. Training: `query_knowledge` with `execute=true` can append to `training.jsonl`. |
| **ingest** \<path\> | Recursively index a directory into the RAG DB (BLAKE3 manifest, parallel ingest). Path must be under `ALLOWED_ROOTS`. Run after cloning or when you want to refresh the index. |
| **ingest-web** | Fetch URLs from `data/web_sources.json`, write `data/web.jsonl`, ingest into RAG. Optional: `--prune-after-days N` to prune web chunks older than N days. |
| **query** \<string\> | Run one RAG query and print formatted context to stdout. Example: `rag-mcp query "ownership rules rust"`. Useful for quick verification without the MCP server. |

### 3.1 Skills and pipeline

Skills live in `Skills/` (or `02_Skills/` / `skills/` when **RULES_VAULT** or **GLOBAL_RULES_DIR** is set). The agent discovers them via **list_skill_metadata** then **get_skill_content**(skill_id), or via **query_knowledge** when skill content is indexed in RAG. RECALL and pipeline steps should consider skills when the task is procedural or quality-focused (MCP server quality, pipeline phases, security checklists). After adding or changing skill files, run **refresh_file_index** on the Skills paths (or full `rag-mcp ingest <repo_root>`) so query_knowledge can retrieve updated content. See [docs/SKILL_INDEX.md](../../docs/SKILL_INDEX.md) for the roster.

### 3.2 Why so many rag-mcp processes? How to prevent

**Why:** Cursor spawns **one** `rag-mcp` process **per MCP connection**. You get extra processes when:

- You have **multiple Cursor windows** or **multiple chats** that each use the monolith server.
- Cursor **reconnects** (e.g. after Reload Window, or retries) and starts a new process without the previous one exiting.
- Cursor **exits uncleanly** (crash, force close); the child `rag-mcp` processes are left running as orphans. Next time you open Cursor, more are spawned.

So you can end up with several to many `rag-mcp.exe` instances (e.g. 8+) that are redundant and can block a clean rebuild.

**How to prevent / reduce:**

1. **Before a rebuild:** Always kill existing instances so the next Cursor connection uses the new binary:
   - **Windows:** `taskkill /F /IM rag-mcp.exe`
   - **Or:** From repo root: `powershell -ExecutionPolicy Bypass -File scripts/ops/kill_orphan_mcp.ps1 -Force`
2. **Use one Cursor window** for this workspace when possible; each extra window can add another process.
3. **Close Cursor cleanly** when you’re done (File → Exit / Quit) so it can terminate its MCP child processes.
4. **Periodically:** If you see “MCP disconnected” or odd behavior, run the kill command above, then reload the window or restart Cursor so a single fresh process starts.

There is no in-process “single instance” lock in the server today; the above workflow keeps the number of processes under control.

---

## 4. The Background Janitor

The **background** command runs a continuous loop: ingest workspace → ingest-web (with prune) → trim-training, then sleeps for a configurable interval.

**How to run:**

- **Standalone:**  
  `rag-mcp background`  
  Optionally: `--interval-hours 24 --web-prune-days 30 --trim-keep-last 100000` (defaults come from env if not set).
- **With the server:** Set `JANITOR_WITH_SERVER=true` in the environment used to start `rag-mcp serve`; the server will spawn the janitor on startup.

**What it does each cycle:**

1. Runs `rag-mcp ingest <first-allowed-root>`.
2. Runs `rag-mcp ingest-web --prune-after-days N` (N from `JANITOR_WEB_PRUNE_DAYS` or flag).
3. Runs `rag-mcp trim-training --keep-last M` (M from `JANITOR_TRIM_KEEP_LAST` or flag).
4. If `JANITOR_REVIEW_LESSONS_DAYS` is set and > 0: runs `rag-mcp review-lessons` when the last run was more than N days ago (stamp file: `data_dir/.last_review_lessons`).
5. Sleeps for `JANITOR_INTERVAL_HOURS` (or `--interval-hours`).

Use for set-and-forget freshness of RAG and training log size. Run in background (e.g. `nohup rag-mcp background &` or Windows `Start-Process -WindowStyle Hidden`).

**Periodic sweep (repo-level):** From the repo root, run `scripts/sweep.ps1` to run cargo check, cargo test, cargo clippy in monolith and a lightweight secrets grep; the report is written to `docs/reports/SWEEP_YYYY-MM-DD.md`. Schedule via Task Scheduler or cron if desired. To list recent outbox results by date, run `scripts/task_status.ps1`.

**Slim verification (quarterly):** Run the five Slim verification steps in [PIPELINE_VERIFICATION_CHECKLIST.md](../../docs/setup/PIPELINE_VERIFICATION_CHECKLIST.md) (scripts audit, archive reports >90 days, rag-mcp --help, GET_RELEVANT_TOOLS_TOP_K). Next run due date is noted in the checklist.

---

## 5. Testing & Validation

| Command | Purpose |
|---------|---------|
| **chaos** | Chaos engineering: path traversal, AST overload, RRF keyword, secrets. Writes `docs/reports/CHAOS_TEST_RESULTS.md`. Run to prove resilience; all phases should PASS. |
| **verify-retrieval** | Runs fixed RAG queries (same path as `query_knowledge`). Exits 1 if no chunks returned. Run after ingest and before relying on training or web ingest. |
| **audit** | One-shot environment check: ORT, Nomic, Reranker, Qwen paths. Run before first ingest to catch missing DLLs or model paths. |

**Suggested order:** `audit` → `ingest <path>` → `verify-retrieval` → (optional) `chaos`.

---

## 6. Agent / MCP client instructions

When configuring an MCP client (e.g. Cursor) to use this server, instruct the agent to:

- **Tool selection:** When unsure which tool matches the user's request, call `get_tool_selection_guide` or `get_relevant_tools`; do not guess among tools.
- **Long-running commands:** Use `fork_terminal` for long-running or interactive commands (e.g. `npm run dev`, `cargo run`) so the user sees live output and MCP does not block.
- **Design system:** Use `get_design_tokens` when design system data (colors, typography) is needed; use `get_ui_blueprint` and `verify_ui_integrity` for layout and linting.
- **Rules sync:** Use `compile_rules` when `RULES_VAULT` or `GLOBAL_RULES_DIR` is set and the user wants rules synced to a project (writes `.cursorrules`, `GEMINI.md`, `CLAUDE.md`).

**Reference docs:** [MCP_Tools_Reference.md](MCP_Tools_Reference.md) (per-tool what/when/value), [TOOL_SELECTION_GUIDE.md](TOOL_SELECTION_GUIDE.md) (situation→tool table). Link to these from your MCP server configuration so agents can open them.

**Tool list integrity:** The response to `tools/list` includes `meta.tool_list_checksum` (16-char hex). Clients or gateways can compare this value across sessions to detect if the server’s tool set has changed ("rug pull" detection).

---

## 7. Disaster Recovery

**Critical assets:**

- **RAG database:** `DATA_DIR/rag.db` (or vault path). SQLite DB containing workspace chunks, vectors, FTS index, symbol index.
- **Training dataset:** `DATA_DIR/training.jsonl` (or `GOLDEN_SET_DIR/training.jsonl`). One JSONL row per training event; used for LoRA/QLoRA or curation.
- **Manifest (optional):** `DATA_DIR/rag_manifest.json`. Content hashes for ingest skip/dirty; can be recreated by re-ingesting.

**Backup instructions:**

1. **rag.db:** Copy `rag.db` (and if present `rag.db-wal`, `rag.db-shm`) to a backup location. Ensure no other process has the DB open when copying, or use SQLite backup API / `sqlite3 .backup` for a consistent snapshot.
2. **training.jsonl:** Copy the file; it is append-only. No special shutdown required.
3. **Restore:** Replace `rag.db` (and WAL/SHM) with the backup; restore `training.jsonl` if desired. Restart the server. If the DB was from a different machine or path, ensure `ALLOWED_ROOTS` and paths in the DB still match the current environment.

**After data loss:** Re-run `rag-mcp ingest <path>` to rebuild the RAG index. Training rows are not recoverable unless backed up.

---

## 8. Dependencies

- **ort (ONNX Runtime):** The project uses `ort` 2.0.0-rc.11 (release-candidate). When stable 2.x is released, upgrade to it (see `Cargo.toml` note). Semantic search and Nomic embeddings depend on ONNX; without ort, the server runs FTS-only.
- **rmcp:** Pinned to a specific patch version (e.g. 0.15.0) to avoid unnoticed breaking changes. When adding long-running tools or task semantics, consider rmcp 0.16+ (SEP-1686 tasks).

---

## 9. Host-only deployment (no Docker)

Docker-based deployment has been removed; run the `rag-mcp` Rust binary directly on the host as documented in MCP-SETUP.md and this runbook.

**Docker containers named `mcp` or `vector-db` (e.g. in Docker Desktop):** This repo does **not** define or start any Docker containers. There is no `docker-compose` or Dockerfile here for MCP or a vector DB. Those containers, if present, come from elsewhere (e.g. another project, a tutorial, or Docker Desktop’s “MCP Toolkit” feature). They are **not required** for this workspace: monolith uses its own host-run binary and `data/rag.db` (SQLite). You can safely stop/remove the `mcp` and `vector-db` containers when working only in this repo. To see where a container came from: `docker inspect <container_name>` and check `Config.Labels` or `Mounts` for compose project / image source.

---

## 10. Optional OpenTelemetry (gateway / multi-server)

When the server is placed behind a gateway or used in a multi-server setup, you can build with the **otel** feature to export tracing spans to an OpenTelemetry collector (or Jaeger). Spans include tool calls and RAG steps; the request-scoped `request_id` is available in log output and can be correlated with OTel trace IDs when the layer is configured.

**Build:** `cargo build --release --features otel`  
**Run:** Set `OTEL_EXPORTER_OTLP_ENDPOINT` (default `http://localhost:4317`) to your collector or agent. Ensure the collector is running so the pipeline can connect; otherwise startup may fail.

---

## 11. Observability: TSR/TTC and prompt–tool correlation

**Task Success Rate (TSR) and Turns-to-Completion (TTC):** The server does not compute these internally. You can derive them from the optional audit log:

- Set `MCP_AUDIT_LOG_PATH` to a file. Each line is JSON: `request_id`, `tool`, `params_hash`, `elapsed_ms`, `error` (null on success).
- **TSR:** Over a time window, count lines where `error` is null vs non-null; success rate = successes / total.
- **TTC:** Count tool calls per “turn” or session. The server does not have a session/turn boundary; each tool call has its own `request_id`. To measure turns, aggregate by a client-supplied session id if you add one, or count tool calls between client “user message” boundaries in your pipeline.

**Prompt–tool correlation:** Each tool call gets a unique `request_id` (16-char hex). All `mcp_timing` logs and the audit log line for that call include `request_id`, so you can correlate one tool invocation with its RAG steps and outcome. To correlate a **user prompt** to the tool calls that followed it, the client or gateway must associate a session/trace id with the prompt and with subsequent tool requests; the server does not receive the user prompt, only `tools/call` requests. When using the **otel** feature, trace IDs in OTel can link tool calls; attach the same trace id to the client’s “prompt” span if your client supports it.

---

## 12. Gateway: circuit breaker, retries, idempotent tools

When this server is behind a **gateway** that aggregates multiple MCP servers or forwards requests:

- **Circuit breaker:** Use a circuit breaker around calls to this server (or any downstream MCP). Open the circuit after N failures or a high error rate; half-open to probe before closing again. Prevents cascading failure when the server is slow or unavailable.
- **Retries:** Retry **only idempotent** operations. Safe to retry: `list_tools`, `read_resource`, `get_doc_outline`, `get_section`, `query_knowledge` (read-only), `get_relevant_tools`, `discover_tool`, `read_manifest`, `get_system_status`, `get_tool_selection_guide`, `get_design_tokens`, `get_ui_blueprint`, `list_skill_metadata`, `get_skill_content`, `get_skill_reference`, `get_codebase_outline`, `project_packer`, `resolve_symbol`, `get_related_code`. Do **not** retry non-idempotent tools (e.g. `commit_to_memory`, `save_rule_to_memory`, `refresh_file_index`, `execute_shell_command`, `invoke_tool`, `log_training_row`, `approve_pattern`, `auto_approve_pattern`, `ingest_web_context`, `submit_task`) unless the protocol supports exactly-once semantics.
- **Caching:** Avoid caching `tools/call` results; you may cache `tools/list` (and use `meta.tool_list_checksum` to invalidate when the tool set changes).

The response to `tools/list` includes `meta.tool_list_checksum` and `meta.idempotent_tools` (array of tool names that are safe to retry). Use `meta.idempotent_tools` when configuring gateway retry policy.

---

## 13. Remote vs stdio, and when to add a gateway

**What “remote” means:** Today this server runs **stdio-only**: the client (e.g. Cursor) spawns the process and talks over stdin/stdout. A **remote** setup would expose the same MCP protocol over the network—typically **SSE/HTTP** (MCP’s standard remote transport). That implies: a long-lived HTTP server, authentication/authorization (IAM), TLS, and possibly rate limits. The rag-mcp binary does **not** implement an HTTP/SSE transport; you’d put it behind a **gateway** or proxy that does.

**What a “gateway” is:** A component that sits between clients and one or more MCP servers. It can: (1) expose MCP over HTTP/SSE so remote clients can connect, (2) aggregate several MCP servers (e.g. rag-mcp + filesystem + Slack) behind one endpoint, (3) add circuit breakers, retries (for idempotent tools only), and observability (see §11–§12). The gateway speaks MCP to the backend servers (often via stdio by spawning them, or via another transport).

**When stdio-only is enough:** Single user, IDE (Cursor/Claude Code) on the same machine, one or a few MCP servers configured in the client. No need for a gateway or remote exposure.

**When to add a gateway (and thus “remote”):** (1) **Multiple users or services** that need to call MCP without each running the server process locally. (2) **Remote clients** (browser, mobile, another datacenter) that cannot spawn stdio processes. (3) **Many MCP servers** you want behind one URL with one auth and one place to enforce circuit breakers and retries. (4) **Centralized observability** (e.g. OTel) where the gateway assigns trace IDs and forwards to rag-mcp (stdio or remote).

**Summary:** If you only use Cursor/IDE on your machine with rag-mcp as one of the MCP servers, you don’t need a remote transport or gateway. Add a gateway when you need multi-user access, remote clients, or a single entry point for several MCP servers with resilience (circuit breaker, retries) and observability.
