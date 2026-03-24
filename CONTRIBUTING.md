# Contributing to Monolith MCP

Thanks for your interest in contributing!

## Prerequisites

- **Rust** (stable, latest recommended)
- **ONNX Runtime 1.23+** (optional; FTS-only mode works without it)

## Getting Started

```bash
git clone https://github.com/DakodaStemen/Stratum
cd Stratum
make setup          # downloads models + builds
cargo test          # run tests
```

For a faster build without ONNX models:

```bash
make build-fts      # FTS5-only, no model download needed
```

## Code Style

- Format with `cargo fmt` before committing
- Lint with `cargo clippy -- -D warnings`
- No `TODO`, `FIXME`, or `unimplemented!()` in submitted code

## Pull Requests

1. Fork the repo and create a feature branch
2. Write tests for new functionality
3. Ensure `cargo test`, `cargo clippy`, and `cargo fmt --check` all pass
4. Keep PRs focused -- one feature or fix per PR
5. Write a clear description of what changed and why

## Project Structure

```
src/                  Rust source code
  rag/                RAG pipeline (chunking, embedding, retrieval)
  rag/handler/        MCP tool implementations
  commands/           CLI subcommands
  tools/              Web fetch, search, Slack
skills/               Curated knowledge library (147 skills)
docs/                 Architecture docs, tool reference
scripts/              Build and maintenance scripts
```

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
