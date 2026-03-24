.PHONY: setup build build-fts run test clippy fmt download-models ingest clean help

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

setup: download-models build ## Full setup: download models + build release binary

build: ## Build release binary (with ONNX embeddings)
	cargo build --release

build-fts: ## Build without ONNX (FTS5 keyword search only, faster build)
	cargo build --release --no-default-features

run: ## Start MCP server (stdio transport)
	./target/release/rag-mcp serve

test: ## Run all tests
	cargo test

clippy: ## Run Clippy linter
	cargo clippy -- -D warnings

fmt: ## Check formatting
	cargo fmt -- --check

download-models: ## Download ONNX models and runtime
	bash scripts/download-models.sh

ingest: ## Ingest a directory (usage: make ingest PATH=./src)
	./target/release/rag-mcp ingest $(PATH)

audit: ## Run environment audit
	./target/release/rag-mcp audit

clean: ## Clean build artifacts
	cargo clean
