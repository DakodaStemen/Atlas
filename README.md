# Stratum

A single-binary, local-first MCP server that gives AI coding assistants persistent memory, real codebase understanding, and a curated library of engineering skills — without sending your code to a third-party search API.

[![CI](https://github.com/DakodaStemen/Stratum/actions/workflows/unified-quality.yml/badge.svg)](https://github.com/DakodaStemen/Stratum/actions/workflows/unified-quality.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org/)

---

## Table of Contents

- [What It Is](#what-it-is)
- [How the Search Pipeline Works](#how-the-search-pipeline-works)
- [The Gateway Pattern](#the-gateway-pattern)
- [Getting Started](#getting-started)
- [IDE Integration](#ide-integration)
- [Configuration](#configuration)
- [Build Variants](#build-variants)
- [Tool Reference](#tool-reference)
- [Skills Library](#skills-library)
- [Memory and Persistence](#memory-and-persistence)
- [CI / Quality Gates](#ci--quality-gates)
- [Contributing](#contributing)
- [License](#license)

---

## What It Is

Stratum is an MCP (Model Context Protocol) server written in Rust that runs entirely on your local machine. It exposes a search index over your codebase and a curated library of engineering skills to any MCP-compatible IDE (Claude Code, Cursor, Windsurf, and others).

It grew out of a practical problem: AI coding assistants are good at synthesis but have no persistent view of a codebase. They rely on the IDE to feed them context, which means every session starts cold, every cross-file question requires manual context assembly, and nothing learned in one session carries forward. Stratum solves this with a local RAG pipeline that the IDE can query at any time, combined with a memory layer that persists across sessions.

**What runs locally (no cloud calls):**
- BM25 keyword search (SQLite FTS5)
- Semantic vector search (sqlite-vec + ONNX Nomic embeddings)
- Cross-encoder reranking (ONNX ms-marco-MiniLM)
- All 147 skills
- All memory reads and writes

**What optionally calls external APIs:**
- Web search (`search_web` — requires Tavily or Serper key)
- Web fetch (`fetch_web_markdown` — optional Jina key for cleaner output)
- Model routing (`route_task` — requires Anthropic/Gemini/OpenAI key)

---

## How the Search Pipeline Works

A query goes through four stages before results are returned:

### 1. Parallel Retrieval

BM25 and semantic search run simultaneously via Tokio tasks:

- **BM25 (FTS5):** SQLite's built-in full-text search. Fast, exact keyword matching with TF-IDF scoring. Excellent for identifiers, function names, error strings, and any case where the user knows what they are looking for.
- **Semantic (sqlite-vec + ONNX):** The query is embedded with `nomic-embed-text-v1.5` (768-dimensional, ONNX Runtime, runs on CPU). The embedding is compared against pre-indexed chunk embeddings using cosine similarity via sqlite-vec's `vec_cosine_distance`. This retrieves conceptually related results that do not share keywords.

### 2. Reciprocal Rank Fusion

Results from both retrieval legs are merged using Reciprocal Rank Fusion (RRF):

```
RRF(d) = Σ_r  1 / (k + rank_r(d))
```

where `k = 60` (standard constant) and `rank_r(d)` is the rank of document `d` in retrieval leg `r`. Documents that appear in both BM25 and semantic results get a doubled signal; documents exclusive to one leg still contribute. The fusion score is purely rank-based — no score normalization across legs is required.

### 3. Cross-Encoder Reranking

The top N fused candidates (default: 20) are passed through a cross-encoder model (`ms-marco-MiniLM-L-6-v2`, ONNX). Unlike bi-encoders (which embed query and document independently), a cross-encoder reads the query and document together and outputs a relevance score. This is slower but substantially more accurate for borderline candidates.

The reranker runs on CPU via ONNX Runtime. Latency on a 20-candidate set is typically 50–200ms depending on average chunk length.

### 4. Result Formatting

The reranked results are assembled into a context block with file paths, line ranges, and content. The `execute` parameter controls output mode: `execute=false` returns raw results for the IDE to inspect; `execute=true` returns a synthesized context block ready for the model to read.

### Ingestion

Before search works, the codebase must be indexed:

```bash
./target/release/rag-mcp ingest /path/to/your/project
```

The ingestion pipeline:
1. Walks the directory tree, respecting `.gitignore` and the `ALLOWED_ROOTS` allowlist.
2. Chunks each file into overlapping windows (default: 512 tokens, 64-token overlap) using a token-aware chunker.
3. Hashes each chunk with BLAKE3. Chunks whose hash matches the stored hash are skipped — only new or changed chunks are re-embedded.
4. Embeds new chunks with `nomic-embed-text-v1.5` via ONNX Runtime and stores them in sqlite-vec.
5. Updates the FTS5 index.

Re-indexing after file changes is handled by `refresh_file_index`, which re-runs ingestion incrementally.

---

## The Gateway Pattern

Stratum exposes 43+ tools, but by default only 5 are visible to the model. Exposing all 43 at once adds thousands of tokens of tool descriptions to every request — expensive and noisy.

The default visible tools are:
- `query_knowledge` — search the indexed codebase
- `get_relevant_tools` — discover other tools by natural-language description
- `invoke_tool` — execute any discovered tool by name
- `execute_shell_command` — run an allowlisted shell command
- `commit_to_memory` — persist a learning across sessions

To find and use any other tool, the model calls `get_relevant_tools("description of what I need")`, which performs a skills-index search and returns matching tool names and signatures. Then `invoke_tool(name, args)` runs it.

Set `MCP_FULL_TOOLS=1` to expose all tools at once (useful for exploration or agents that need direct access).

---

## Getting Started

**Requirements:** Rust stable toolchain, ~600 MB disk for ONNX models.

```bash
git clone https://github.com/DakodaStemen/Stratum.git
cd Stratum
make setup        # downloads models and builds release binary
cp .env.example .env
# edit .env with your API keys (all optional)
```

`make setup` downloads:
- `nomic-embed-text-v1.5.onnx` — embedding model (~275 MB)
- `ms-marco-MiniLM-L-6-v2.onnx` — cross-encoder reranker (~23 MB)
- ONNX Runtime shared library

Then index your project:

```bash
./target/release/rag-mcp ingest /path/to/your/project
./target/release/rag-mcp audit   # verify the index is healthy
```

---

## IDE Integration

Add this to your project's `.mcp.json` (Claude Code), `.cursor/mcp.json` (Cursor), or `.windsurf/mcp.json` (Windsurf):

```json
{
  "mcpServers": {
    "stratum": {
      "command": "/path/to/Stratum/target/release/rag-mcp",
      "args": ["serve"],
      "env": {
        "ORT_DYLIB_PATH": "/path/to/Stratum/lib/libonnxruntime.so.1.23.0",
        "DATA_DIR": "/path/to/Stratum/data",
        "ALLOWED_ROOTS": "/path/to/your/project",
        "RUST_LOG": "info"
      }
    }
  }
}
```

The server name (`"stratum"` above) is arbitrary — use whatever makes sense in your workflow. The tool names exposed by the server are independent of this key.

---

## Configuration

Full reference in [`.env.example`](.env.example). The variables you will actually use:

| Variable | Default | Description |
|----------|---------|-------------|
| `ORT_DYLIB_PATH` | — | Absolute path to the ONNX Runtime `.so`/`.dll`. Required for semantic search and reranking. |
| `DATA_DIR` | `./data` | Directory where SQLite databases are stored. |
| `ALLOWED_ROOTS` | `.` | Colon-separated list of directories the server is allowed to ingest and read. Paths outside this list are rejected. |
| `GOOGLE_API_KEY` | — | Enables Gemini model routing via `route_task`. The free tier is sufficient for most uses. |
| `ANTHROPIC_API_KEY` | — | Enables Claude model routing. |
| `OPENAI_API_KEY` | — | Enables OpenAI model routing. |
| `TAVILY_API_KEY` | — | Enables `search_web` via Tavily. |
| `SERPER_API_KEY` | — | Alternative web search provider. |
| `JINA_API_KEY` | — | Improves `fetch_web_markdown` output quality. |
| `MCP_FULL_TOOLS` | `0` | Set to `1` to expose all 43+ tools instead of the default gateway 5. |
| `MAX_RESULTS` | `10` | Default number of results returned by `query_knowledge`. |
| `CHUNK_SIZE` | `512` | Token window size for ingestion chunking. |
| `CHUNK_OVERLAP` | `64` | Token overlap between adjacent chunks. |

---

## Build Variants

```bash
# Default: full pipeline with ONNX embeddings and reranking
cargo build --release

# FTS5 only: no ONNX, no model download, smaller binary
cargo build --release --no-default-features

# CUDA acceleration for embeddings (requires CUDA toolkit)
cargo build --release --features cuda

# OpenTelemetry tracing (exports spans to an OTLP endpoint)
cargo build --release --features otel
```

The FTS5-only build (`--no-default-features`) is the right choice for CI environments or machines where the ~300 MB model download is not practical. Search quality is lower (keyword-only, no semantic retrieval) but the build is fast and has zero model dependencies.

---

## Tool Reference

The core tools used in day-to-day AI-assisted development:

| Tool | Description |
|------|-------------|
| `query_knowledge` | Hybrid BM25 + semantic search over the indexed codebase. Primary entry point for all codebase questions. |
| `get_relevant_tools` | Searches the tool registry by plain-English description. Returns matching tool names and signatures. Use this before `invoke_tool`. |
| `invoke_tool` | Execute any tool by name with a JSON argument object. |
| `get_related_code` | Returns the definition of a symbol plus all of its call sites and references. |
| `resolve_symbol` | Jump-to-definition: given a symbol name, returns the file path and line number of its definition. |
| `execute_shell_command` | Run an allowlisted shell command. Permitted commands: `cargo`, `git`, `grep`, `ls`, `npm`. Attempts to run anything else are rejected. |
| `refresh_file_index` | Re-index changed files. Run this after making file changes so search results reflect the current state. |
| `verify_integrity` | Runs `cargo build`, `cargo test`, and `cargo clippy` in sequence and returns the combined output. Single call to confirm nothing is broken. |
| `commit_to_memory` | Persist a key learning, decision, or rule to the memory store. Persists across sessions. |
| `save_rule_to_memory` | Persist a constraint or behavioral rule that should govern future behavior. |
| `plan_task` | Break a complex task into tracked steps with a structured plan. |
| `get_loop_state` | Check the state of an active managed control loop. |
| `analyze_error_log` | Given an error log or stack trace, identify the root cause and suggest a fix. |
| `get_file_history` | Retrieve the git history for a file. Essential before deleting or refactoring code (Chesterton's fence). |
| `security_audit` | Run a static analysis security scan over specified paths. |
| `scan_secrets` | Scan staged or modified files for accidentally committed secrets or credentials. |
| `search_web` | Web search via Tavily or Serper. Requires the corresponding API key. |
| `fetch_web_markdown` | Fetch a URL and return its content as clean markdown. |
| `ingest_web_context` | Fetch a URL, parse it, and add it to the knowledge base. |
| `route_task` | Route a task to the most appropriate AI model based on task type, cost constraints, and available providers. |
| `get_doc_outline` | Return the heading structure of a documentation file. |
| `get_section` | Return a specific section of a documentation file by heading. |

Full reference with argument schemas: [`docs/MCP_Tools_Reference.md`](docs/MCP_Tools_Reference.md)

---

## Skills Library

147 curated skills organized into 10 domains. Skills are markdown documents with YAML frontmatter that the server can retrieve and inject as context.

| Domain | Count | Examples |
|--------|-------|---------|
| Orchestration | 22 | MEGA-AI-TOOLS, MEGA-ARCHITECTURE-PATTERNS, CI/CD workflows |
| Research | 4 | Competitive intelligence, research methodology, R data science |
| Frontend/UI | 12 | React, Tailwind v4, Three.js/WebGL, WebGPU, Electron/Tauri |
| Backend Systems | 12 | Go, Rust, Python, Node/Express, Hono, Supabase, WebAssembly |
| Data Engineering | 16 | Vector DBs, Postgres optimization, Redis streams, dbt, Spark |
| AI Engineering | 14 | RAG systems, LLM fine-tuning, MCP client patterns, evals |
| Infrastructure | 16 | AWS, Kubernetes, Terraform, Cloudflare, observability stacks |
| Security | 14 | OWASP patterns, Vault/Consul, k8s security, supply chain |
| Quality Assurance | 10 | Playwright, Vitest/Jest, Cypress, k6 load testing, pytest |
| Content / YouTube | 13 | Research, writing, SEO, thumbnail, audio, distribution |

Full index: [`docs/SKILL_INDEX.md`](docs/SKILL_INDEX.md)

---

## Memory and Persistence

Stratum maintains two persistence layers:

**Operational memory** (`commit_to_memory`, `save_rule_to_memory`): Key/value entries written to a SQLite table. Entries are retrieved by semantic similarity to the current query, so relevant past decisions surface automatically without the model having to remember to ask for them.

**Web context** (`ingest_web_context`): Documentation pages, RFCs, and research fetched at query time are chunked and added to the same search index as local files. This lets the model search across local code and ingested web context in a single query.

---

## CI / Quality Gates

Four GitHub Actions workflows run on every push and PR:

| Workflow | What it checks |
|----------|----------------|
| `unified-quality.yml` | Build (all feature variants), clippy, tests |
| `security-audit.yml` | `cargo audit` for known CVEs in dependencies |
| `performance.yml` | Query latency benchmarks; fails if regression > 20% |
| `integration-tests.yml` | End-to-end MCP tool call smoke tests via Playwright |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

MIT License — see [LICENSE](LICENSE).

Copyright (c) 2026 Dakoda Stemen
