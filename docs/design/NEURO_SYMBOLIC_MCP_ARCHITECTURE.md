# Neuro-Symbolic MCP Architecture: The "Self-Evolving Brain" (2026)

This document details the architectural blueprint for transforming the MCP server into a highly autonomous, self-healing, self-learning, and self-documenting "Neural Network" of agentic capabilities.

## 1. Core Philosophy: The Neuro-Symbolic Agent
The MCP server is no longer just a collection of scripts; it is a **continuous reasoning engine**. 
*   **Neuro (LLM/Agent):** The creative, flexible problem-solving capability (Gemini/Claude).
*   **Symbolic (MCP/RAG):** The hard-coded facts, deterministic tools, schemas, and indexed memory.
*   **The Network:** Specialized agents (`.cursor/rules/agents/`) act as distinct "cortical columns", communicating through a shared memory state (`STATE.md` and RAG).

## 2. The Autonomic Nervous System (Self-Healing & Self-Fixing)
The system must automatically recover from failures without human intervention.
*   **The Guardrail Loop:** Before any code is committed, it passes through `review_diff` and `scan_secrets`.
*   **The Auto-Fix Protocol:** If `verify_integrity` (tests, linting, compilation) fails, the server intercepts the stderr/exit code and immediately routes it to the `analyze_error_log` tool. The agent automatically executes up to 3 repair cycles (`Plan -> Act -> Validate`) before ever asking the user.
*   **Fallback Memory:** If a system dependency crashes, the `devex-tooling` agent consults `lessons_learned.md` for historical workarounds.

## 3. The Hippocampus (Self-Learning & Self-Documenting)
The system must never lose context and must permanently acquire new skills.
*   **Zero-Shot Research Trigger:** If `query_knowledge` (RECALL) returns "No relevant information found", the agent **must not invent a solution**. Instead, it triggers the `knowledge-evolution.mdc` agent.
*   **The Ingestion Pipeline:** 
    1. `google_web_search` for high-signal sources or GitHub repos.
    2. `web_fetch` to ingest the raw markdown/code.
    3. `generalist` synthesis to create a clean, atomic `SKILL.md`.
    4. Auto-rebuild of `SKILL_INDEX.md` and immediate injection into the RAG database.
*   **The Harvesting Subroutine:** The system actively monitors public GitHub repositories (e.g., `vercel-labs/agent-skills`, `anthropics/skills`) using scheduled `harvest_skills_from_github.ps1` runs to absorb community knowledge without wasting context window.

## 4. The Prefrontal Cortex (Continuous Context & Self-Saving)
The system maintains a durable "train of thought" across sessions and reboots.
*   **Continuous `STATE.md`:** At the end of every major sub-task, the agent updates `STATE.md` with:
    *   Current Phase & Position.
    *   Last completed action.
    *   Next immediate action.
    *   Active blockers.
*   **The Roll-Over Mechanism:** On session start, the agent reads `STATE.md`. If a task was interrupted, it immediately resumes where it left off.
*   **Evolution Logging:** Non-trivial successes are automatically logged via `log_training_row` and `commit_to_memory` to build the system's fine-tuning dataset over time.

## 5. The Specialized Hemispheres (The 16 Agents)
The system routes tasks to dedicated neurological centers (Agents in `.cursor/rules/agents/`):
*   **Strategic/Abstract:** `product-strategy`, `system-architecture`, `knowledge-evolution`.
*   **Logic/Execution:** `backend-systems`, `frontend-ui`, `data-engineering`, `infrastructure`.
*   **Validation/Security:** `security-audit`, `quality-assurance`, `offensive-security`.
*   **Specialized Processing:** `fintech-payments`, `gis-geospatial`, `ai-engineering`.
## 7. The Adversarial Layer (The Skeptic / Logic Auditor)
To solve for confirmation bias and "shortcut" reasoning, the architecture includes a mandatory **Internal Audit Loop**.
*   **The Skeptic Persona:** A dedicated Agent (`logic-auditor.mdc`) whose sole purpose is to challenge the primary agent's plans.
*   **Audit-by-Exception:** High-stakes operations (security, bulk changes, architectural shifts) trigger a mandatory internal debate.
*   **Shadow Critique:** The system uses the `generalist` sub-agent with a "Skeptic" persona to provide a second, heterogeneous opinion on complex logic, ensuring that "System 2" thinking is applied to high-entropy tasks.

## 8. Implementation Plan for the Next 1,000 Skills
...

Instead of writing skills from scratch, the server will "absorb" the global intelligence of the CS community:
1.  **Run the Harvester:** Execute `harvest_skills_from_github.ps1` against known gold-standard repositories.
2.  **Run the Deep CS Scraper:** Utilize web search agents to pull down modern 2026 patterns (GraphRAG, WASM runtimes, Edge Computing, Neuro-Symbolic architectures).
3.  **Deduplicate and Synapse:** Merge the absorbed knowledge into the 01-09 taxonomy, ensuring there are no overlapping "neurons" (duplicate skills), creating a dense, high-signal RAG index.
