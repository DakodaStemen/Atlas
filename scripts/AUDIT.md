# Scripts Audit — Slim Plan Part 1

Classification: **(A)** replaceable by rag-mcp, **(B)** thin wrapper that calls rag-mcp, **(C)** keep (with reason).

| Script | Purpose | Classification | Replacement / Note |
|--------|---------|-----------------|---------------------|
| sweep.ps1 | Integrity + secrets sweep; writes docs/reports/SWEEP_YYYY-MM-DD.md | B | Thin wrapper: cargo check, test, clippy + secrets grep. Use `rag-mcp verify-integrity` + scan_secrets for equivalent; sweep.ps1 calls manifest-path only. |
| task_runner.ps1 | Process _tasks/inbox by type; write to outbox; uses cargo/rag-mcp only | B | Keep. Uses only `cargo run --manifest-path` for research, ingest, refresh_file_index, verify-integrity, research_ingest. Add type `data-clean` → `rag-mcp data-clean`. |
| task_status.ps1 | List or summarize task outbox | C | Keep. Thin helper for task queue. |
| drop_task.ps1 | Drop a JSON task into inbox | C | Keep. Thin helper. |
| ops/data_quality_audit.py | Deep data quality: RAG DB, training.jsonl, manifest, web_sources | C | Keep. Deep audit; optional future port to `rag-mcp audit-data-quality`. Run `rag-mcp janitor-cycle` or `rag-mcp data-clean` on a schedule for routine maintenance. |
| ops/weekly_knowledge_pulse.py | Weekly knowledge pulse (optional) | B | Document: schedule `rag-mcp janitor-cycle` or `rag-mcp ingest-web` via Task Scheduler for recurring ingest. Script can remain for custom logic. |
| ops/validate_mcp_config.js | Validate MCP config | C | Keep. Config validation. |
| ops/watchdog.ps1 | Watchdog: no commit for N min → log or kill | C | Keep. See docs/architecture/COGNITIVE_CEILING.md. |
| ouroboros/run_ouroboros_clean.ps1 | bin-to-jsonl + backup + replace + prune-manifest-stale | B | Keep. Calls only `cargo run -- bin-to-jsonl`, `cargo run -- prune-manifest-stale`. Option B (thin wrapper). |
| ouroboros/*.ps1, *.py | Ouroboros batch, edits, limits, tests | C | Keep. Optional Ouroboros helpers. |
| orchestrator/*.ps1 | Route task, run orchestrator, ingest metrics, e2e, gemini audit, token report | C | Keep. Orchestration/orchestrator-specific. |
| test/full-test.bat | Full test entry point | B | Calls cargo/test; consolidate with `rag-mcp verify-integrity` where applicable. |
| test/*.ps1 | Design viewer smoke, random code sample, speedtest, god-mode | C | Keep. Test helpers. |
| ci/verify_ui_integrity.ps1 | Run verify-ui-integrity on UI files | B | Thin wrapper: calls `cargo run --manifest-path ... -- verify-ui-integrity`. Keep. |
| training/run_training_queries.ts | Training queries (TypeScript) | C | Keep. Training pipeline. |

**Exit criterion:** No script invokes a deleted script. Every recurring operation has a rag-mcp path or explicit "keep" reason.
