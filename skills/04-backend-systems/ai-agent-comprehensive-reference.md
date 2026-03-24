---
name: ai-agent-comprehensive-reference
description: Comprehensive reference for building AI agents and LLM-powered applications with Claude, MCP, and Cloudflare AI tools.
domain: ai-ml
category: agents
tags: [Claude, AI, MCP, Agent-SDK, Cloudflare, AutoRAG, RealtimeKit, LLM]
triggers: Claude API, Anthropic SDK, MCP integration, AI search, building AI agents, Model Context Protocol
---

# AI Agent & Claude Comprehensive Reference

This document is a unified, high-density reference for building AI agents and LLM-powered applications. It consolidates multiple fragmented skill files covering the Claude API, Model Context Protocol (MCP), and Cloudflare AI services.

---

## Table of Contents

1. [Claude API Defaults & Model Selection](#1-claude-api-defaults--model-selection)
2. [Language-Specific SDK Support](#2-language-specific-sdk-support)
3. [Model Context Protocol (MCP)](#3-model-context-protocol-mcp)
4. [AI Search (Cloudflare AutoRAG)](#4-ai-search-cloudflare-autorag)
5. [RealtimeKit (Cloudflare)](#5-realtimekit-cloudflare)
6. [AI Agent Design & Quality](#6-ai-agent-design--quality)

---

## 1. Claude API Defaults & Model Selection

### Recommended Model
- **Primary:** `claude-opus-4-6` (Claude Opus 4.6).
- **Secondary:** `claude-sonnet-4-6`, `claude-haiku-4-5`.
- **CRITICAL:** Use exact model strings. Do not append date suffixes.

### Core Features
- **Adaptive Thinking:** Always use `thinking: {type: "adaptive"}` for Opus 4.6. Note: `budget_tokens` is deprecated for 4.6 models.
- **Effort Parameter:** Control reasoning depth via `output_config: {effort: "low"|"medium"|"high"|"max"}`.
- **Streaming:** Default to streaming for long inputs/outputs to prevent timeouts. Use `.get_final_message()` to collect results.

---

## 2. Language-Specific SDK Support

| Language | Tool Runner | Agent SDK | Integration Pattern |
| ---------- | ----------- | --------- | ------------------- |
| **Python** | Yes (beta) | Yes | `@beta_tool` decorator |
| **TypeScript** | Yes (beta) | Yes | `betaZodTool` + Zod |
| **Go** | Yes (beta) | No | `BetaToolRunner` |
| **Java** | Yes (beta) | No | Annotated classes (`Supplier<String>`) |
| **Ruby** | Yes (beta) | No | `BaseTool` + `tool_runner` |
| **C#** | No | No | IChatClient integration |
| **PHP** | No | No | Official client only |

---

## 3. Model Context Protocol (MCP)

MCP enables plugins to integrate with external services via structured tool access.

### Configuration
- **Method 1:** Dedicated `.mcp.json` at plugin root (Recommended).
- **Method 2:** Inline in `plugin.json`.

### Server Types
- **stdio:** Local child processes (best for local files/DBs).
- **SSE:** Server-Sent Events (best for cloud services with OAuth).
- **Streamable HTTP:** Bidirectional HTTP (modern replacement for SSE).
- **WebSocket:** Real-time bidirectional communication.

### Implementation Best Practices
- **Naming:** Python: `{service}_mcp`; Node: `{service}-mcp-server`.
- **Tool Naming:** `snake_case` with service prefix: `slack_send_message`.
- **Gateway Pattern:** Expose only 3-5 tools by default. Use `get_relevant_tools` + `invoke_tool`.
- **Validation:** Use Pydantic (Python) or Zod (TS) for `inputSchema`.

---

## 4. AI Search (Cloudflare AutoRAG)

Unified interface for semantic search and retrieval.

### Core API
- **aiSearch():** Semantic search with LLM-generated answer.
- **search():** Raw vector retrieval of chunks.
- **Filters:** Support `and`/`or` logic on metadata like `filename`, `folder`, `timestamp`.

### Gotchas
- **Precision:** Use 10-digit Unix seconds for timestamps.
- **Folders:** Use `gte` operator for "starts with" folder matching.
- **Limits:** Max 4MB per file; 6-hour index cycle.

---

## 5. RealtimeKit (Cloudflare)

SDK for building real-time collaborative apps (video/audio/chat).

### Architecture
- **meeting.self:** Local participant state and controls.
- **meeting.participants:** Reactive Maps of remote participants.
- **meeting.ai:** Access to live transcripts.

### Troubleshooting
- **Mismatched Count:** `meeting.participants` does NOT include `self`.
- **CORS:** All REST API calls MUST be server-side.
- **Events:** Register listeners BEFORE calling `meeting.join()`.

---

## 6. AI Agent Design & Quality

### Bounded Autonomy
- Require human approval for destructive operations (file deletes, large commits).
- Always use a sandbox (like E2B) for executing agent-generated code.

### Context Management
- Use **Compaction** (beta) for long conversations to prevent context window overflow.
- **RECALL First:** Always perform a knowledge search (`query_knowledge`) as the first action each turn.

---

## Checklist

- [ ] Model is set to `claude-opus-4-6`.
- [ ] Adaptive thinking is enabled for complex reasoning.
- [ ] Compaction state is preserved across turns (append `response.content`).
- [ ] MCP tools are pre-allowed in command frontmatter.
- [ ] Sensitive operations require explicit human approval.
- [ ] Timeouts and rate limits are handled via SDK exceptions.
