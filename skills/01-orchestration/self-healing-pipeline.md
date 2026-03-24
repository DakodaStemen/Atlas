---
name: self-healing-pipeline
description: Autonomous loop for detecting knowledge gaps, auto-researching, generating skills, and validating staleness. Triggers when query_knowledge returns no results for a domain-relevant task.
domain: orchestration
tags: [self-healing, knowledge-gap, auto-research, skill-generation, staleness, validation]
triggers: knowledge gap, no relevant information, self-heal, staleness check, skill validation, auto-research
---

# Self-Healing Orchestration Pipeline

Autonomous loop that detects knowledge gaps, fills them via research, optionally generates new skills, and validates existing skills for staleness.

## 1. Knowledge Gap Detection

**Trigger:** `query_knowledge` returns "No relevant information found" for a task that clearly needs domain knowledge.

### Procedure

1. **Log the gap:**
   - Topic (keywords used in the failed query)
   - Context (what task the agent was performing)
   - Timestamp
   - Append to `docs/lessons_learned.md` under a `## Knowledge Gaps` section if not already tracked.

2. **Check for near-matches:**
   - Scan `docs/SKILL_INDEX.md` for skills with overlapping domain/tags.
   - Re-query `query_knowledge` with broader/alternate keywords (synonyms, parent concepts).
   - If a near-match exists: use it and note the gap in terminology for future alias improvement.

3. **If no match found:** proceed to Research Flow.

## 2. Research Flow (Auto-Fill)

**Goal:** Acquire the missing knowledge with minimal noise.

### Procedure

1. `search_web("<topic> best practices site:docs OR site:github OR site:rfc-editor")` — prioritize official docs, specs, RFCs.
2. For each high-signal result (max 3): `fetch_web_markdown(url)`.
3. Validate and store:
   - `research_and_verify(topic, [urls])` for multi-source corroboration.
   - OR `ingest_web_context(url, topic)` for single authoritative sources.
4. **Decision gate — skill or memory?**
   - **Create a skill** if: topic is reusable across projects, involves a multi-step procedure, or covers a tool/framework the agent will encounter again.
   - **Memory only** if: topic is one-off, project-specific, or ephemeral. Run `commit_to_memory` with a concise summary.

## 3. Skill Creation Flow (Auto-Generate)

**Trigger:** Research Flow decides the topic warrants a reusable skill.

### Procedure

1. **Determine category folder:**
   - Map the topic domain to `skills/XX-category/` (e.g., `01-orchestration`, `02-development`, etc.).
   - If no category fits, use the closest match or propose a new one.

2. **Author the skill file** with SKILL_SCHEMA frontmatter:
   ```yaml
   ---
   name: <kebab-case-name>
   description: <one-line summary of what the skill does and when to use it>
   domain: <category>
   tags: [<3-6 relevant tags>]
   triggers: <comma-separated trigger phrases>
   ---
   ```

3. **Write concise procedures** — not essays. Follow existing conventions:
   - Numbered steps for sequential flows.
   - Decision gates for branching logic.
   - Tool calls referenced by name (e.g., `query_knowledge`, `fetch_web_markdown`).

4. **Register the skill:**
   - Add an entry to `docs/SKILL_INDEX.md` with name, description, and file path.
   - Run `refresh_file_index` so the skill is immediately discoverable.

5. **Post-creation checks:**
   - `scan_secrets` — ensure no credentials leaked.
   - `review_diff` — sanity-check the generated content.
   - `commit_to_memory` — log that a new skill was auto-generated, with topic and path.

## 4. Staleness Detection

**Criteria for flagging a skill as potentially stale:**

| Condition | Action |
|---|---|
| Skill references a tool/framework with a known major version change (e.g., Next.js 14 -> 15) | Flag for review |
| Skill file `mtime` > 6 months AND no `freshness-checked` metadata | Flag for review |
| Skill contains external URLs | Validate links; flag if any return 4xx/5xx |
| Skill references files that no longer exist in the repo | Flag for immediate update |
| Skill's domain tools have been deprecated or renamed | Flag for immediate update |

**When to run:** On explicit user request ("check staleness", "validate skills"), or as a periodic maintenance task.

## 5. Validation Protocol

**Trigger:** Manual invocation or scheduled review.

### Procedure

1. **Enumerate all skills:** Glob `skills/**/*.md`.
2. **For each skill, check:**
   - **Frontmatter parses?** YAML between `---` fences must be valid. Required fields: `name`, `description`.
   - **Referenced files exist?** Any path mentioned in the skill body should resolve relative to repo root.
   - **External links valid?** For each URL, `fetch_web_markdown(url)` — flag if it fails or returns error content.
   - **Age check:** Compare file `mtime` against 6-month threshold.
3. **Generate staleness report:**
   - List skills by status: `OK`, `STALE`, `BROKEN`.
   - For each flagged skill: reason, last modified date, recommended action.
   - Write report to `docs/research/staleness-report.md` (overwrite previous).
4. **Flag skills needing update:**
   - Add `<!-- NEEDS_REVIEW: <reason> -->` comment to the top of flagged skill files.
   - Log flagged skills via `commit_to_memory` for agent awareness.

## Loop Summary

```
Task requires domain knowledge
        │
        ▼
  query_knowledge(topic)
        │
   ┌────┴────┐
   │ Found?  │
   └────┬────┘
   Yes  │  No
   ▼    │   ▼
  Use   │  Log gap → Check SKILL_INDEX near-matches
  it    │              │
        │         ┌────┴────┐
        │         │ Match?  │
        │         └────┬────┘
        │        Yes   │  No
        │         ▼    │   ▼
        │        Use   │  Research Flow
        │        it    │     │
        │              │  ┌──┴──┐
        │              │  │Reusable?│
        │              │  └──┬──┘
        │              │ Yes │  No
        │              │  ▼  │   ▼
        │              │ Skill│ commit_to_memory
        │              │ Creation
        │              │  Flow
        ▼              ▼     ▼
      Resume task with knowledge
```
