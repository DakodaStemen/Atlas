---
name: claude-code-plugin-development
description: Complete guide to Claude Code plugin development covering plugin structure (manifest, auto-discovery, directory layout), agent development (subagent structure, system prompts, triggering), hook development (PreToolUse, PostToolUse, Stop, SessionStart event handlers), and plugin settings (YAML frontmatter .local.md configuration pattern). Use when creating, scaffolding, or configuring Claude Code plugins.
domain: ai-engineering
tags: [claude-code, plugin, agent, hook, PreToolUse, PostToolUse, settings, manifest, auto-discovery]
triggers: claude code plugin, scaffold plugin, plugin.json, CLAUDE_PLUGIN_ROOT, create agent, subagent, hook development, PreToolUse, PostToolUse, plugin settings, .local.md
---

# Claude Code Plugin Development

## 1. Plugin Structure

### Directory Layout

```text
plugin-name/
├── .claude-plugin/
│   └── plugin.json          # Required: Plugin manifest
├── commands/                 # Slash commands (.md files)
├── agents/                   # Subagent definitions (.md files)
├── skills/                   # Agent skills (subdirectories)
│   └── skill-name/
│       └── SKILL.md         # Required for each skill
├── hooks/
│   ├── hooks.json           # Event handler configuration
│   └── scripts/             # Hook scripts
├── .mcp.json                # MCP server definitions
└── scripts/                 # Helper scripts
```

### Critical Rules

1. Manifest MUST be in `.claude-plugin/` directory.
2. Component directories (commands, agents, skills, hooks) MUST be at plugin root, NOT inside `.claude-plugin/`.
3. Only create directories for components the plugin actually uses.
4. Use kebab-case for all directory and file names.

### Plugin Manifest (plugin.json)

```json
{
  "name": "plugin-name",
  "version": "1.0.0",
  "description": "Brief explanation of plugin purpose",
  "commands": "./custom-commands",
  "agents": ["./agents", "./specialized-agents"],
  "hooks": "./config/hooks.json",
  "mcpServers": "./.mcp.json"
}
```

Custom paths supplement defaults -- they do not replace them. Paths must be relative, starting with `./`.

### Auto-Discovery

1. Plugin manifest read on enable.
2. Commands: All `.md` in `commands/`.
3. Agents: All `.md` in `agents/`.
4. Skills: All `SKILL.md` in `skills/*/`.
5. Hooks: Load from `hooks/hooks.json` or manifest.
6. MCP servers: Load from `.mcp.json` or manifest.

### Portable Paths

Always use `${CLAUDE_PLUGIN_ROOT}` for intra-plugin paths. Never hardcode absolute paths, home directory shortcuts, or relative paths from working directory.

```json
{ "command": "bash ${CLAUDE_PLUGIN_ROOT}/scripts/run.sh" }
```

## 2. Agent Development

### Agent File Format

Location: `agents/` directory. Format: `.md` with YAML frontmatter.

```markdown
---
description: Agent role and expertise
capabilities:
  - Specific capability 1
  - Specific capability 2
---

Detailed agent instructions...
```

### System Prompt Design

- Define the agent's role, expertise boundaries, and output format.
- Include explicit constraints (what NOT to do).
- Specify tools the agent can use and when.
- Include few-shot examples for complex behaviors.

### Triggering

- Users invoke manually via slash commands or agent names.
- Claude Code auto-selects based on task context matching the description.
- Write descriptions that match how users phrase their needs.

### Agent Patterns

- **Specialist**: Deep expertise in one domain (code review, test generation, security audit).
- **Coordinator**: Manages workflow across multiple specialists.
- **Validator**: Checks outputs against criteria (grading, compliance).

### Agent Communication

- Agents receive structured inputs via prompt parameters.
- Return structured outputs (JSON preferred for machine consumption).
- Pass context between agents via working files or state objects.

## 3. Hook Development

### Available Events

| Event | When It Fires | Common Use |
|-------|---------------|------------|
| `PreToolUse` | Before any tool call | Validate, block dangerous ops |
| `PostToolUse` | After tool call completes | Transform output, log results |
| `Stop` | Before final response to user | Validate completeness |
| `SubagentStop` | Before subagent returns | Check subagent output |
| `SessionStart` | New session begins | Load state, initialize |
| `SessionEnd` | Session ends | Save state, cleanup |
| `UserPromptSubmit` | User sends a message | Classify, route |
| `PreCompact` | Before context compaction | Preserve critical context |
| `Notification` | System notification | Alert, log |

### Hook Configuration

```json
{
  "PreToolUse": [{
    "matcher": "Write|Edit",
    "hooks": [{
      "type": "command",
      "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/scripts/validate.sh",
      "timeout": 30
    }]
  }],
  "PostToolUse": [{
    "matcher": ".*",
    "hooks": [{
      "type": "command",
      "command": "node ${CLAUDE_PLUGIN_ROOT}/hooks/scripts/log-tool.js",
      "timeout": 10
    }]
  }]
}
```

### Matcher Patterns

- `"Write|Edit"` — matches Write or Edit tool calls.
- `".*"` — matches all tool calls.
- `"Bash"` — matches Bash tool calls only.
- Matchers use regex against tool names.

### Hook Script Interface

Hooks receive JSON on stdin with context (tool name, arguments, result for PostToolUse). Return JSON on stdout with optional modifications.

**PreToolUse response options**:
- `{ "decision": "allow" }` — proceed normally.
- `{ "decision": "block", "reason": "Explanation" }` — prevent tool call.
- `{ "decision": "modify", "args": { ... } }` — modify tool arguments.

### Hook Best Practices

- Set appropriate timeouts (hooks block execution). Default: 30s for validation, 10s for logging.
- Handle errors gracefully (hook failure should not crash the session).
- Log hook decisions for debugging.
- Test hooks independently before integrating.

## 4. Plugin Settings

### .local.md Pattern

Store per-project configuration in `.claude/plugin-name.local.md`:

```markdown
---
debug: false
max_retries: 3
style: conventional
ignore_patterns:
  - "*.test.ts"
  - "*.spec.ts"
---

Additional context or custom prompt content here.
```

### Reading Settings

Parse YAML frontmatter from the .local.md file. Use defaults for missing values. Validate types and ranges.

### Settings Patterns

- **Boolean flags**: Feature toggles (`debug: true`).
- **String enums**: Mode selection (`style: conventional | angular | custom`).
- **Lists**: Inclusion/exclusion patterns.
- **Nested objects**: Complex configuration.

### Best Practices

- Document all settings in the plugin README.
- Provide sensible defaults for all settings.
- Validate settings at session start, warn on invalid values.
- Use `.local.md` (gitignored) for user-specific settings.
- Use non-local files for shared team settings.

## 5. Skill Development

### SKILL.md Format

```markdown
---
name: Skill Name
description: When to use this skill (primary activation trigger)
version: 1.0.0
domain: optional_domain
triggers: optional keywords
---

## Critical Rules
- Do's and don'ts (bullet list)

## Workflow Process
Step-by-step instructions

## Technical Deliverables
Concrete outputs (checklist, diff, snippet)

## Success Metrics
How to know the task is done
```

### Progressive Disclosure

- Description is the primary activation trigger (1-2 sentences matching task phrasing).
- Critical rules are the minimum for correct execution.
- Workflow process provides step-by-step guidance.
- References provide deep-dive details.

### Supporting Files

Skills can include scripts, references, and examples in subdirectories:

```text
skills/api-testing/
├── SKILL.md
├── scripts/test-runner.py
├── references/api-spec.md
└── examples/sample-test.ts
```

## Troubleshooting

### Component Not Loading

- Verify file is in correct directory with correct extension.
- Check YAML frontmatter syntax.
- Ensure skill has `SKILL.md` (not README.md).
- Confirm plugin is enabled.

### Path Resolution Errors

- Replace all hardcoded paths with `${CLAUDE_PLUGIN_ROOT}`.
- Verify paths are relative starting with `./`.
- Test with `echo $CLAUDE_PLUGIN_ROOT` in scripts.

### Auto-Discovery Issues

- Confirm directories are at plugin root (not in `.claude-plugin/`).
- Check kebab-case naming and correct extensions.
- Restart Claude Code to reload plugin configuration.
