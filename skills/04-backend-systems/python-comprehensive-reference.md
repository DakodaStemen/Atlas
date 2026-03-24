---
name: python-comprehensive-reference
description: Comprehensive reference for Python development — modern tooling (uv, Ruff), async patterns, web frameworks (FastAPI, Flask, Django), security, distributed computing (Ray), and AI integration (Claude SDK, MCP).
domain: languages
category: python
tags: [Python, uv, Ruff, FastAPI, Flask, Django, async, security, Ray, Claude, MCP, DBOS]
triggers: Python best practices, modern Python tooling, Python async, FastAPI patterns, Python web security, Claude Python SDK, Python MCP server
---

# Python Comprehensive Reference

This document is a unified, high-density reference for modern Python development. It consolidates multiple fragmented skill files into a single source of truth covering tooling, frameworks, security, and AI integration.

---

## Table of Contents

1. [Modern Tooling & Project Layout](#1-modern-tooling--project-layout)
2. [Async Python Patterns (asyncio)](#2-async-python-patterns)
3. [Web Frameworks (FastAPI, Flask, Django)](#3-web-frameworks)
4. [Web Security Specifications](#4-web-security-specifications)
5. [Data Validation (Pydantic v2)](#5-data-validation-pydantic-v2)
6. [Durable Workflows (DBOS)](#6-durable-workflows-dbos)
7. [Distributed Computing (Ray)](#7-distributed-computing-ray)
8. [AI Integration (Claude SDK & Agentic Patterns)](#8-ai-integration-claude-sdk--agentic-patterns)
9. [MCP Server Implementation](#9-mcp-server-implementation)
10. [Domain Specific (QGIS Automation)](#10-domain-specific-qgis-automation)

---

## 1. Modern Tooling & Project Layout

### The Standard Stack
- **uv:** Replaces pip, virtualenv, poetry, pyenv. Fast Rust-based binary.
- **Ruff:** Replaces flake8, black, isort. Extremely fast linter and formatter.
- **mypy / pyright:** Static type checking.
- **pyproject.toml:** Single configuration source for the whole project.

### Layout Best Practices
- **src layout:** Package code under `src/<package_name>/`.
- **Environment:** Use `uv venv` or `uv sync` to manage `.venv`.
- **Lockfile:** Always commit `uv.lock` to version control.

---

## 2. Async Python Patterns

- **Entry Point:** Always use `asyncio.run(main())`.
- **Concurrency:**
  - `TaskGroup` (Python 3.11+): Preferred over `gather`.
  - `Semaphore`: Throttle concurrency to avoid overwhelming services.
- **Blocking I/O:** Never call sync I/O inside `async def`. Use `run_in_executor` or `asyncio.to_thread`.
- **Queues:** Use `asyncio.Queue` for producer-consumer patterns.

---

## 3. Web Frameworks

### FastAPI
- **Async First:** Best for I/O-bound tasks.
- **Dependency Injection:** Use `Depends` for DB sessions, auth, etc.
- **Background Tasks:** Built-in `BackgroundTasks` for post-response work.

### Flask
- **WSGI:** Synchronous by default. Use Gunicorn for production.
- **App Factory:** Use `create_app()` pattern for better testing and config.

### Django
- **Full Featured:** ORM, Admin, Auth included.
- **Security:** Built-in CSRF, XSS, and SQLi protections. Use `manage.py check --deploy`.

---

## 4. Web Security Specifications

### General MUSTs
- **Secrets:** Never log or commit `SECRET_KEY`, API keys, or passwords.
- **Debug:** `DEBUG = False` in production.
- **Allowed Hosts:** Set strict `ALLOWED_HOSTS` / `TRUSTED_HOSTS`.
- **Cookies:** Set `Secure`, `HttpOnly`, and `SameSite='Lax'`.

### Injection Prevention
- **SQLi:** Use ORM or parameterized queries. Never use f-strings for SQL.
- **Command Injection:** Pass args as a list to `subprocess.run()`. Avoid `shell=True`.
- **XSS:** Rely on template auto-escaping. Use `mark_safe` only on trusted data.

---

## 5. Data Validation (Pydantic v2)

- **Config:** Use `model_config = ConfigDict(from_attributes=True)`.
- **Validators:** `@field_validator` (single field) and `@model_validator` (cross-field).
- **Settings:** Use `BaseSettings` for environment variable management.

---

## 6. Durable Workflows (DBOS)

- **Durable Workflows:** Resilient to failures, state persists in DB.
- **Steps:** Any non-deterministic or external API call MUST be a `@DBOS.step()`.
- **Workflow:** Orchestrates steps via `@DBOS.workflow()`.

---

## 7. Distributed Computing (Ray)

- **Tasks:** `@ray.remote` functions for parallel execution.
- **Actors:** `@ray.remote` classes for stateful distributed services.
- **Object Store:** Shared memory for zero-copy data sharing (Numpy arrays).
- **Ray Serve:** Scalable model serving.

---

## 8. AI Integration (Claude SDK & Agentic Patterns)

### Agent SDK
- **query():** Main entry point for agentic interactions.
- **Hooks:** Use `PostToolUse` for auditing or logging edits.
- **Permission Modes:** `default`, `plan`, `acceptEdits`, `bypassPermissions`.

### Messages API Features
- **Streaming:** Use `client.messages.stream()` for real-time output.
- **Tool Use:** Use `@beta_tool` decorator with `tool_runner`.
- **Structured Outputs:** Use `client.messages.parse(output_format=MyPydanticModel)`.
- **Thinking:** Use `thinking: {type: "adaptive"}` for Opus 4.6.

### Supporting APIs
- **Files API:** Upload once, reference multiple times via `file_id`.
- **Batches API:** Process up to 100k requests asynchronously at 50% cost.

---

## 9. MCP Server Implementation

- **FastMCP:** High-level framework for building MCP servers.
- **Naming:** `{service}_mcp` (e.g., `github_mcp`).
- **Tools:** Action-oriented `snake_case` names (e.g., `slack_send_message`).
- **Validation:** Use Pydantic models for `inputSchema`.

---

## 10. Domain Specific

### QGIS Automation (PyQGIS)
- **Initialization:** Must call `QgsApplication.initQgis()` in standalone scripts.
- **Layer API:** Manage vector and raster layers via `QgsVectorLayer` / `QgsRasterLayer`.
- **Processing:** Run GIS algorithms via `processing.run()`.

---

## Checklist

- [ ] Project uses `uv` and `src` layout.
- [ ] Type hints are applied to public APIs.
- [ ] No blocking calls inside async event loop.
- [ ] Security headers and secure cookie attributes are set for production.
- [ ] AI tools use explicit Pydantic schemas for structured output.
- [ ] Production deployments use multiple workers (Uvicorn/Gunicorn).
