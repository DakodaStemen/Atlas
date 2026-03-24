# Scripts

Project scripts. **Pipeline orchestration is in the rag-mcp binary**; use `rag-mcp` subcommands from repo root or `monolith/`. See [monolith/docs/](../monolith/docs/) for CLI and operations (e.g. MCP_Tools_Reference.md, OPERATIONAL_RUNBOOK.md).

## Replaced by rag-mcp (use these instead)

| Old script | Use instead |
|------------|-------------|
| compile_synapse_context.ps1 | `rag-mcp compile-constitution` |
| build_web_sources.ps1 | `rag-mcp build-web-sources` |
| run_janitor_cycle.ps1 | `rag-mcp janitor-cycle` or `rag-mcp background` |
| run_data_clean.ps1 | `rag-mcp data-clean` |
| run_ingest_web_background.ps1 | `rag-mcp ingest-web` (or schedule `rag-mcp janitor-cycle`) |
| research_pipeline.ps1 | MCP tools (fetch_web_markdown, ingest_web_context) or `rag-mcp ingest-from-jsonl <path>` |
| scheduled_maintenance.ps1 | Schedule `rag-mcp janitor-cycle` (e.g. Task Scheduler) |
| ops/prune_manifest_stale.py | `rag-mcp prune-manifest-stale` |
| godly_rag/*.py | `rag-mcp list-tools`, `rag-mcp query "..."`, MCP query_knowledge / ingest |
| training/*.py (merge, sample, prep, etc.) | `rag-mcp merge-unsloth-jsonl`, `rag-mcp sample-training` |

## Remaining scripts

Full audit: [AUDIT.md](AUDIT.md). All recurring operations use `rag-mcp` or are explicit keep.

| Script | Purpose | Use rag-mcp |
|--------|---------|-------------|
| [sweep.ps1](sweep.ps1) | Integrity + secrets sweep; writes `docs/reports/SWEEP_YYYY-MM-DD.md`. | Equivalent: `rag-mcp verify-integrity` + scan_secrets; sweep calls cargo only. |
| [task_runner.ps1](task_runner.ps1) | Process _tasks/inbox (research, ingest, refresh_file_index, verify-integrity, research_ingest, data-clean); outbox. | All types call `cargo run --manifest-path ... --` (rag-mcp). |
| [task_status.ps1](task_status.ps1), [drop_task.ps1](drop_task.ps1) | Task queue status and drop. | — |
| [ops/data_quality_audit.py](ops/data_quality_audit.py) | Deep data quality: RAG DB, training.jsonl, manifest, web_sources. | Routine: `rag-mcp janitor-cycle` or `rag-mcp data-clean`. |
| [ops/validate_mcp_config.js](ops/validate_mcp_config.js) | Validate MCP config. | — |
| [ops/kill_orphan_mcp.ps1](ops/kill_orphan_mcp.ps1) | Kill leftover `rag-mcp` processes before Cursor reconnect. | — |
| [ops/weekly_knowledge_pulse.py](ops/weekly_knowledge_pulse.py) | Weekly knowledge pulse (optional). | Schedule: `rag-mcp janitor-cycle` or `rag-mcp ingest-web`. |
| [ops/watchdog.ps1](ops/watchdog.ps1) | Watchdog: no commit for N min → log or kill. | Optional: call `rag-mcp janitor-cycle` / `rag-mcp data-clean`. |
| [ouroboros/](ouroboros/) | Ouroboros clean, batch, edits; calls `rag-mcp bin-to-jsonl`, `rag-mcp prune-manifest-stale`. | Yes. |
| [orchestrator/](orchestrator/) | Route task, run orchestrator, metrics, e2e, audit. | — |
| [test/](test/) | full-test.bat, design viewer smoke, random code sample, speedtest. | Verify: `rag-mcp verify-integrity`. |
| [ci/verify_ui_integrity.ps1](ci/verify_ui_integrity.ps1) | Lint UI files against DESIGN_AXIOMS. | Calls `cargo run -- verify-ui-integrity`. |
| [training/run_training_queries.ts](training/run_training_queries.ts) | Training queries. Run from repo root: `npx ts-node scripts/training/run_training_queries.ts`. | — |

See [docs/README.md](../docs/README.md) for full doc index; [docs/setup/AGENTIC_OPERATOR_RULE.md](../docs/setup/AGENTIC_OPERATOR_RULE.md) for agent behavior.
