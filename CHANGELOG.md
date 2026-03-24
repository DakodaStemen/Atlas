# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Refactored `agentic-monolith/` directory to `monolith/` (commit e81c08a)
- Consolidated codebase from 247 files to 49 files across multiple passes
- Replaced hardcoded `C:\Projects\...` ZIO path with `ZIO_BOUNTY_CORE_PATH` env var
- Expanded `SECRET_FILENAME` regex to catch `.env`, `credentials.json`, `api_key.*`, etc.
- Upgraded Ollama fallback model from nonexistent `qwen3-coder:30b` to `qwen2.5-coder:32b`

### Fixed

- Stagnation detection float comparison bug in control loop (epsilon-based comparison)
- Semantic cache now invalidated on `refresh_file_index` to prevent stale cached responses
- Git diff timeout added (30s) in control loop to prevent hanging on large repos
- Secret scan output now masks values instead of returning raw truncated secrets
- Semgrep ruleset parameter validated against allowed roots to prevent path injection
- CI workflow glob patterns fixed (`skills/**/*.md` instead of `skills/*/*.md`)
- Nightly workflow SKILL_INDEX.md path corrected (`docs/` not `skills/`)
- Nightly workflow git push fixed with proper permissions and auth token
- `build_skill_index.sh` case mismatch fixed (`skills/` not `Skills/`)
- Neural janitor hardcoded path replaced with relative path detection
- 8 broken documentation links in README-SETUP.md fixed or replaced
- Corrupted duplicate research paper deleted

### Removed

- Redundant `ci.yml` workflow (unified-quality.yml covers same jobs)
- Vestigial `monolith/package-lock.json` (Node wrapper was removed)

## [1.1.0] - 2026-03-07

### Added

- RAG pipeline: SQLite + sqlite-vec, Nomic ONNX embeddings (768-d), FTS5 hybrid search, cross-encoder reranking, MMR diversification.
- MCP tools: `query_knowledge`, `get_related_code`, `resolve_symbol`, `fetch_web_markdown`, `ingest_web_context`, `rag-mcp ingest-web` with `web_sources.json`.
- Graph resources: `graph://symbol/{name}` for symbol definitions and references.
- SQLite WAL mode and `busy_timeout` for concurrent ingest and query; tempfile isolation in tests.
- Shell output sanitization (paths, API keys redacted) for `execute_shell_command`.
- Ingest path added to allowed roots when running `rag-mcp ingest <path>` so paths outside default roots index correctly.
- Web chunks (URL sources) included in retrieval via `path_under_allowed` for `http://` and `https://` sources.
- `rag-mcp query <query>` CLI subcommand for verification.

### Changed

- Tavily removed; web content via `ingest_web_context` or `rag-mcp ingest-web` only. No API key required for RAG web ingest.
- Embed dimension validated at embedder and store; FTS-only ingest when embedder unavailable; clearer error when DB has 0-d vectors.

### Fixed

- Ingest 0 chunks when path outside `ALLOWED_ROOTS`: CLI ingest now adds the ingest path to allowed roots for that run.
- "Expected 0 floats" / embedding dimension mismatch: documented fix (set `ORT_DYLIB_PATH`, delete rag.db, re-ingest in same session).

### Security

- `sanitize_shell_output` applied to all `execute_shell_command` stdout/stderr so logs and training data do not leak paths or API keys.

---

When cutting a release: add a new `## [X.Y.Z] - YYYY-MM-DD` section, move Unreleased items into it, and bump version in `monolith/Cargo.toml`.
