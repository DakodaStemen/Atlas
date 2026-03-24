# MCP Server Architectural Blueprint (2026)

This document defines the "Perfect Structure" for the Model Context Protocol (MCP) server, ensuring scalability, high-signal retrieval, and high-quality agentic reasoning.

---

## 1. The Five-Pillar Structure

| Pillar | Location | Description |
|--------|----------|-------------|
| **Skills (The Atoms)** | `skills/01-09/` | Atomic, categorized markdown files following the `SKILL_SCHEMA`. Each skill provides idiomatic, high-signal knowledge on a single topic. |
| **Agents (The Neurons)** | `.cursor/rules/agents/` | Specialized `.mdc` rules that define domain-specific principles, pipelines, and reference sets. They act as "mental models" for the LLM. |
| **Context (The Docs)** | `docs/` | Comprehensive technical documentation, ADRs (Architecture Decision Records), PRDs, and Research reports. The "Source of Truth" for the system's "Why." |
| **Tools (The Effectors)** | `scripts/` & MCP Tools | Automated scripts (PowerShell, Python, Rust) that perform repeatable tasks, maintenance, and complex integrations. |
| **Memory (The Data)** | `data/` & `AGENTS.md` | RAG indexed data, training metrics, and durable workspace facts. Keeps the system "learned" across sessions. |

---

## 2. Skill Categories (01-09 Taxonomy)

1.  **01-Orchestration:** Workflows, PRDs, ADRs, Task Coordination.
2.  **02-Research:** Codebase Analysis, Market Research, Technical Reporting.
3.  **03-Frontend-UI:** Design Systems, React/Angular, Accessibility, Performance.
4.  **04-Backend-Systems:** API Design, Server-side Logic, Concurrency.
5.  **05-Data-Engineering:** Databases (SQL/NoSQL/Vector), ETL, Data Modeling.
6.  **06-AI-Engineering:** Agent Development, LLM Integration, RAG, Evals.
7.  **07-Infrastructure:** Cloud (AWS/GCP/Azure), K8s, CI/CD, IaC.
8.  **08-Security-Audit:** Audit, Compliance, Secret Scanning, Pentest.
9.  **09-Quality-Assurance:** Unit/E2E Testing, Performance Validation.

---

## 3. The Agentic Loop (RECALL -> RESEARCH -> EXECUTE)

1.  **RECALL:** Every turn starts with `query_knowledge` (RECALL) to pull the most relevant atoms (skills) and context (docs).
2.  **ROUTING:** The `agentic-operator.mdc` (Global) routes to a specialized Agent in `.cursor/rules/agents/` based on the domain.
3.  **RESEARCH:** If gaps exist, the `knowledge-evolution.mdc` agent triggers the research pipeline.
4.  **EXECUTE:** The agent uses the Atoms (Skills) to implement a high-quality, idiomatic solution.
5.  **EVOLVE:** Successes are logged to `lessons_learned.md`, and new knowledge is atomized into skills.

---

## 4. Quality Guardrails

- **No Proactive Docs:** Documentation is only created or updated upon explicit user request.
- **Surgical Edits:** Use `replace` or targeted tools to minimize context usage.
- **Continuous Indexing:** Every skill addition triggers a rebuild of the `SKILL_INDEX.md`.
- **Zero-Waste:** Follow the `zero-waste-philosophy.md` for token savings and high-signal output.
