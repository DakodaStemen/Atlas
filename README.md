# Stratum

[![CI](https://github.com/DakodaStemen/Stratum/actions/workflows/unified-quality.yml/badge.svg)](https://github.com/DakodaStemen/Stratum/actions/workflows/unified-quality.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org/)

A single-binary MCP server that brings local RAG to your IDE. Hybrid BM25 + semantic search, ONNX embeddings, cross-encoder reranking, 43+ tools, and 147 curated engineering skills — all running on your machine. No cloud calls for search, no data leaving your box.

It started as a way to give an AI coding assistant persistent memory and real codebase understanding without shipping everything off to a third-party API. It grew from there.

## Getting started

```bash
git clone https://github.com/DakodaStemen/Stratum.git
cd Stratum
make setup        # downloads models (~600MB) and builds the release binary
cp .env.example .env
```

Edit `.env` with your API keys (all optional — the server works without them), then wire it into your IDE (see below).

If you don't want the ONNX models at all, `make build-fts` gives you a smaller FTS5-only binary with no model download required.

## How it works

The MCP server sits between your IDE and your codebase. When you ask a question, it runs BM25 keyword search and semantic vector search in parallel, fuses the results with Reciprocal Rank Fusion, and passes the top candidates through a cross-encoder reranker before returning them. All of this runs locally using SQLite FTS5 and sqlite-vec.

Tools are exposed through a gateway: by default only 5 are visible to keep token usage down. You call `get_relevant_tools` with a plain-English description of what you want, and it returns the right subset.

## IDE setup

Add this to your project's `.mcp.json` (Claude Code), `.cursor/mcp.json` (Cursor), or `.windsurf/mcp.json` (Windsurf):

```json
{
  "mcpServers": {
    "monolith": {
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

Then index your project:

```bash
./target/release/rag-mcp ingest /path/to/your/project
./target/release/rag-mcp audit   # sanity check
```

## Configuration

Full list in [`.env.example`](.env.example). The ones you'll actually touch:

| Variable | Default | Notes |
|----------|---------|-------|
| `ORT_DYLIB_PATH` | — | Path to ONNX Runtime `.so`/`.dll`. Required for semantic search. |
| `DATA_DIR` | `./data` | Where the SQLite databases live. |
| `ALLOWED_ROOTS` | `.` | Directories the server is allowed to ingest. |
| `GOOGLE_API_KEY` | — | Enables Gemini model routing. Free tier works fine. |
| `MCP_FULL_TOOLS` | `0` | Set to `1` to expose all 43+ tools instead of the default 5. |

## Build variants

```bash
cargo build --release                        # with ONNX embeddings (default)
cargo build --release --no-default-features  # FTS5-only, no models needed
cargo build --release --features cuda        # CUDA acceleration
cargo build --release --features otel        # OpenTelemetry tracing
```

## Tools

The main ones you'll use:

| Tool | What it does |
|------|-------------|
| `query_knowledge` | Search your indexed codebase |
| `get_relevant_tools` | Find the right tool for a task |
| `invoke_tool` | Run any discovered tool |
| `execute_shell_command` | Run allowlisted shell commands |
| `refresh_file_index` | Re-index after file changes |
| `get_related_code` | Definition + all references for a symbol |
| `resolve_symbol` | Jump-to-definition |
| `verify_integrity` | build + test + clippy in one call |
| `commit_to_memory` | Save learnings across sessions |
| `search_web` | Web search via Tavily or Serper |
| `fetch_web_markdown` | Fetch a URL as clean markdown |
| `plan_task` | Break a task into tracked steps |

Full reference: [`docs/MCP_Tools_Reference.md`](docs/MCP_Tools_Reference.md)

## Skills library

147 curated skills covering orchestration, research, frontend, backend, data engineering, AI/ML, infrastructure, security, QA, and content creation. See [`docs/SKILL_INDEX.md`](docs/SKILL_INDEX.md) for the full list.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
