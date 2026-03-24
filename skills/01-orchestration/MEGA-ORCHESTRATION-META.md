---
name: MEGA-ORCHESTRATION-META
description: Orchestration meta-skills - code navigation mastery, loop guard awareness, research synthesis, and gateway pattern tool selection.
domain: orchestration
triggers: code navigation, loop guard, research synthesis, gateway pattern, tool selection, codebase exploration
---

# MEGA-ORCHESTRATION-META

Meta-skills for AI agent orchestration: code navigation, loop detection, research synthesis, and intelligent tool routing.


---

<!-- merged from: code-navigation-mastery.md -->

# Code Navigation Mastery

## When to Use

- You need to find the definition or all usages of a specific function, struct, trait, or type
- You want to understand the dependency graph between modules before making changes
- You are entering an unfamiliar codebase and need a high-level map before diving in
- Grep returns too many false positives (e.g., searching for a common word like `state` or `error`)
- You need to find code related to a symbol but do not know which files contain it
- You are preparing a refactor and need to identify all downstream consumers of a function

## Decision Tree: Which Tool When

1. **"What does this codebase contain?"** -> `get_codebase_outline`
   - Returns top-level modules, public symbols, and file structure
   - Use as your first move in an unfamiliar project
   - Token-efficient: returns structure, not full source

2. **"Where is X defined and what is its signature?"** -> `resolve_symbol`
   - Input: symbol name (e.g., `handle_query`, `ToolRegistry`)
   - Returns: file path, line number, signature, and doc comments
   - Faster and more precise than grep for definitions

3. **"What depends on module Y / what does Y depend on?"** -> `module_graph`
   - Input: module path or file path
   - Returns: upstream (imports into Y) and downstream (Y imports from) edges
   - Essential before renaming, moving, or deleting a module

4. **"What other code is related to this symbol?"** -> `get_related_code`
   - Input: symbol name or file path
   - Returns: callers, callees, sibling symbols, and co-occurring patterns
   - Use when you need context beyond the definition itself

## Rules / Key Practices

- **Start broad, narrow down.** Begin with `get_codebase_outline` to orient, then use `resolve_symbol` for specifics. Do not jump straight to reading random files.
- **AST beats grep for structured queries.** If you need "all functions that return Result<Response>", AST tools filter by type signature. Grep would match string literals, comments, and docs too.
- **Grep beats AST for content search.** If you need "where is this error message string used", grep is the right tool. AST tools do not index string literals.
- **Combine tools in sequence.** A common pattern: `get_codebase_outline` -> identify module -> `module_graph` on that module -> `resolve_symbol` for key types -> `get_related_code` for callers.
- **Cache mental models.** After running `get_codebase_outline`, you know the project shape for the rest of the session. Do not re-run it unless files were added or removed.
- **Token efficiency matters.** `resolve_symbol` returns just the signature and location (tens of tokens). Reading the full file costs hundreds to thousands. Always resolve first, read second.
- **Qualify ambiguous names.** If `resolve_symbol("new")` returns too many hits, qualify it: `resolve_symbol("ToolRegistry::new")` or provide the module path hint.

## AST-Based vs Grep-Based Trade-offs

| Dimension | AST Tools | Grep |
|---|---|---|
| Precision for definitions | High (exact match on AST node) | Low (matches comments, strings) |
| Speed on large codebases | Fast (pre-indexed) | Slower (full scan) |
| String/content search | Not supported | Excellent |
| Cross-language support | Limited to indexed languages | Universal |
| Regex pattern matching | Not applicable | Full regex support |
| Token cost | Low (structured output) | Variable (can be very high) |

## Common Patterns

- **Pre-refactor audit:** `module_graph(target_module)` -> list all dependents -> `resolve_symbol` each public API used by dependents -> plan migration
- **Bug triage:** `resolve_symbol(error_function)` -> `get_related_code(error_function)` -> read callers to find where bad input originates
- **Onboarding:** `get_codebase_outline` -> pick the entry point module -> `module_graph(entry_module)` -> follow the dependency chain

## Checklist

- [ ] Run `get_codebase_outline` before exploring any new project area
- [ ] Use `resolve_symbol` instead of grep for finding definitions
- [ ] Check `module_graph` before moving or deleting any module
- [ ] Use `get_related_code` to find callers before changing a function signature
- [ ] Prefer qualified symbol names to avoid ambiguous results
- [ ] Only fall back to grep for string-literal or regex-pattern searches

## Reference

- Related: `skills/01-orchestration/gateway-pattern-tool-selection.md` for discovering these tools via the gateway
- Related: `skills/01-orchestration/advanced-search-techniques.md` for grep-based search patterns
- MCP server tools: `resolve_symbol`, `module_graph`, `get_codebase_outline`, `get_related_code`
- Full tool list: invoke `get_relevant_tools` with category "code_navigation"


---

<!-- merged from: loop-guard-awareness.md -->

# Loop Guard Awareness

## When to Use

- A tool call returns a loop guard error instead of the expected result
- You are designing an autonomous workflow that calls the same tool multiple times
- You need to configure the loop guard threshold for a specific use case
- You are debugging why a previously working tool call suddenly fails
- You want to understand why the MCP server blocks certain call patterns

## What Is the Loop Guard

The MCP server tracks tool invocations and blocks calls when it detects the same tool being called with the same arguments N times in a session. This prevents:

- **Infinite retry loops** where an agent keeps calling a failing tool with identical arguments
- **Token waste** from redundant calls that will return the same result
- **Resource abuse** from runaway autonomous agents hitting external APIs repeatedly

The guard compares both the tool name AND the argument payload. Changing either one resets the counter for that combination.

## How It Triggers

The guard maintains a counter per unique `(tool_name, arguments_hash)` pair:

1. First call with `(scan_secrets, {path: "/path/to/project"})` — counter = 1, executes normally
2. Second identical call — counter = 2, executes normally
3. ... continues until counter = N
4. Call N+1 — **BLOCKED**, returns loop guard error

The default threshold N is **5** (configurable via `MCP_LOOP_GUARD_THRESHOLD`).

## Configuring the Threshold

Set the environment variable before starting the MCP server:

```bash
export MCP_LOOP_GUARD_THRESHOLD=10  # Allow up to 10 identical calls
```

- **Lower values (2-3):** Stricter, catches loops faster. Good for production/CI where loops are always bugs.
- **Default (5):** Balanced. Allows reasonable retries while catching real loops.
- **Higher values (10-20):** Permissive. Use for batch processing workflows that legitimately poll the same resource.
- **Very high values (100+):** Effectively disables the guard. Not recommended.

## Patterns That Commonly Trigger the Guard

### 1. Retry Without Changing Arguments
```
# BAD: Same call 6 times because the first 5 "seemed slow"
invoke_tool({ tool_name: "fetch_web_markdown", arguments: { url: "https://example.com" } })
invoke_tool({ tool_name: "fetch_web_markdown", arguments: { url: "https://example.com" } })
... (blocked on attempt 6)
```
**Fix:** If a call fails, analyze the error. Do not blindly retry. If you must retry, add a distinguishing parameter or try an alternative approach.

### 2. Polling for State Changes
```
# BAD: Checking if a build finished by calling the same status tool repeatedly
invoke_tool({ tool_name: "get_workflow_state", arguments: { key: "build_status" } })
invoke_tool({ tool_name: "get_workflow_state", arguments: { key: "build_status" } })
... (blocked after 5 checks)
```
**Fix:** Add a timestamp or sequence number to arguments, or use a dedicated polling mechanism with backoff.

### 3. Querying Knowledge with the Same Keywords
```
# BAD: Searching for the same thing hoping for different results
query_knowledge({ query: "database migration" })
query_knowledge({ query: "database migration" })
... (blocked after 5)
```
**Fix:** Vary your query keywords. Try "schema migration", "db migrate steps", "migration rollback" instead of repeating the exact same query.

### 4. Scanning the Same Path Repeatedly
```
# BAD: Running scan_secrets on the same path after every small change
invoke_tool({ tool_name: "scan_secrets", arguments: { path: "/path/to/project" } })
... (blocked after 5 scans of the same path)
```
**Fix:** Scan specific changed files instead of the whole tree: `{ path: "/path/to/project/src/tools.rs" }`.

## Workaround Strategies

### Strategy 1: Vary the Arguments
Add or change a parameter that makes the call unique without changing semantics:
- Add a `context` or `reason` field if the tool accepts optional parameters
- Narrow the scope (scan a subdirectory instead of the root)
- Use different but equivalent query phrasings

### Strategy 2: Change Your Approach
If the same tool keeps being needed, the approach may be wrong:
- Instead of polling status 10 times, do other work and check once later
- Instead of re-querying knowledge, read the file directly
- Instead of re-fetching a URL, work with the content you already received

### Strategy 3: Break the Work Into Phases
For legitimate batch processing:
- Process items in groups with different scope parameters
- Use STATE.md to track progress and resume with different arguments
- Split large scans into per-directory or per-file scans

### Strategy 4: Adjust the Threshold
If your workflow legitimately needs more identical calls, increase `MCP_LOOP_GUARD_THRESHOLD`. But first ask: is there a better way to structure the workflow?

## Checklist

- [ ] If a tool call is blocked, check whether you are repeating identical arguments
- [ ] Vary query keywords instead of repeating the same search
- [ ] Narrow scan/search scope to specific files or directories
- [ ] Analyze errors before retrying; do not blindly repeat failed calls
- [ ] Use STATE.md to track progress in autonomous loops
- [ ] Only increase `MCP_LOOP_GUARD_THRESHOLD` as a last resort
- [ ] Design workflows to use different arguments for each iteration

## Reference

- CLAUDE.md Section 8: Constraints — "5+ turns without success -> Fresh Eyes"
- MCP server instruction: "Loop guard: same tool+args blocked after N calls (MCP_LOOP_GUARD_THRESHOLD, default 5)"
- Related: `skills/01-orchestration/gateway-pattern-tool-selection.md` for `invoke_tool` usage
- Related: `docs/setup/AGENTIC_OPERATOR_RULE.md` for full pipeline details and pitfalls


---

<!-- merged from: master-research-synthesis.md -->

# Master Research Synthesis

## When to Use

- You need to answer a question that requires current information beyond training data
- You are evaluating a library, API, or technology and need up-to-date docs and comparisons
- You want to build a persistent research document that can be queried in future sessions
- You need to verify a claim, best practice, or version compatibility against live sources
- The user asks about something released or changed after your knowledge cutoff
- You are preparing a technology evaluation, architecture decision, or vendor comparison

## Core Research Pipeline

### Phase 1: Search — Find Relevant Sources
```
search_web({ query: "Rust onnxruntime 2.0 migration guide" })
```
- Use specific, targeted queries (not vague ones like "Rust libraries")
- Include version numbers, dates, or specific terms to filter results
- Run 2-3 varied queries to cover different angles of the topic

### Phase 2: Fetch — Retrieve Full Content
```
fetch_web_markdown({ url: "https://example.com/migration-guide" })
```
- Fetch the top 2-4 results from search, not all of them
- Markdown conversion strips ads, nav, and boilerplate
- Check that the returned content is substantive before ingesting

### Phase 3: Ingest — Store in Knowledge Base
```
ingest_web_context({ url: "https://...", content: "...", topic: "onnxruntime-2.0-migration" })
```
- Always provide a clear, specific topic label for retrieval
- Content is chunked and embedded for semantic search
- Only ingest information that is better or newer than what you already have
- Do not ingest entire pages of irrelevant content; extract the relevant sections

### Phase 4: Synthesize — Build Master Document
After ingesting multiple sources, the knowledge base contains fragments. To create a coherent master research document:
- Query the ingested content with `query_knowledge` using several angles
- Combine findings into a structured synthesis (comparisons, trade-offs, recommendations)
- Use `commit_to_memory` to persist the synthesized conclusions

### Phase 5: Query — Retrieve Later
```
query_master_research({ topic: "onnxruntime-2.0-migration", question: "What broke between 1.x and 2.0?" })
```
- Queries the synthesized research, not raw web pages
- More precise than general `query_knowledge` for research topics
- Returns relevant sections with source attribution

## Automated vs Manual Research

### `research_and_verify` — The Automated Path
Use when:
- The question is straightforward and factual ("What is the latest version of X?")
- You need a quick answer with source verification
- The topic does not require deep multi-source synthesis

What it does: Searches the web, fetches top results, cross-references claims, and returns a verified answer with sources. One tool call replaces the entire Phase 1-3 pipeline.

### Manual `search_web` + `fetch_web_markdown` + `ingest_web_context` — The Controlled Path
Use when:
- You need to compare multiple competing sources or opinions
- The topic is nuanced and requires selecting which content to ingest
- You want to build a persistent master research document for future sessions
- The automated path returned shallow or incomplete results
- You need to fetch specific URLs the user provided

## Maintaining Freshness

- **Check dates.** Before trusting ingested research, verify when it was captured. Technology moves fast.
- **Re-research periodically.** If a topic was last researched months ago, run fresh searches before relying on cached results.
- **Layer new over old.** When re-researching, ingest with the same topic label. New content supplements (does not replace) existing content. Query results will reflect both.
- **Flag staleness.** If `query_master_research` returns results that reference old versions or deprecated APIs, note this and trigger a fresh research cycle.

## Quality Controls

- **Triangulate claims.** Never trust a single source. Verify key facts across 2-3 sources before ingesting.
- **Prefer primary sources.** Official docs > blog posts > Stack Overflow answers > LLM-generated content.
- **Note conflicts.** When sources disagree, ingest both viewpoints and flag the conflict in synthesis.
- **Attribute sources.** Always record the URL and approximate date with ingested content.

## Checklist

- [ ] Start with 2-3 targeted `search_web` queries covering different angles
- [ ] Fetch only substantive, relevant results (not every search hit)
- [ ] Use clear, consistent topic labels when ingesting
- [ ] Ingest only new/better information, not duplicates of what exists
- [ ] Use `research_and_verify` for quick factual lookups
- [ ] Use the manual pipeline for deep synthesis and persistent research
- [ ] Triangulate claims across multiple sources before committing
- [ ] Check freshness of existing research before relying on it

## Reference

- CLAUDE.md Section 3.1: RESEARCH step in the pipeline
- Related: `skills/01-orchestration/search-and-fetch-standard.md` for search best practices
- Related: `skills/01-orchestration/gateway-pattern-tool-selection.md` for discovering research tools
- MCP tools: `search_web`, `fetch_web_markdown`, `ingest_web_context`, `query_master_research`, `research_and_verify`


---

<!-- merged from: gateway-pattern-tool-selection.md -->

# Gateway Pattern Tool Selection

## When to Use

- You need an MCP tool beyond the 3 defaults (`query_knowledge`, `get_relevant_tools`, `invoke_tool`)
- You are unsure which tool handles a specific task (e.g., "how do I scan for secrets?")
- You want to minimize token usage by not loading all tool schemas into context
- You are building a multi-step workflow and need to chain several specialized tools
- A direct tool call fails or is unavailable and you need the gateway fallback

## The Gateway Architecture

The MCP server exposes 43+ tools but only 3 are loaded into the LLM context by default:

1. **`query_knowledge`** — RAG retrieval from the knowledge base (RECALL step)
2. **`get_relevant_tools`** — Discovery: describe what you need, get matching tool names and schemas
3. **`invoke_tool`** — Execution: call any tool by name with JSON arguments

This design saves thousands of tokens per turn. Loading all 43 tool schemas would consume ~4000-6000 tokens of context on every message. The gateway pattern loads only what you need, when you need it.

## Full Workflow: Query -> Discover -> Validate -> Invoke -> Verify

### Step 1: Identify the Need
Determine what capability you need. Examples: "scan for secrets", "get file history", "build module graph".

### Step 2: Discover with `get_relevant_tools`
```
get_relevant_tools({ query: "scan secrets security", include_descriptions: true })
```
- Use 2-4 keywords describing the capability
- Set `include_descriptions: true` when you need to understand what each tool does (costs more tokens but avoids wrong tool selection)
- Set `include_descriptions: false` (default) when you just need the tool name and already know what it does

### Step 3: Validate the Match
Read the returned tool description and parameter schema. Confirm:
- The tool does what you expect
- You have all required parameters
- The parameter types match your data

### Step 4: Invoke with `invoke_tool`
```
invoke_tool({ tool_name: "scan_secrets", arguments: { path: "/path/to/project" } })
```
- Pass the exact tool name from Step 2
- Arguments must be a JSON object matching the tool's schema
- Do not guess parameter names; use the schema from discovery

### Step 5: Verify the Result
Check the return value. If it indicates an error:
- Re-read the tool schema for correct parameter format
- Try with adjusted arguments
- If the tool itself is wrong, go back to Step 2 with different keywords

## Gateway Mode vs Direct Tool Calls

| Scenario | Use Gateway | Use Direct |
|---|---|---|
| Tool is one of the 3 defaults | No | Yes — `query_knowledge`, `get_relevant_tools`, `invoke_tool` |
| Tool is used once in a session | Yes — discover and invoke | No — not worth loading schema permanently |
| Tool is used repeatedly (5+ times) | Consider requesting `MCP_FULL_TOOLS=1` | Yes, if available in context |
| You do not know the tool name | Yes — discovery is the whole point | Not possible |
| Token budget is tight | Yes — gateway saves ~4000 tokens | No |

## Token Budget Optimization

- **Do not discover tools you already know.** If you used `scan_secrets` earlier in the session, skip `get_relevant_tools` and go straight to `invoke_tool`.
- **Batch related discoveries.** If you need 3 tools for a workflow, run `get_relevant_tools` once with a broad query rather than 3 separate calls.
- **Use `include_descriptions: false`** when you just need to confirm a tool exists.
- **Cache tool names mentally.** After the first discovery, remember the tool name for the rest of the session.

## Common Discovery Queries

| Need | Query Keywords |
|---|---|
| File change tracking | "file history git changes" |
| Secret detection | "scan secrets security credentials" |
| Code structure | "codebase outline module structure" |
| Web research | "search web fetch markdown" |
| Memory persistence | "commit memory store knowledge" |
| Diff review | "review diff changes code" |
| Error diagnosis | "analyze error log debug" |

## Checklist

- [ ] Always use `get_relevant_tools` before guessing a tool name
- [ ] Include `include_descriptions: true` on first use of an unfamiliar tool
- [ ] Pass arguments as a proper JSON object to `invoke_tool`
- [ ] Verify the tool's return value before proceeding
- [ ] Do not re-discover tools you already used in the same session
- [ ] Prefer gateway mode over `MCP_FULL_TOOLS=1` unless you need many tools repeatedly

## Reference

- CLAUDE.md Section 2: Gateway Pattern definition
- Related: `skills/01-orchestration/code-navigation-mastery.md` for navigation tools discovered via gateway
- Related: `skills/01-orchestration/loop-guard-awareness.md` for when repeated `invoke_tool` calls get blocked
- MCP server instruction: "Default is minimal (5 tools) for token savings; set MCP_FULL_TOOLS=1 for full list"