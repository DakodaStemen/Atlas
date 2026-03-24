---
name: "MEGA-AI-TOOLS (Part 1)"
description: Consolidated AI tool patterns - ChatGPT Apps SDK, image generation (imagegen), video generation (Sora), screenshot capture, tool use concepts/TypeScript, CLI references, web game development, search-and-fetch standard.
domain: ai-engineering
triggers: chatgpt apps, imagegen, sora, screenshot, tool use, openai api, image generation, video generation, web game, search fetch
---


# MEGA-AI-TOOLS

Consolidated AI tool integration patterns: ChatGPT Apps SDK, OpenAI image/video generation, screenshot capture, tool-use API patterns, CLI references, and development workflows.


---

<!-- merged from: chatgpt-apps.md -->

﻿---
name: chatgpt-apps
description: Build, scaffold, refactor, and troubleshoot ChatGPT Apps SDK applications that combine an MCP server and widget UI. Use when Codex needs to design tools, register UI resources, wire the MCP Apps bridge or ChatGPT compatibility APIs, apply Apps SDK metadata or CSP or domain settings, or produce a docs-aligned project scaffold. Prefer a docs-first workflow by invoking the openai-docs skill or OpenAI developer docs MCP tools before generating code.
---

# ChatGPT Apps

## Overview

Scaffold ChatGPT Apps SDK implementations with a docs-first, example-first workflow, then generate code that follows current Apps SDK and MCP Apps bridge patterns.

Use this skill to produce:

- A primary app-archetype classification and repo-shape decision
- A tool plan (names, schemas, annotations, outputs)
- An upstream starting-point recommendation (official example, ext-apps example, or local fallback scaffold)
- An MCP server scaffold (resource registration, tool handlers, metadata)
- A widget scaffold (MCP Apps bridge first, `window.openai` compatibility/extensions second)
- A reusable Node + `@modelcontextprotocol/ext-apps` starter scaffold for low-dependency fallbacks
- A validation report against the minimum working repo contract
- Local dev and connector setup steps
- A short stakeholder summary of what the app does (when requested)

## Mandatory Docs-First Workflow

Use `$openai-docs` first whenever building or changing a ChatGPT Apps SDK app.

1. Invoke `$openai-docs` (preferred) or call the OpenAI docs MCP server directly.
2. Fetch current Apps SDK docs before writing code, especially (baseline pages):
   - `apps-sdk/build/mcp-server`
   - `apps-sdk/build/chatgpt-ui`
   - `apps-sdk/build/examples`
   - `apps-sdk/plan/tools`
   - `apps-sdk/reference`
3. Fetch `apps-sdk/quickstart` when scaffolding a new app or generating a first-pass implementation, and check the official examples repo/page before inventing a scaffold from scratch.
4. Fetch deployment/submission docs when the task includes local ChatGPT testing, hosting, or public launch:
   - `apps-sdk/deploy`
   - `apps-sdk/deploy/submission`
   - `apps-sdk/app-submission-guidelines`
5. Cite the docs URLs you used when explaining design choices or generated scaffolds.
6. Prefer current docs guidance over older repo patterns when they differ, and call out compatibility aliases explicitly.
7. If doc search times out or returns poor matches, fetch the canonical Apps SDK pages directly by URL and continue; do not let search failure block scaffolding.

If `$openai-docs` is unavailable, use:

- `mcp__openaiDeveloperDocs__search_openai_docs`
- `mcp__openaiDeveloperDocs__fetch_openai_doc`

Read `references/apps-sdk-docs-workflow.md` for suggested doc queries and a compact checklist.
Read `references/app-archetypes.md` to classify the request into a small number of supported app shapes before choosing examples or scaffolds.
Read `references/repo-contract-and-validation.md` when generating or reviewing a repo so the output stays inside a stable “working app” contract.
Read `references/search-fetch-standard.md` when the app is connector-like, data-only, sync-oriented, or meant to work well with company knowledge or deep research.
Read `references/upstream-example-workflow.md` when starting a greenfield app or when deciding whether to adapt an upstream example or use the local fallback scaffold.
Read `references/window-openai-patterns.md` when the task needs ChatGPT-specific widget behavior or when translating repo examples that use wrapper-specific `app.*` helpers.

## Prompt Guidance

Use prompts that explicitly pair this skill with `$openai-docs` so the resulting scaffold is grounded in current docs.

Preferred prompt patterns:

- `Use $chatgpt-apps with $openai-docs to scaffold a ChatGPT app for <use case> with a <TS/Python> MCP server and <React/vanilla> widget.`
- `Use $chatgpt-apps with $openai-docs to adapt the closest official Apps SDK example into a ChatGPT app for <use case>.`
- `Use $chatgpt-apps and $openai-docs to refactor this Apps SDK demo into a production-ready structure with tool annotations, CSP, and URI versioning.`
- `Use $chatgpt-apps with $openai-docs to plan tools first, then generate the MCP server and widget code.`

When responding, ask for or infer these inputs before coding:

- Use case and primary user flows
- Read-only vs mutating tools
- Demo vs production target
- Private/internal use vs public directory submission
- Backend language and UI stack
- Auth requirements
- External API domains for CSP allowlists
- Hosting target and local dev approach
- Org ownership/verification readiness (for submission tasks)

## Classify The App Before Choosing Code

Before choosing examples, repo shape, or scaffolds, classify the request into one primary archetype and state it.

- `tool-only`
- `vanilla-widget`
- `react-widget`
- `interactive-decoupled`
- `submission-ready`

Infer the archetype unless a missing detail is truly blocking. Use the archetype to choose:

- whether a UI is needed at all
- whether to preserve a split `server/` + `web/` layout
- whether to prefer official OpenAI examples, ext-apps examples, or the local fallback scaffold
- which validation checks matter most
- whether `search` and `fetch` should be the default read-only tool surface

Read `references/app-archetypes.md` for the decision rubric.

## Default Starting-Point Order

For greenfield apps, prefer these starting points in order:

1. **Official OpenAI examples** when a close example already matches the requested stack or interaction pattern.
2. **Version-matched `@modelcontextprotocol/ext-apps` examples** when the user needs a lower-level or more portable MCP Apps baseline.
3. **`scripts/scaffold_node_ext_apps.mjs`** only when no close example fits, the user wants a tiny Node + vanilla starter, or network access/example retrieval is undesirable.

Do not generate a large custom scaffold from scratch if a close upstream example already exists.
Copy the smallest matching example, remove unrelated demo code, then patch it to the current docs and the user request.

## Build Workflow

### 0. Classify The App Archetype

Pick one primary archetype before planning tools or choosing a starting point.

- Prefer a single primary archetype instead of mixing several.
- If the request is broad, infer the smallest archetype that can still satisfy it.
- Escalate to `submission-ready` only when the user asks for public launch, directory submission, or review-ready deployment.
- Call out the chosen archetype in your response so the user can correct it early if needed.

### 1. Plan Tools Before Code

Define the tool surface area from user intents.

- Use one job per tool.
- Write tool descriptions that start with "Use this when..." behavior cues.
- Make inputs explicit and machine-friendly (enums, required fields, bounds).
- Decide whether each tool is data-only, render-only, or both.
- Set annotations accurately (`readOnlyHint`, `destructiveHint`, `openWorldHint`; add `idempotentHint` when true).
- If the app is connector-like, data-only, sync-oriented, or intended for company knowledge or deep research, default to the standard `search` and `fetch` tools instead of inventing custom read-only equivalents.
- For educational/demo apps, prefer one concept per tool so the model can pick the right example cleanly.
- Group demo tools by learning objective: data into the widget, widget actions back into the conversation or tools, host/layout environment signals, and lifecycle/streaming behavior.

Read `references/search-fetch-standard.md` when `search` and `fetch` may be relevant.

### 2. Choose an App Architecture

Choose the simplest structure that fits the goal.

- Use a **minimal demo pattern** for quick prototypes, workshops, or proofs of concept.
- Use a **decoupled data/render pattern** for production UX so the widget does not re-render on every tool call.

Prefer the decoupled pattern for non-trivial apps:

- Data tools return reusable `structuredContent`.
- Render tools attach `_meta.ui.resourceUri` and optional `_meta["openai/outputTemplate"]`.
- Render tool descriptions state prerequisites (for example, "Call `search` first").

### 2a. Start From An Upstream Example When One Fits

Default to upstream examples for greenfield work when they are close to the requested app.

- Check the official OpenAI examples first for ChatGPT-facing apps, polished UI patterns, React components, file upload flows, modal flows, or apps that resemble the docs examples.
- Use `@modelcontextprotocol/ext-apps` examples when the request is closer to raw MCP Apps bridge/server wiring, or when version-matched package patterns matter more than ChatGPT-specific polish.
- Pick the smallest matching example and copy only the relevant files; do not transplant an entire showcase app unchanged.
- After copying, reconcile the example with the current docs you fetched: tool names/descriptions, annotations, `_meta.ui.*`, CSP, URI versioning, and local run instructions.
- State which example you chose and why in one sentence.

Read `references/upstream-example-workflow.md` for the selection and adaptation rubric.

### 2b. Use the Starter Script When a Low-Dependency Fallback Helps

Use `scripts/scaffold_node_ext_apps.mjs` only when the user wants a quick, greenfield Node starter and a vanilla HTML widget is acceptable, and no upstream example is a better starting point.

- Run it only after fetching current docs, then reconcile the generated files with the docs you fetched.
- If you choose the script instead of an upstream example, say why the fallback is better for that request.
- Skip it when a close official example exists, when the user already has an existing app structure, when they need a non-Node stack, when they explicitly want React first, or when they only want a plan/review instead of code.
- The script generates a minimal `@modelcontextprotocol/ext-apps` server plus a vanilla HTML widget that uses the MCP Apps bridge by default.
- The generated widget keeps follow-up messaging on the standard `ui/message` bridge and only uses `window.openai` for optional host signals/extensions.
- After running it, patch the generated output to match the current docs and the user request: adjust tool names/descriptions, annotations, resource metadata, URI versioning, and README/run instructions.

### 3. Scaffold the MCP Server

Generate a server that:

- Registers a widget resource/template with the MCP Apps UI MIME type (`text/html;profile=mcp-app`) or the SDK constant (`RESOURCE_MIME_TYPE`) when using `@modelcontextprotocol/ext-apps/server`
- Registers tools with clear names, schemas, titles, and descriptions
- Returns `structuredContent` (model + widget), `content` (model narration), and `_meta` (widget-only data) intentionally
- Keeps handlers idempotent or documents non-idempotent behavior explicitly
- Includes tool status strings (`openai/toolInvocation/*`) when helpful in ChatGPT

Keep `structuredContent` concise. Move large or sensitive widget-only payloads to `_meta`.

### 4. Scaffold the Widget UI

Use the MCP Apps bridge first for portability, then add ChatGPT-specific `window.openai` APIs when they materially improve UX.

- Listen for `ui/notifications/tool-result` (JSON-RPC over `postMessage`)
- Render from `structuredContent`
- Use `tools/call` for component-initiated tool calls
- Use `ui/update-model-context` only when UI state should change what the model sees

Use `window.openai` for compatibility and extensions (file upload, modal, display mode, etc.), not as the only integration path for new apps.

#### API Surface Guardrails

- Some examples wrap the bridge with an `app` object (for example, `@modelcontextprotocol/ext-apps/react`) and expose helper names like `app.sendMessage()`, `app.callServerTool()`, `app.openLink()`, or host getter methods.
- Treat those wrappers as implementation details or convenience layers, not the canonical public API to teach by default.
- For ChatGPT-facing guidance, prefer the current documented surface: `window.openai.callTool(...)`, `window.openai.sendFollowUpMessage(...)`, `window.openai.openExternal(...)`, `window.openai.requestDisplayMode(...)`, and direct globals like `window.openai.theme`, `window.openai.locale`, `window.openai.displayMode`, `window.openai.toolInput`, `window.openai.toolOutput`, `window.openai.toolResponseMetadata`, and `window.openai.widgetState`.
- If you reference wrapper helpers from repo examples, map them back to the documented `window.openai` or MCP Apps bridge primitives and call out that the wrapper is not the normative API surface.
- Use `references/window-openai-patterns.md` for the wrapper-to-canonical mapping and for React helper extraction patterns.

### 5. Add Resource Metadata and Security

Set resource metadata deliberately on the widget resource/template:

- `_meta.ui.csp` with exact `connectDomains` and `resourceDomains`
- `_meta.ui.domain` for app submission-ready deployments
- `_meta.ui.prefersBorder` (or OpenAI compatibility alias when needed)
- Optional `openai/widgetDescription` to reduce redundant narration

Avoid `frameDomains` unless iframe embeds are core to the product.

### 5a. Enforce A Minimum Working Repo Contract

Every generated repo should satisfy a small, stable contract before you consider it done.

- The repo shape matches the chosen archetype.
- The MCP server and tools are wired to a reachable `/mcp` endpoint.
- Tools have clear descriptions, accurate annotations, and UI metadata where needed.
- Connector-like, data-only, sync-oriented, and company-knowledge-style apps use the standard `search` and `fetch` tool shapes when relevant.
- The widget uses the MCP Apps bridge correctly when a UI exists.
- The repo includes enough scripts or commands for a user to run and check it locally.
- The response explicitly says what validation was run and what was not run.

Read `references/repo-contract-and-validation.md` for the detailed checklist and validation ladder.

### 6. Validate the Local Loop

Validate against the minimum working repo contract, not just “did files get created.”

- Run the lowest-cost checks first:
  - static contract review
  - syntax or compile checks when feasible
  - local `/mcp` health check when feasible
- Then move up to runtime checks:
  - verify tool descriptors and widget rendering in MCP Inspector
  - test the app in ChatGPT developer mode through HTTPS tunneling
  - exercise retries and repeated tool calls to confirm idempotent behavior
  - check widget updates after host events and follow-up tool calls
- If you are only delivering a scaffold and are not installing dependencies, still run low-cost checks and say exactly what you did not run.

Read `references/repo-contract-and-validation.md` for the validation ladder.

### 7. Connect and Test in ChatGPT (Developer Mode)

For local development, include explicit ChatGPT setup steps (not just code/run commands).

- Run the MCP server locally on `http://localhost:<port>/mcp`
- Expose the local server with a public HTTPS tunnel (for example `ngrok http <port>`)
- Use the tunneled HTTPS URL plus `/mcp` path when connecting from ChatGPT
- In ChatGPT, enable Developer Mode under **Settings → Apps & Connectors → Advanced settings**
- In ChatGPT app settings, create a new app for the remote MCP server and paste the public MCP URL
- Tell users to refresh the app after MCP tool/metadata changes so ChatGPT reloads the latest descriptors

Note: Some docs/screenshots still use older "connector" terminology. Prefer current product wording ("app") while acknowledging both labels when giving step-by-step instructions.

### 8. Plan Production Hosting and Deployment

When the user asks to deploy or prepare for launch, generate hosting guidance for the MCP server (and widget assets if hosted separately).

- Host behind a stable public HTTPS endpoint (not a tunnel) with dependable TLS
- Preserve low-latency streaming behavior on `/mcp`
- Configure secrets outside the repo (environment variables / secret manager)
- Add logging, request latency tracking, and error visibility for tool calls
- Add basic observability (CPU, memory, request volume) and a troubleshooting path
- Re-test the hosted endpoint in ChatGPT Developer Mode before submission

### 9. Prepare Submission and Publish (Public Apps Only)

Only include these steps when the user intends a public directory listing.

- Use `apps-sdk/deploy/submission` for the submission flow and `apps-sdk/app-submission-guidelines` for review requirements
- Keep private/internal apps in Developer Mode instead of submitting
- Confirm org verification and Owner-role prerequisites before submission work
- Ensure the MCP server uses a public production endpoint (no localhost/testing URLs) and has submission-ready CSP configured
- Prepare submission artifacts: app metadata, logo/screenshots, privacy policy URL, support contact, test prompts/responses, localization info
- If auth is required, include review-safe demo credentials and test the login path end-to-end
- Submit for review in the Platform dashboard, monitor review status, and publish only after approval

## Interactive State Guidance

Read `references/interactive-state-sync-patterns.md` when the app has long-lived widget state, repeated interactions, or component-initiated tool calls (for example, games, boards, maps, dashboards, editors).

Use it to choose patterns for:

- State snapshots plus monotonic event tokens (`stateVersion`, `resetCount`, etc.)
- Idempotent retry-safe handlers
- `structuredContent` vs `_meta` partitioning
- MCP Apps bridge-first update flows with optional `window.openai` compatibility
- Decoupled data/render tool architecture for more complex interactive apps

## Output Expectations

When using this skill to scaffold code, produce output in this order unless the user asks otherwise:

- For direct scaffold requests, do not stop at the plan: give the brief plan, then create the files immediately.

1. Primary app archetype chosen and why
2. Tool plan and architecture choice (minimal vs decoupled)
3. Upstream starting point chosen (official example, ext-apps example, or local fallback scaffold) and why
4. Doc pages/URLs used from `$openai-docs`
5. File tree to create or modify
6. Implementation (server + widget)
7. Validation performed against the minimum working repo contract
8. Local run/test instructions (including tunnel + ChatGPT Developer Mode app setup)
9. Deployment/hosting guidance (if requested or implied)
10. Submission-readiness checklist (for public launch requests)
11. Risks, gaps, and follow-up improvements

## References

- `references/app-archetypes.md` for classifying requests into a small number of supported app shapes
- `references/apps-sdk-docs-workflow.md` for doc queries, page targets, and code-generation checklist
- `references/interactive-state-sync-patterns.md` for reusable patterns for stateful or highly interactive widget apps
- `references/repo-contract-and-validation.md` for the minimum working repo contract and lightweight validation ladder
- `references/search-fetch-standard.md` for when and how to default to the standard `search` and `fetch` tools
- `references/upstream-example-workflow.md` for choosing between official examples, ext-apps examples, and the local fallback scaffold
- `references/window-openai-patterns.md` for ChatGPT-specific extensions, wrapper API translation, and React helper patterns
- `scripts/scaffold_node_ext_apps.mjs` for a minimal Node + `@modelcontextprotocol/ext-apps` fallback starter scaffold


---

<!-- merged from: imagegen.md -->

﻿---
name: "imagegen"
description: "Use when the user asks to generate or edit images via the OpenAI Image API (for example: generate image, edit/inpaint/mask, background removal or replacement, transparent background, product shots, concept art, covers, or batch variants); run the bundled CLI (`scripts/image_gen.py`) and require `OPENAI_API_KEY` for live calls."
---

# Image Generation Skill

Generates or edits images for the current project (e.g., website assets, game assets, UI mockups, product mockups, wireframes, logo design, photorealistic images, infographics). Defaults to `gpt-image-1.5` and the OpenAI Image API, and prefers the bundled CLI for deterministic, reproducible runs.


---

<!-- merged from: sora.md -->

﻿---
name: "sora"
description: "Use when the user asks to generate, remix, poll, list, download, or delete Sora videos via OpenAI\u2019s video API using the bundled CLI (`scripts/sora.py`), including requests like \u201cgenerate AI video,\u201d \u201cSora,\u201d \u201cvideo remix,\u201d \u201cdownload video/thumbnail/spritesheet,\u201d and batch video generation; requires `OPENAI_API_KEY` and Sora API access."
---

# Sora Video Generation Skill

Creates or manages short video clips for the current project (product demos, marketing spots, cinematic shots, UI mocks). Defaults to `sora-2` and a structured prompt augmentation workflow, and prefers the bundled CLI for deterministic runs. Note: `$sora` is a skill tag in prompts, not a shell command.


---

<!-- merged from: screenshot.md -->

﻿---
name: "screenshot"
description: "Use when the user explicitly asks for a desktop or system screenshot (full screen, specific app or window, or a pixel region), or when tool-specific capture capabilities are unavailable and an OS-level capture is needed."
---

# Screenshot Capture

Follow these save-location rules every time:

1) If the user specifies a path, save there.
2) If the user asks for a screenshot without a path, save to the OS default screenshot location.
3) If Codex needs a screenshot for its own inspection, save to the temp directory.

## Tool priority

- Prefer tool-specific screenshot capabilities when available (for example: a Figma MCP/skill for Figma files, or Playwright/agent-browser tools for browsers and Electron apps).
- Use this skill when explicitly asked, for whole-system desktop captures, or when a tool-specific capture cannot get what you need.
- Otherwise, treat this skill as the default for desktop apps without a better-integrated capture tool.

## macOS permission preflight (reduce repeated prompts)

On macOS, run the preflight helper once before window/app capture. It checks
Screen Recording permission, explains why it is needed, and requests it in one
place.

The helpers route Swift's module cache to `$TMPDIR/codex-swift-module-cache`
to avoid extra sandbox module-cache prompts.

```bash
bash <path-to-skill>/scripts/ensure_macos_permissions.sh
```

To avoid multiple sandbox approval prompts, combine preflight + capture in one
command when possible:

```bash
bash <path-to-skill>/scripts/ensure_macos_permissions.sh && \
python3 <path-to-skill>/scripts/take_screenshot.py --app "Codex"
```

For Codex inspection runs, keep the output in temp:

```bash
bash <path-to-skill>/scripts/ensure_macos_permissions.sh && \
python3 <path-to-skill>/scripts/take_screenshot.py --app "<App>" --mode temp
```

Use the bundled scripts to avoid re-deriving OS-specific commands.

## macOS and Linux (Python helper)

Run the helper from the repo root:

```bash
python3 <path-to-skill>/scripts/take_screenshot.py
```

Common patterns:

- Default location (user asked for "a screenshot"):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py
```

- Temp location (Codex visual check):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py --mode temp
```

- Explicit location (user provided a path or filename):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py --path output/screen.png
```

- App/window capture by app name (macOS only; substring match is OK; captures all matching windows):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py --app "Codex"
```

- Specific window title within an app (macOS only):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py --app "Codex" --window-name "Settings"
```

- List matching window ids before capturing (macOS only):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py --list-windows --app "Codex"
```

- Pixel region (x,y,w,h):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py --mode temp --region 100,200,800,600
```

- Focused/active window (captures only the frontmost window; use `--app` to capture all windows):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py --mode temp --active-window
```

- Specific window id (use --list-windows on macOS to discover ids):

```bash
python3 <path-to-skill>/scripts/take_screenshot.py --window-id 12345
```

The script prints one path per capture. When multiple windows or displays match, it prints multiple paths (one per line) and adds suffixes like `-w<windowId>` or `-d<display>`. View each path sequentially with the image viewer tool, and only manipulate images if needed or requested.

### Workflow examples

- "Take a look at <App> and tell me what you see": capture to temp, then view each printed path in order.

```bash
bash <path-to-skill>/scripts/ensure_macos_permissions.sh && \
python3 <path-to-skill>/scripts/take_screenshot.py --app "<App>" --mode temp
```

- "The design from Figma is not matching what is implemented": use a Figma MCP/skill to capture the design first, then capture the running app with this skill (typically to temp) and compare the raw screenshots before any manipulation.

### Multi-display behavior

- On macOS, full-screen captures save one file per display when multiple monitors are connected.
- On Linux and Windows, full-screen captures use the virtual desktop (all monitors in one image); use `--region` to isolate a single display when needed.

### Linux prerequisites and selection logic

The helper automatically selects the first available tool:

1) `scrot`
2) `gnome-screenshot`
3) ImageMagick `import`

If none are available, ask the user to install one of them and retry.

Coordinate regions require `scrot` or ImageMagick `import`.

`--app`, `--window-name`, and `--list-windows` are macOS-only. On Linux, use
`--active-window` or provide `--window-id` when available.

## Windows (PowerShell helper)

Run the PowerShell helper:

```powershell
powershell -ExecutionPolicy Bypass -File <path-to-skill>/scripts/take_screenshot.ps1
```

Common patterns:

- Default location:

```powershell
powershell -ExecutionPolicy Bypass -File <path-to-skill>/scripts/take_screenshot.ps1
```

- Temp location (Codex visual check):

```powershell
powershell -ExecutionPolicy Bypass -File <path-to-skill>/scripts/take_screenshot.ps1 -Mode temp
```

- Explicit path:

```powershell
powershell -ExecutionPolicy Bypass -File <path-to-skill>/scripts/take_screenshot.ps1 -Path "C:\Temp\screen.png"
```

- Pixel region (x,y,w,h):

```powershell
powershell -ExecutionPolicy Bypass -File <path-to-skill>/scripts/take_screenshot.ps1 -Mode temp -Region 100,200,800,600
```

- Active window (ask the user to focus it first):

```powershell
powershell -ExecutionPolicy Bypass -File <path-to-skill>/scripts/take_screenshot.ps1 -Mode temp -ActiveWindow
```

- Specific window handle (only when provided):

```powershell
powershell -ExecutionPolicy Bypass -File <path-to-skill>/scripts/take_screenshot.ps1 -WindowHandle 123456
```

## Direct OS commands (fallbacks)

Use these when you cannot run the helpers.

### macOS

- Full screen to a specific path:

```bash
screencapture -x output/screen.png
```

- Pixel region:

```bash
screencapture -x -R100,200,800,600 output/region.png
```

- Specific window id:

```bash
screencapture -x -l12345 output/window.png
```

- Interactive selection or window pick:

```bash
screencapture -x -i output/interactive.png
```

### Linux

- Full screen:

```bash
scrot output/screen.png
```

```bash
gnome-screenshot -f output/screen.png
```

```bash
import -window root output/screen.png
```

- Pixel region:

```bash
scrot -a 100,200,800,600 output/region.png
```

```bash
import -window root -crop 800x600+100+200 output/region.png
```

- Active window:

```bash
scrot -u output/window.png
```

```bash
gnome-screenshot -w -f output/window.png
```

## Error handling

- On macOS, run `bash <path-to-skill>/scripts/ensure_macos_permissions.sh` first to request Screen Recording in one place.
- If you see "screen capture checks are blocked in the sandbox", "could not create image from display", or Swift `ModuleCache` permission errors in a sandboxed run, rerun the command with escalated permissions.
- If macOS app/window capture returns no matches, run `--list-windows --app "AppName"` and retry with `--window-id`, and make sure the app is visible on screen.
- If Linux region/window capture fails, check tool availability with `command -v scrot`, `command -v gnome-screenshot`, and `command -v import`.
- If saving to the OS default location fails with permission errors in a sandbox, rerun the command with escalated permissions.
- Always report the saved file path in the response.


---

<!-- merged from: cli-reference-scriptsimagegenpy.md -->

﻿---
name: CLI reference (`scripts/image_gen.py`)
description: # CLI reference (`scripts/image_gen.py`)
 
 This file contains the “command catalog” for the bundled image generation CLI. Keep `SKILL.md` as overview-first; put verbose CLI details here.
---

# CLI reference (`scripts/image_gen.py`)

This file contains the “command catalog” for the bundled image generation CLI. Keep `SKILL.md` as overview-first; put verbose CLI details here.

## What this CLI does

- `generate`: generate new images from a prompt
- `edit`: edit an existing image (optionally with a mask) — inpainting / background replacement / “change only X”
- `generate-batch`: run many jobs from a JSONL file (one job per line)

Real API calls require **network access** + `OPENAI_API_KEY`. `--dry-run` does not.

## Quick start (works from any repo)

Set a stable path to the skill CLI (default `CODEX_HOME` is `~/.codex`):

```bash
export CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
export IMAGE_GEN="$CODEX_HOME/skills/imagegen/scripts/image_gen.py"
```

Dry-run (no API call; no network required; does not require the `openai` package):

```bash
python "$IMAGE_GEN" generate --prompt "Test" --dry-run
```

Generate (requires `OPENAI_API_KEY` + network):

```text
uv run --with openai python "$IMAGE_GEN" generate --prompt "A cozy alpine cabin at dawn" --size 1024x1024
```

No `uv` installed? Use your active Python env:

```python
python "$IMAGE_GEN" generate --prompt "A cozy alpine cabin at dawn" --size 1024x1024
```

## Guardrails (important)

- Use `python "$IMAGE_GEN" ...` (or equivalent full path) for generations/edits/batch work.
- Do **not** create one-off runners (e.g. `gen_images.py`) unless the user explicitly asks for a custom wrapper.
- **Never modify** `scripts/image_gen.py`. If something is missing, ask the user before doing anything else.

## Defaults (unless overridden by flags)

- Model: `gpt-image-1.5`
- Size: `1024x1024`
- Quality: `auto`
- Output format: `png`
- Background: unspecified (API default). If you set `--background transparent`, also set `--output-format png` or `webp`.

## Quality + input fidelity

- `--quality` works for `generate`, `edit`, and `generate-batch`: `low|medium|high|auto`.
- `--input-fidelity` is **edit-only**: `low|high` (use `high` for strict edits like identity or layout lock).

Example:

```text
python "$IMAGE_GEN" edit --image input.png --prompt "Change only the background" --quality high --input-fidelity high
```

## Masks (edits)

- Use a **PNG** mask; an alpha channel is strongly recommended.
- The mask should match the input image dimensions.
- In the edit prompt, repeat invariants (e.g., “change only the background; keep the subject unchanged”) to reduce drift.

## Optional deps

Prefer `uv run --with ...` for an out-of-the-box run without changing the current project env; otherwise install into your active env:

```bash
uv pip install openai
```

## Common recipes

Generate + also write a downscaled copy for fast web loading:

```text
uv run --with openai --with pillow python "$IMAGE_GEN" generate \
  --prompt "A cozy alpine cabin at dawn" \
  --size 1024x1024 \
  --downscale-max-dim 1024
```

Notes:

- Downscaling writes an extra file next to the original (default suffix `-web`, e.g. `output-web.png`).
- Downscaling requires Pillow (use `uv run --with pillow ...` or install it into your env).

Generate with augmentation fields:

```text
python "$IMAGE_GEN" generate \
  --prompt "A minimal hero image of a ceramic coffee mug" \
  --use-case "landing page hero" \
  --style "clean product photography" \
  --composition "centered product, generous negative space" \
  --constraints "no logos, no text"
```

Generate multiple prompts concurrently (async batch):

```bash
mkdir -p tmp/imagegen
cat > tmp/imagegen/prompts.jsonl << 'EOF'
{"prompt":"Cavernous hangar interior with a compact shuttle parked center-left, open bay door","use_case":"game concept art environment","composition":"wide-angle, low-angle, cinematic framing","lighting":"volumetric light rays through drifting fog","constraints":"no logos or trademarks; no watermark","size":"1536x1024"}
{"prompt":"Gray wolf in profile in a snowy forest, crisp fur texture","use_case":"wildlife photography print","composition":"100mm, eye-level, shallow depth of field","constraints":"no logos or trademarks; no watermark","size":"1024x1024"}
EOF

python "$IMAGE_GEN" generate-batch --input tmp/imagegen/prompts.jsonl --out-dir out --concurrency 5

# Cleanup (recommended)
rm -f tmp/imagegen/prompts.jsonl
```

Notes:

- Use `--concurrency` to control parallelism (default `5`). Higher concurrency can hit rate limits; the CLI retries on transient errors.
- Per-job overrides are supported in JSONL (e.g., `size`, `quality`, `background`, `output_format`, `n`, and prompt-augmentation fields).
- `--n` generates multiple variants for a single prompt; `generate-batch` is for many different prompts.
- Treat the JSONL file as temporary: write it under `tmp/` and delete it after the run (don’t commit it).

Edit:

```text
python "$IMAGE_GEN" edit --image input.png --mask mask.png --prompt "Replace the background with a warm sunset"
```

## CLI notes

- Supported sizes: `1024x1024`, `1536x1024`, `1024x1536`, or `auto`.
- Transparent backgrounds require `output_format` to be `png` or `webp`.
- Default output is `output.png`; multiple images become `output-1.png`, `output-2.png`, etc.
- Use `--no-augment` to skip prompt augmentation.

## See also

- API parameter quick reference: `references/image-api.md`
- Prompt examples: `references/sample-prompts.md`


---

<!-- merged from: cli-reference-scriptssorapy.md -->

﻿---
name: CLI reference (`scripts/sora.py`)
description: # CLI reference (`scripts/sora.py`)
 
 This file contains the command catalog for the bundled video generation CLI. Keep `SKILL.md` overview-first; put verbose CLI details here.
---

# CLI reference (`scripts/sora.py`)

This file contains the command catalog for the bundled video generation CLI. Keep `SKILL.md` overview-first; put verbose CLI details here.

## What this CLI does

- `create`: create a new video job (async)
- `create-and-poll`: create a job, poll until complete, optionally download
- `poll`: wait for an existing job to finish
- `status`: retrieve job status/details
- `download`: download video/thumbnail/spritesheet
- `list`: list recent jobs
- `delete`: delete a job
- `remix`: remix a completed video
- `create-batch`: create multiple jobs from a JSONL file

Real API calls require **network access** + `OPENAI_API_KEY`. `--dry-run` does not.

## Quick start (works from any repo)

Set a stable path to the skill CLI (default `CODEX_HOME` is `~/.codex`):

```bash
export CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
export SORA_CLI="$CODEX_HOME/skills/sora/scripts/sora.py"
```

If you're in this repo, you can set the path directly:

```javascript
# Use repo root (tests run from output/* so $PWD is not the root)
export SORA_CLI="$(git rev-parse --show-toplevel)/<path-to-skill>/scripts/sora.py"
```

If `git` isn't available, set `SORA_CLI` to the absolute path to `<path-to-skill>/scripts/sora.py`.

If uv cache fails with permission errors, set a writable cache:

```bash
export UV_CACHE_DIR="/tmp/uv-cache"
```

Dry-run (no API call; no network required; does not require the `openai` package):

```bash
python "$SORA_CLI" create --prompt "Test" --dry-run
```

Preflight a full prompt without running the API:

```text
python "$SORA_CLI" create --prompt-file prompt.txt --dry-run --json-out out/request.json
```

Create a job (requires `OPENAI_API_KEY` + network):

```text
uv run --with openai python "$SORA_CLI" create \
  --model sora-2 \
  --prompt "Wide tracking shot of a teal coupe on a desert highway" \
  --size 1280x720 \
  --seconds 8
```

Create from a prompt file (avoids shell-escaping issues for multi-line prompts):

```bash
cat > prompt.txt << 'EOF'
Use case: landing page hero
Primary request: a matte black camera on a pedestal
Action: slow 30-degree orbit over 4 seconds
Camera: 85mm, shallow depth of field
Lighting/mood: soft key light, subtle rim
Constraints: no logos, no text
EOF

uv run --with openai python "$SORA_CLI" create \
  --prompt-file prompt.txt \
  --size 1280x720 \
  --seconds 4
```

If your prompt file is already structured (Use case/Scene/Camera/etc), disable tool augmentation:

```text
uv run --with openai python "$SORA_CLI" create \
  --prompt-file prompt.txt \
  --no-augment \
  --size 1280x720 \
  --seconds 4
```

Create + poll + download:

```text
uv run --with openai python "$SORA_CLI" create-and-poll \
  --model sora-2-pro \
  --prompt "Close-up of a steaming coffee cup on a wooden table" \
  --size 1280x720 \
  --seconds 8 \
  --download \
  --variant video \
  --out coffee.mp4
```

Create + poll + write JSON bundle:

```json
uv run --with openai python "$SORA_CLI" create-and-poll \
  --prompt "Minimal product teaser of a matte black camera" \
  --json-out out/coffee-job.json
```

Remix a completed video:

```text
uv run --with openai python "$SORA_CLI" remix \
  --id video_abc123 \
  --prompt "Same shot, shift palette to teal/sand/rust with warm backlight."
```

Download a thumbnail or spritesheet:

```text
uv run --with openai python "$SORA_CLI" download --id video_abc123 --variant thumbnail --out thumb.webp
uv run --with openai python "$SORA_CLI" download --id video_abc123 --variant spritesheet --out sheet.jpg
```

## Guardrails (important)

- Use `python "$SORA_CLI" ...` (or equivalent full path) for all video work.
- For API calls, prefer `uv run --with openai ...` to avoid missing SDK errors.
- Do **not** create one-off runners unless the user explicitly asks.
- **Never modify** `scripts/sora.py` unless the user asks.

## Defaults (unless overridden by flags)

- Model: `sora-2`
- Size: `1280x720`
- Seconds: `4` (API expects a string enum: "4", "8", "12")
- Variant: `video`
- Poll interval: `10` seconds

## JSON output (`--json-out`)

- For `create`, `status`, `list`, `delete`, `poll`, and `remix`, `--json-out` writes the JSON response to a file.
- For `create-and-poll`, `--json-out` writes a bundle: `{ "create": ..., "final": ... }`.
- If the path has no extension, `.json` is added automatically.
- In `--dry-run`, `--json-out` writes the request preview instead of a response.

## Input reference images

- Must be jpg/png/webp; they should match the target size.
- Provide the path with `--input-reference`.

## Optional deps

Prefer `uv run --with ...` for an out-of-the-box run without changing the current project env; otherwise install into your active env:

```bash
uv pip install openai
```

## JSONL schema for `create-batch`

Each line is a JSON object (or a raw prompt string). Required key: `prompt`.

Top-level override keys:

- `model`, `size`, `seconds`
- `input_reference` (path)
- `out` (optional output filename for the job JSON)

Prompt augmentation keys (top-level or under `fields`):

- `use_case`, `scene`, `subject`, `action`, `camera`, `style`, `lighting`, `palette`, `audio`, `dialogue`, `text`, `timing`, `constraints`, `negative`

Notes:

- `fields` merges into the prompt augmentation inputs.
- Top-level keys override CLI defaults.
- `seconds` must be one of: "4", "8", "12".

## Common recipes

Create with prompt augmentation fields:

```text
uv run --with openai python "$SORA_CLI" create \
  --prompt "A minimal product teaser shot of a matte black camera" \
  --use-case "landing page hero" \
  --camera "85mm, slow orbit" \
  --lighting "soft key, subtle rim" \
  --constraints "no logos, no text"
```

Two-variant workflow (base + remix):

```text
# 1) Base clip
uv run --with openai python "$SORA_CLI" create-and-poll \
  --prompt "Ceramic mug on a sunlit wooden table in a cozy cafe" \
  --size 1280x720 --seconds 4 --download --out output.mp4

# 2) Remix with invariant (same shot, change only the drink)
uv run --with openai python "$SORA_CLI" remix \
  --id video_abc123 \
  --prompt "Same shot and framing; replace the mug with an iced americano in a glass, visible ice and condensation."

# 3) Poll and download the remix
uv run --with openai python "$SORA_CLI" poll \
  --id video_def456 --download --out remix.mp4
```

Poll and download after a job finishes:

```text
uv run --with openai python "$SORA_CLI" poll --id video_abc123 --download --variant video --out out.mp4
```

Write JSON response to a file:

```json
uv run --with openai python "$SORA_CLI" status --id video_abc123 --json-out out/status.json
```

Batch create (JSONL input):

```json
mkdir -p tmp/sora
cat > tmp/sora/prompts.jsonl << 'EOB'
{"prompt":"A neon-lit rainy alley, slow dolly-in","seconds":"4"}
{"prompt":"A warm sunrise over a misty lake, gentle pan","seconds":"8",
 "fields":{"camera":"35mm, slow pan","lighting":"soft dawn light"}}
EOB

uv run --with openai python "$SORA_CLI" create-batch --input tmp/sora/prompts.jsonl --out-dir out --concurrency 3

# Cleanup (recommended)
rm -f tmp/sora/prompts.jsonl
```

Notes:

- `create-batch` writes one JSON response per job under `--out-dir`.
- Output names default to `NNN-<prompt-slug>.json`.
- Use `--concurrency` to control parallelism (default `3`). Higher concurrency can hit rate limits.
- Treat the JSONL file as temporary: write it under `tmp/` and delete it after the run (do not commit it). If `rm` is blocked in your sandbox, skip cleanup or truncate the file.

## CLI notes

- Supported sizes depend on model (see `references/video-api.md`).
- Seconds are limited to 4, 8, or 12.
- Download URLs expire after about 1 hour; copy assets to your own storage.
- In CI/sandboxes where long-running commands time out, prefer `create` + `poll` (or add `--timeout`).

## See also

- API parameter quick reference: `references/video-api.md`
- Prompt structure and examples: `references/prompting.md`
- Sample prompts: `references/sample-prompts.md`
- Troubleshooting: `references/troubleshooting.md`


---

<!-- merged from: tool-use-concepts.md -->

﻿---
name: Tool Use Concepts
description: # Tool Use Concepts
 
 This file covers the conceptual foundations of tool use with the Claude API. For language-specific code examples, see the `python/`, `typescript/`, or other language folders.
---

# Tool Use Concepts

This file covers the conceptual foundations of tool use with the Claude API. For language-specific code examples, see the `python/`, `typescript/`, or other language folders.

## User-Defined Tools

### Tool Definition Structure

> **Note:** When using the Tool Runner (beta), tool schemas are generated automatically from your function signatures (Python), Zod schemas (TypeScript), annotated classes (Java), `jsonschema` struct tags (Go), or `BaseTool` subclasses (Ruby). The raw JSON schema format below is for the manual approach or SDKs without tool runner support.

Each tool requires a name, description, and JSON Schema for its inputs:

```json
{
  "name": "get_weather",
  "description": "Get current weather for a location",
  "input_schema": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "City and state, e.g., San Francisco, CA"
      },
      "unit": {
        "type": "string",
        "enum": ["celsius", "fahrenheit"],
        "description": "Temperature unit"
      }
    },
    "required": ["location"]
  }
}
```

#### Best practices for tool definitions

- Use clear, descriptive names (e.g., `get_weather`, `search_database`, `send_email`)
- Write detailed descriptions — Claude uses these to decide when to use the tool
- Include descriptions for each property
- Use `enum` for parameters with a fixed set of values
- Mark truly required parameters in `required`; make others optional with defaults

---

### Tool Choice Options

Control when Claude uses tools:

| Value | Behavior |
| --------------------------------- | --------------------------------------------- |
| `{"type": "auto"}` | Claude decides whether to use tools (default) |
| `{"type": "any"}` | Claude must use at least one tool |
| `{"type": "tool", "name": "..."}` | Claude must use the specified tool |
| `{"type": "none"}` | Claude cannot use tools |

Any `tool_choice` value can also include `"disable_parallel_tool_use": true` to force Claude to use at most one tool per response. By default, Claude may request multiple tool calls in a single response.

---

### Tool Runner vs Manual Loop

**Tool Runner (Recommended):** The SDK's tool runner handles the agentic loop automatically — it calls the API, detects tool use requests, executes your tool functions, feeds results back to Claude, and repeats until Claude stops calling tools. Available in Python, TypeScript, Java, Go, and Ruby SDKs (beta). The Python SDK also provides MCP conversion helpers (`anthropic.lib.tools.mcp`) to convert MCP tools, prompts, and resources for use with the tool runner — see `python/claude-api/tool-use.md` for details.

**Manual Agentic Loop:** Use when you need fine-grained control over the loop (e.g., custom logging, conditional tool execution, human-in-the-loop approval). Loop until `stop_reason == "end_turn"`, always append the full `response.content` to preserve tool_use blocks, and ensure each `tool_result` includes the matching `tool_use_id`.

**Stop reasons for server-side tools:** When using server-side tools (code execution, web search, etc.), the API runs a server-side sampling loop. If this loop reaches its default limit of 10 iterations, the response will have `stop_reason: "pause_turn"`. To continue, re-send the user message and assistant response and make another API request — the server will resume where it left off. Do NOT add an extra user message like "Continue." — the API detects the trailing `server_tool_use` block and knows to resume automatically.

```python
# Handle pause_turn in your agentic loop
if response.stop_reason == "pause_turn":
    messages = [
        {"role": "user", "content": user_query},
        {"role": "assistant", "content": response.content},
    ]
    # Make another API request — server resumes automatically
    response = client.messages.create(
        model="claude-opus-4-6", messages=messages, tools=tools
    )
```

Set a `max_continuations` limit (e.g., 5) to prevent infinite loops. For the full guide, see: `https://platform.claude.com/docs/en/build-with-claude/handling-stop-reasons`

> **Security:** The tool runner executes your tool functions automatically whenever Claude requests them. For tools with side effects (sending emails, modifying databases, financial transactions), validate inputs within your tool functions and consider requiring confirmation for destructive operations. Use the manual agentic loop if you need human-in-the-loop approval before each tool execution.

---

### Handling Tool Results

When Claude uses a tool, the response contains a `tool_use` block. You must:

1. Execute the tool with the provided input
2. Send the result back in a `tool_result` message
3. Continue the conversation

**Error handling in tool results:** When a tool execution fails, set `"is_error": true` and provide an informative error message. Claude will typically acknowledge the error and either try a different approach or ask for clarification.

**Multiple tool calls:** Claude can request multiple tools in a single response. Handle them all before continuing — send all results back in a single `user` message.

---
