---
name: rust-comprehensive-reference
description: Comprehensive reference for Rust development — web frameworks (Axum, Actix-web), async (Tokio), serialization (Serde), testing, game dev (Bevy), and WebAssembly.
domain: languages
category: rust
tags: [Rust, Axum, Actix-web, Serde, Tokio, async, testing, Bevy, WASM, cargo]
triggers: Rust best practices, Axum router, Actix-web, Serde patterns, Rust testing, Bevy ECS, wasm-bindgen, Cargo workspace
---

# Rust Comprehensive Reference

This document is a unified, high-density reference for Rust development across web, systems, games, and WebAssembly. It consolidates multiple fragmented skill files into a single source of truth.

---

## Table of Contents

1. [Async Web Development (Axum & Actix-web)](#1-async-web-development)
2. [Async/Await & Tokio Runtime](#2-asyncawait--tokio-runtime)
3. [Serialization (Serde)](#3-serialization-serde)
4. [Testing Patterns](#4-testing-patterns)
5. [Game Development (Bevy)](#5-game-development-bevy)
6. [WebAssembly (wasm-bindgen & wasm-pack)](#6-webassembly)
7. [Cargo & Workspace Management](#7-cargo--workspace-management)

---

## 1. Async Web Development

### Axum vs Actix-web

| Feature | Axum | Actix-web |
| --- | --- | --- |
| Ecosystem | Tower/Tokio | Own actix-rt |
| Macros | Minimal (none on handlers) | Attribute-heavy (`#[get("/")]`) |
| Middleware | Tower `Service` trait | Own `Transform`/`Service` |
| State | `State<T>` extractor | `web::Data<T>` extractor |

### Axum Patterns (v0.8+)
- **Router:** Parameters use `{name}` and `{*wildcard}`.
- **Extractors:** Processed in order; body-consuming extractor (JSON, Form) must be last.
- **State:** Use `Arc<AppState>` with `.with_state(state)`.
- **Error Handling:** Errors are HTTP responses via `IntoResponse`.

### Actix-web Patterns
- **App Data:** Use `web::Data::new(state)` created *outside* the closure for shared state.
- **Configure:** Modularize routes with `.configure(module::config)`.
- **Middleware:** Order is reversed; last `.wrap()` executes first.

---

## 2. Async/Await & Tokio Runtime

- **Fundamentals:** Futures are lazy; `.await` suspends tasks.
- **Runtime:** Default to `#[tokio::main]`. Use `spawn_blocking` for CPU-bound or synchronous I/O.
- **Task Spawning:** `tokio::spawn` requires `'static` and `Send`.
- **Channels:**
  - `mpsc`: Multi-producer, single-consumer work funnel.
  - `oneshot`: Single response.
  - `broadcast`: Pub/sub (all receivers see every message).
  - `watch`: Latest value only.

---

## 3. Serialization (Serde)

Serde is a zero-cost, compile-time serialization framework.

### Field Attributes
- `#[serde(rename = "...")]`: Change name on wire.
- `#[serde(default)]`: Use `Default` trait if field is missing.
- `#[serde(skip)]`: Ignore field during ser/de.
- `#[serde(flatten)]`: Inline fields of a nested struct.

### Enum Representations
- **External (default):** `{"Variant": { ... }}`
- **Internal:** `#[serde(tag = "type")]` -> `{"type": "Variant", ...}`
- **Adjacent:** `#[serde(tag = "t", content = "c")]` -> `{"t": "Variant", "c": { ... }}`
- **Untagged:** `#[serde(untagged)]` -> `{ ... }` (ambiguous, use sparingly).

---

## 4. Testing Patterns

### Levels of Testing
- **Unit Tests:** Inside the module `mod tests { #[cfg(test)] }`. Can access private items.
- **Integration Tests:** In `tests/` directory. Public API only.
- **Doc Tests:** `/// ```` examples in comments that stay compilable.

### Tools & Patterns
- **tokio::test:** For async test execution.
- **proptest:** Property-based testing with shrinking.
- **mockall:** Trait mocking for isolating dependencies.
- **cargo-nextest:** Faster parallel test runner with process isolation.
- **cargo-llvm-cov:** Source-based code coverage.

---

## 5. Game Development (Bevy)

Bevy is an ECS-driven engine (v0.15+).

### ECS Primitives
- **Entities:** Unique IDs (not stable across sessions).
- **Components:** Plain structs. Use `#[require(T)]` for component dependencies.
- **Systems:** Functions that run logic on queries.
- **Resources:** Global singletons (Score, Config).

### Systems & Scheduling
- **Schedules:** `Startup`, `Update`, `FixedUpdate` (physics).
- **Ordering:** Use `.after()`, `.before()`, or `.chain()`.
- **Observers (v0.14+):** Push-style reactive event handlers.

---

## 6. WebAssembly

- **wasm-bindgen:** Generates JS/Rust glue.
- **wasm-pack:** Standard build/package tool.
- **web-sys:** Rust bindings for Web APIs (DOM, Fetch, Canvas).
- **Memory:** Minimize boundary crossings; strings/slices are copied.
- **serde-wasm-bindgen:** Preferred for passing structured data between JS and Rust.

---

## 7. Cargo & Workspace Management

- **Cargo Add:** Run `cargo search <crate>` first to confirm name and version.
- **Monolith Paths:** Always use canonical paths (e.g., `monolith/data/`).
- **Integrity:** Run `verify_integrity` from the monolith root.
- **Profiles:** Use `panic = "abort"` and `lto = true` for small WASM binaries.

---

## Checklist

- [ ] `cargo check` / `cargo clippy` runs clean.
- [ ] Async tasks are `Send` and `'static` if spawned.
- [ ] No blocking I/O in async handlers.
- [ ] Serde tags are explicit for external APIs.
- [ ] Tests use `mockall` for external services.
- [ ] Bevy systems are small and focused for parallelism.
- [ ] WASM feature flags are explicitly enabled in `web-sys`.
