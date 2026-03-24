---
name: figma-design-workflow
description: Complete Figma MCP workflow for design-to-code implementation. Covers MCP server setup, config, tool catalog, prompt patterns, design system rule generation, and the step-by-step workflow for translating Figma nodes into production code with 1:1 visual fidelity. Use when implementing UI from Figma files, providing Figma URLs, or establishing Figma-to-code conventions.
domain: design
triggers: implement design, Figma to code, Figma MCP, create design system rules, Figma URL, figma-implement-design, design-to-code
compatibility: Figma MCP server (figma, figma-desktop)
---

# Figma Design Workflow

Unified reference for Figma MCP integration, design implementation, and design system rule generation.

---

## 1. MCP Server Setup

### Config (codex config.toml)

```toml
[mcp_servers.figma]
url = "https://mcp.figma.com/mcp"
bearer_token_env_var = "FIGMA_OAUTH_TOKEN"
http_headers = { "X-Figma-Region" = "us-east-1" }
```

- Enable RMCP client: `[features].rmcp_client = true` (or `experimental_use_rmcp_client = true` on older builds).
- Token setup: `export FIGMA_OAUTH_TOKEN="<token>"` in shell profile. Verify with `echo $FIGMA_OAUTH_TOKEN`.
- Alternative setup: `codex mcp add figma --url https://mcp.figma.com/mcp` then `codex mcp login figma`.

### Troubleshooting

- Token not picked up: Export in same shell that launches the client, or add to profile and restart.
- OAuth errors: Verify `rmcp_client` is enabled and token is valid (no surrounding quotes).
- Network: Keep `X-Figma-Region` aligned with your org's region.

---

## 2. Tool Catalog

| Tool | Context | Purpose |
| --- | --- | --- |
| `get_design_context` | Design, Make | Primary tool. Returns structured design data + default React/Tailwind code |
| `get_metadata` | Design | Sparse XML outline of layer IDs/names/types. Use before re-calling `get_design_context` on large nodes |
| `get_screenshot` | Design, FigJam | Screenshot for visual fidelity checks |
| `get_variable_defs` | Design | Lists variables/styles (colors, spacing, typography) |
| `get_figjam` | FigJam | XML + screenshots for diagrams |
| `create_design_system_rules` | No file context | Generates rule file with design-to-code guidance for your stack |
| `get_code_connect_map` | Design | Returns mapping of Figma node IDs to code components |
| `add_code_connect_map` | Design | Adds/updates mapping between node and code component |
| `whoami` | Remote only | Returns authenticated Figma user identity |

### Prompt Patterns

- Change framework: "generate my Figma selection in Vue" / "in plain HTML + CSS" / "for iOS"
- Use existing components: "generate using components from `src/components/ui`"
- Get variables: "what color and spacing variables are used in my Figma selection?"
- Code connect: "map this node to `src/components/ui/Button.tsx` with name `Button`"

---

## 3. Implementation Workflow

### Prerequisites

- Figma MCP server connected and accessible
- Figma URL format: `https://figma.com/design/:fileKey/:fileName?node-id=1-2`
- OR when using `figma-desktop` MCP: select node directly in Figma desktop app

### Step 1: Extract Node ID

From URL: extract `:fileKey` (segment after `/design/`) and `node-id` query parameter.

Example: `https://figma.com/design/kL9xQn2VwM8pYrTb4ZcHjF/DesignSystem?node-id=42-15`
- File key: `kL9xQn2VwM8pYrTb4ZcHjF`
- Node ID: `42-15`

Note: `figma-desktop` MCP uses only `nodeId` (auto-detects open file).

### Step 2: Fetch Design Context

```
get_design_context(fileKey=":fileKey", nodeId="1-2")
```

If response is truncated: `get_metadata` first to get node map, then re-fetch specific child nodes.

### Step 3: Capture Visual Reference

```
get_screenshot(fileKey=":fileKey", nodeId="1-2")
```

### Step 4: Download Assets

- If Figma MCP returns `localhost` source for images/SVGs, use that source directly.
- DO NOT import new icon packages. All assets come from the Figma payload.
- DO NOT create placeholders if a localhost source is provided.

### Step 5: Translate to Project Conventions

- Treat Figma MCP output (React + Tailwind) as design representation, not final code style.
- Replace Tailwind utility classes with project's preferred tokens/utilities.
- Reuse existing components instead of duplicating functionality.
- Use project's color system, typography scale, and spacing tokens.
- Respect existing routing, state management, and data-fetch patterns.

### Step 6: Achieve 1:1 Visual Parity

- Avoid hardcoded values; use design tokens from Figma.
- When conflicts arise: prefer design system tokens but adjust spacing minimally.
- Follow WCAG requirements for accessibility.

### Step 7: Validate

- [ ] Layout matches (spacing, alignment, sizing)
- [ ] Typography matches (font, size, weight, line height)
- [ ] Colors match exactly
- [ ] Interactive states work (hover, active, disabled)
- [ ] Responsive behavior follows Figma constraints
- [ ] Assets render correctly
- [ ] Accessibility standards met

---

## 4. Design System Rule Generation

Use `create_design_system_rules` when establishing project-specific Figma-to-code conventions.

### Parameters

- `clientLanguages`: e.g., "typescript,javascript"
- `clientFrameworks`: e.g., "react", "vue", "svelte", "angular"

### Rule File Targets

| Agent | Rule File |
| --- | --- |
| Claude Code | `CLAUDE.md` |
| Codex CLI | `AGENTS.md` |
| Cursor | `.cursor/rules/figma-design-system.mdc` |

### Workflow

1. Run `create_design_system_rules` tool
2. Analyze codebase: component locations, styling approach, naming conventions, architecture
3. Generate rules covering: component organization, styling, Figma MCP integration flow, asset handling, project-specific conventions
4. Save to appropriate rule file
5. Test with a simple component implementation

### Rule Writing Best Practices

- Be specific: "Use Button from `src/components/ui/Button.tsx`" not "Use the design system"
- Make rules actionable: tell what to do, not just what to avoid
- Prefix critical rules with "IMPORTANT:"
- Document reasoning for non-obvious rules
- Start simple, iterate as inconsistencies emerge

---

## 5. Common Issues

| Issue | Cause | Solution |
| --- | --- | --- |
| Truncated output | Too many nested layers | Use `get_metadata` then fetch specific nodes |
| Design mismatch | Visual discrepancies | Compare side-by-side with screenshot; check spacing/colors/typography |
| Assets not loading | MCP assets endpoint inaccessible | Verify endpoint; use localhost URLs directly |
| Token mismatch | Project tokens differ from Figma | Prefer project tokens; adjust spacing for visual fidelity |
| Agent ignores rules | Rules too vague or not loaded | Make specific, add IMPORTANT prefix, restart agent |

## Resources

- [Figma MCP Server Documentation](https://developers.figma.com/docs/figma-mcp-server/)
- [Figma MCP Tools and Prompts](https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/)
- [Figma Variables and Design Tokens](https://help.figma.com/hc/en-us/articles/15339657135383-Guide-to-variables-in-Figma)
