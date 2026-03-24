---
name: "doc (Part 3)"
description: "Use when the task involves reading, creating, or editing `.docx` documents, especially when formatting or layout fidelity matters; prefer `python-docx` plus the bundled `scripts/render_docx.py` for visual checks. - Part 3"
---


## Data Citations

When presenting data, cite the source:

```markdown
| Metric | Q3 | Q4 | Change |
| -------- | ---- | ---- | -------- |
| Revenue | $2.3M | $2.8M | +21.7% |
| Users | 12.4K | 15.1K | +21.8% |

Source: <mention-page url="...">Financial Dashboard</mention-page>
```

## Database Citations

When referencing database content:

```markdown
Based on analysis of the <mention-database url="...">Projects Database</mention-database>, 67% of projects are on track.
```

## User Citations

When attributing information to specific people:

```markdown
<mention-user url="...">Sarah Chen</mention-user> noted in <mention-page url="...">Architecture Review</mention-page> that the microservices migration is ahead of schedule.
```

## Citation Frequency

**Over-citing** (every sentence):

```markdown
The revenue increased (<mention-page url="...">Report</mention-page>). 
Costs decreased (<mention-page url="...">Report</mention-page>). 
Margin improved (<mention-page url="...">Report</mention-page>).
```

**Under-citing** (no attribution):

```markdown
The revenue increased, costs decreased, and margin improved.
```

**Right balance** (grouped citation):

```markdown
The revenue increased, costs decreased, and margin improved (<mention-page url="...">Q4 Financial Report</mention-page>).
```

## Outdated Information

Note when source information might be outdated:

```markdown
The original API design (<mention-page url="...">API Spec v1</mention-page>, last updated January 2024) has been superseded by the new architecture in <mention-page url="...">API Spec v2</mention-page>.
```

## Cross-References

Link to related research documents:

```markdown
## Related Research

This research builds on previous findings:
- <mention-page url="...">Market Analysis - Q2 2025</mention-page>
- <mention-page url="...">Competitor Landscape Review</mention-page>

For implementation details, see:
- <mention-page url="...">Technical Implementation Guide</mention-page>
```

## Citation Validation

Before finalizing research:

✓ Every key claim has a source citation
✓ All page mentions have valid URLs
✓ Sources section includes all cited pages
✓ Outdated sources are noted as such
✓ Direct quotes are clearly marked
✓ Data sources are attributed

## Citation Style Consistency

Choose one citation style and use throughout:

**Inline style** (lightweight):

```markdown
Revenue grew 23% (Financial Report). Customer count increased 18% (Metrics Dashboard).
```

**Formal style** (full mentions):

```markdown
Revenue grew 23% (<mention-page url="...">Q4 Financial Report</mention-page>). Customer count increased 18% (<mention-page url="...">Metrics Dashboard</mention-page>).
```

**Recommend formal style** for most research documentation as it provides clickable navigation.


---

<!-- merged from: tutorial-patterns.md -->

﻿---
name: Tutorial Patterns
description: # Tutorial Patterns
 
 Use this structure for teaching and walkthroughs:
---

# Tutorial Patterns

Use this structure for teaching and walkthroughs:

- Audience, prerequisites, and learning goals: say who it is for, list what they should already know, and state what they will be able to do by the end.
- Outline: provide a short numbered outline so readers can skim.
- Step-by-step flow: pair a short markdown explanation with a small code cell that runs on its own and a brief interpretation of the result.
- Exercises: include at least one exercise that reinforces the key concept and provide an answer scaffold in the next cell.
- Pitfalls and extensions: call out one common mistake and how to fix it, and suggest one optional extension for curious readers.


---

<!-- merged from: experiment-patterns.md -->

﻿---
name: Experiment Patterns
description: # Experiment Patterns
 
 Use this structure for exploratory and experimental work:
---

# Experiment Patterns

Use this structure for exploratory and experimental work:

- Title and objective: state the question and the success criteria.
- Setup and reproducibility: import only what you need, set a seed early, and keep configuration in one short cell.
- Plan: list hypotheses, sweeps, and metrics before running code.
- Minimal baseline: start with the smallest runnable example and confirm it runs end-to-end before adding complexity.
- Results and notes: summarize findings in markdown near the relevant code and record key metrics in a small dictionary or table-like structure.
- Next steps: decide whether to continue, pivot, or stop, and capture follow-up ideas as short bullets.


---

<!-- merged from: live-documentation-sources.md -->

﻿---
name: Live Documentation Sources
description: # Live Documentation Sources
 
 This file contains WebFetch URLs for fetching current information from platform.claude.com and Agent SDK repositories. Use these when users need the latest data that may have changed since the cached content was last updated.
---

# Live Documentation Sources

This file contains WebFetch URLs for fetching current information from platform.claude.com and Agent SDK repositories. Use these when users need the latest data that may have changed since the cached content was last updated.

## When to Use WebFetch

- User explicitly asks for "latest" or "current" information
- Cached data seems incorrect
- User asks about features not covered in cached content
- User needs specific API details or examples

## Claude API Documentation URLs

### Models & Pricing

| Topic | URL | Extraction Prompt |
| --------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Models Overview | `https://platform.claude.com/docs/en/about-claude/models/overview.md` | "Extract current model IDs, context windows, and pricing for all Claude models" |
| Pricing | `https://platform.claude.com/docs/en/pricing.md` | "Extract current pricing per million tokens for input and output" |

### Core Features

| Topic | URL | Extraction Prompt |
| ----------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Extended Thinking | `https://platform.claude.com/docs/en/build-with-claude/extended-thinking.md` | "Extract extended thinking parameters, budget_tokens requirements, and usage examples" |
| Adaptive Thinking | `https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking.md` | "Extract adaptive thinking setup, effort levels, and Claude Opus 4.6 usage examples" |
| Effort Parameter | `https://platform.claude.com/docs/en/build-with-claude/effort.md` | "Extract effort levels, cost-quality tradeoffs, and interaction with thinking" |
| Tool Use | `https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview.md` | "Extract tool definition schema, tool_choice options, and handling tool results" |
| Streaming | `https://platform.claude.com/docs/en/build-with-claude/streaming.md` | "Extract streaming event types, SDK examples, and best practices" |
| Prompt Caching | `https://platform.claude.com/docs/en/build-with-claude/prompt-caching.md` | "Extract cache_control usage, pricing benefits, and implementation examples" |

### Media & Files

| Topic | URL | Extraction Prompt |
| ----------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Vision | `https://platform.claude.com/docs/en/build-with-claude/vision.md` | "Extract supported image formats, size limits, and code examples" |
| PDF Support | `https://platform.claude.com/docs/en/build-with-claude/pdf-support.md` | "Extract PDF handling capabilities, limits, and examples" |

### API Operations

| Topic | URL | Extraction Prompt |
| ---------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Batch Processing | `https://platform.claude.com/docs/en/build-with-claude/batch-processing.md` | "Extract batch API endpoints, request format, and polling for results" |
| Files API | `https://platform.claude.com/docs/en/build-with-claude/files.md` | "Extract file upload, download, and referencing in messages, including supported types and beta header" |
| Token Counting | `https://platform.claude.com/docs/en/build-with-claude/token-counting.md` | "Extract token counting API usage and examples" |
| Rate Limits | `https://platform.claude.com/docs/en/api/rate-limits.md` | "Extract current rate limits by tier and model" |
| Errors | `https://platform.claude.com/docs/en/api/errors.md` | "Extract HTTP error codes, meanings, and retry guidance" |

### Tools

| Topic | URL | Extraction Prompt |
| -------------- | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Code Execution | `https://platform.claude.com/docs/en/agents-and-tools/tool-use/code-execution-tool.md` | "Extract code execution tool setup, file upload, container reuse, and response handling" |
| Computer Use | `https://platform.claude.com/docs/en/agents-and-tools/tool-use/computer-use.md` | "Extract computer use tool setup, capabilities, and implementation examples" |

### Advanced Features

| Topic | URL | Extraction Prompt |
| ------------------ | ----------------------------------------------------------------------------- | --------------------------------------------------- |
| Structured Outputs | `https://platform.claude.com/docs/en/build-with-claude/structured-outputs.md` | "Extract output_config.format usage and schema enforcement" |
| Compaction | `https://platform.claude.com/docs/en/build-with-claude/compaction.md` | "Extract compaction setup, trigger config, and streaming with compaction" |
| Citations | `https://platform.claude.com/docs/en/build-with-claude/citations.md` | "Extract citation format and implementation" |
| Context Windows | `https://platform.claude.com/docs/en/build-with-claude/context-windows.md` | "Extract context window sizes and token management" |

---

## Claude API SDK Repositories

| SDK | URL | Description |
| ---------- | --------------------------------------------------------- | ------------------------------ |
| Python | `https://github.com/anthropics/anthropic-sdk-python` | `anthropic` pip package source |
| TypeScript | `https://github.com/anthropics/anthropic-sdk-typescript` | `@anthropic-ai/sdk` npm source |
| Java | `https://github.com/anthropics/anthropic-sdk-java` | `anthropic-java` Maven source |
| Go | `https://github.com/anthropics/anthropic-sdk-go` | Go module source |
| Ruby | `https://github.com/anthropics/anthropic-sdk-ruby` | `anthropic` gem source |
| C# | `https://github.com/anthropics/anthropic-sdk-csharp` | NuGet package source |
| PHP | `https://github.com/anthropics/anthropic-sdk-php` | Composer package source |

---

## Agent SDK Documentation URLs

### Core Documentation

| Topic | URL | Extraction Prompt |
| -------------------- | ----------------------------------------------------------- | --------------------------------------------------------------- |
| Agent SDK Overview | `https://platform.claude.com/docs/en/agent-sdk.md` | "Extract the Agent SDK overview, key features, and use cases" |
| Agent SDK Python | `https://github.com/anthropics/claude-agent-sdk-python` | "Extract Python SDK installation, imports, and basic usage" |
| Agent SDK TypeScript | `https://github.com/anthropics/claude-agent-sdk-typescript` | "Extract TypeScript SDK installation, imports, and basic usage" |

### SDK Reference (GitHub READMEs)

| Topic | URL | Extraction Prompt |
| -------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| Python SDK | `https://raw.githubusercontent.com/anthropics/claude-agent-sdk-python/main/README.md` | "Extract Python SDK API reference, classes, and methods" |
| TypeScript SDK | `https://raw.githubusercontent.com/anthropics/claude-agent-sdk-typescript/main/README.md` | "Extract TypeScript SDK API reference, types, and functions" |

### npm/PyPI Packages

| Package | URL | Description |
| ----------------------------------- | -------------------------------------------------------------- | ------------------------- |
| claude-agent-sdk (Python) | `https://pypi.org/project/claude-agent-sdk/` | Python package on PyPI |
| @anthropic-ai/claude-agent-sdk (TS) | `https://www.npmjs.com/package/@anthropic-ai/claude-agent-sdk` | TypeScript package on npm |

### GitHub Repositories

| Resource | URL | Description |
| -------------- | ----------------------------------------------------------- | ----------------------------------- |
| Python SDK | `https://github.com/anthropics/claude-agent-sdk-python` | Python package source |
| TypeScript SDK | `https://github.com/anthropics/claude-agent-sdk-typescript` | TypeScript/Node.js package source |
| MCP Servers | `https://github.com/modelcontextprotocol` | Official MCP server implementations |

---

## Fallback Strategy

If WebFetch fails (network issues, URL changed):

1. Use cached content from the language-specific files (note the cache date)
2. Inform user the data may be outdated
3. Suggest they check platform.claude.com or the GitHub repos directly


---

<!-- merged from: example-conversation-to-faq.md -->

﻿---
name: Example: Conversation to FAQ
description: # Example: Conversation to FAQ
 
 ## User Request
---

# Example: Conversation to FAQ

## User Request (Example: Conversation to FAQ)

> "Save this conversation about deployment troubleshooting to the FAQ"

**Context**: User just had a conversation explaining how to troubleshoot common deployment errors.

## Conversation Summary

The conversation covered:

1. **Question**: "Why does deployment fail with 'port already in use' error?"
2. **Answer**: Process from previous deployment still running, need to kill it
3. **Question**: "How do I find and kill the process?"
4. **Commands shared**: `lsof -ti:3000 | xargs kill -9` or use `pm2 restart app`
5. **Question**: "What about 'cannot connect to database' errors?"
6. **Answer**: Check database credentials, verify database is running, check network connectivity
7. **Best practice**: Always check logs first (`pm2 logs` or `docker logs`)

## Workflow Execution

### Step 1: Identify Content Type

This is Q&A content → **FAQ Entry** format is appropriate

### Step 2: Structure FAQ Entries

Extract distinct Q&A pairs from conversation:

**FAQ 1**: Port already in use error
**FAQ 2**: Cannot connect to database error
**FAQ 3**: General troubleshooting approach

### Step 3: Determine Destination

```yaml
Notion:notion-search
query: "FAQ deployment"
query_type: "internal"
```

Found: "Deployment FAQ" database in Engineering Docs

```yaml
Notion:notion-fetch
id: "deployment-faq-database-id"
```

#### Schema

- Question (title)
- Category (select): Deployment, Configuration, Troubleshooting, etc.
- Tags (multi_select)
- Last Reviewed (date)

### Step 4: Create FAQ Entries

#### FAQ Entry 1: Port Already in Use

```text
Notion:notion-create-pages

parent: { data_source_id: "collection://faq-db-uuid" }

pages: [{
  properties: {
    "Question": "Why does deployment fail with 'port already in use' error?",
    "Category": "Troubleshooting",
    "Tags": "deployment, errors, ports",
    "date:Last Reviewed:start": "2025-10-14",
    "date:Last Reviewed:is_datetime": 0
  },
  content: "## Short Answer\n\nThe deployment port (usually 3000) is still occupied by a process from a previous deployment. You need to kill the existing process before deploying again.\n\n## Detailed Explanation\n\nWhen you deploy the application, it tries to bind to a specific port (e.g., port 3000). If a previous deployment didn't shut down cleanly, that process may still be running and holding the port. This prevents the new deployment from starting.\n\n**Common causes**:\n- Previous deployment crashed without cleanup\n- Manual `node` process started and forgotten\n- PM2 or other process manager didn't restart properly\n- Multiple deployments attempted simultaneously\n\n## Solution\n\n### Option 1: Kill the process using the port\n\n```bash\n# Find and kill process on port 3000\nlsof -ti:3000 | xargs kill -9\n```\n\nReplace `3000` with your application's port number.\n\n### Option 2: If using PM2\n\n```bash\n# Restart the application\npm2 restart app\n\n# Or delete and restart\npm2 delete app\npm2 start app.js --name app\n```\n\n### Option 3: Check all node processes\n\n```bash\n# List all node processes\nps aux | grep node\n\n# Kill specific process by PID\nkill -9 <PID>\n```\n\n## Prevention\n\n1. **Use process managers**: PM2, systemd, or Docker handle cleanup automatically\n2. **Graceful shutdown**: Implement proper shutdown handlers in your app\n3. **Health checks**: Monitor if previous deployment shut down before starting new one\n\n```javascript\n// Graceful shutdown example\nprocess.on('SIGTERM', () => {\n  server.close(() => {\n    console.log('Server closed');\n    process.exit(0);\n  });\n});\n```\n\n## Verification\n\nAfter killing the process, verify the port is free:\n\n```bash\nlsof -ti:3000\n# Should return nothing if port is free\n```\n\n## Related Questions\n- <mention-page url=\"...\">How do I check what's using a port?</mention-page>\n- <mention-page url=\"...\">How do I configure the application port?</mention-page>\n- <mention-page url=\"...\">PM2 deployment best practices</mention-page>\n\n## Last Updated\nOctober 14, 2025"
}]
```

#### FAQ Entry 2: Database Connection Errors

```text
Notion:notion-create-pages

parent: { data_source_id: "collection://faq-db-uuid" }

pages: [{
  properties: {
    "Question": "Why do I get 'cannot connect to database' errors during deployment?",
    "Category": "Troubleshooting",
    "Tags": "deployment, database, errors",
    "date:Last Reviewed:start": "2025-10-14",
    "date:Last Reviewed:is_datetime": 0
  },
  content: "## Short Answer\n\nDatabase connection errors usually mean either the database isn't running, credentials are incorrect, or there's a network connectivity issue. Check database status, verify credentials, and test connectivity.\n\n## Detailed Explanation\n\nThe application can't establish a connection to the database during startup. This prevents the application from initializing properly.\n\n**Common causes**:\n- Database service isn't running\n- Incorrect connection credentials\n- Network connectivity issues (firewall, security groups)\n- Database host/port misconfigured\n- Database is at connection limit\n- SSL/TLS configuration mismatch\n\n## Troubleshooting Steps\n\n### Step 1: Check database status\n\n```bash\n# For local PostgreSQL\npg_isready -h localhost -p 5432\n\n# For Docker\ndocker ps | grep postgres\n\n# For MongoDB\nmongosh --eval \"db.adminCommand('ping')\"\n```\n\n### Step 2: Verify credentials\n\nCheck your `.env` or configuration file:\n\n```bash\n# Common environment variables\nDB_HOST=localhost\nDB_PORT=5432\nDB_NAME=myapp_production\nDB_USER=myapp_user\nDB_PASSWORD=***********\n```\n\nTest connection manually:\n\n```bash\n# PostgreSQL\npsql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME\n\n# MongoDB\nmongosh \"mongodb://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME\"\n```\n\n### Step 3: Check network connectivity\n\n```bash\n# Test if port is reachable\ntelnet $DB_HOST $DB_PORT\n\n# Or using nc\nnc -zv $DB_HOST $DB_PORT\n\n# Check firewall rules (if applicable)\nsudo iptables -L\n```\n\n### Step 4: Check application logs\n\n```bash\n# PM2 logs\npm2 logs app\n\n# Docker logs\ndocker logs container-name\n\n# Application logs\ntail -f /var/log/app/error.log\n```\n\nLook for specific error messages:\n- `ECONNREFUSED`: Database not running or wrong host/port\n- `Authentication failed`: Wrong credentials\n- `Timeout`: Network/firewall issue\n- `Too many connections`: Database connection limit reached\n\n## Solutions by Error Type\n\n### Database Not Running\n\n```bash\n# Start PostgreSQL\nsudo systemctl start postgresql\n\n# Start via Docker\ndocker start postgres-container\n```\n\n### Wrong Credentials\n\n1. Reset database password\n2. Update `.env` file\n3. Restart application\n\n### Connection Limit Reached\n\n```sql\n-- Check current connections (PostgreSQL)\nSELECT count(*) FROM pg_stat_activity;\n\n-- Increase max connections\nALTER SYSTEM SET max_connections = 200;\n```\n\n### SSL/TLS Issues\n\nAdd to connection string:\n```\nssl=true&sslmode=require\n```\n\nOr disable SSL for dev:\n```\nsslmode=disable\n```\n\n## Prevention\n\n1. **Connection pooling**: Limits concurrent connections\n```javascript\npool: {\n  min: 2,\n  max: 10\n}\n```\n\n2. **Health checks**: Verify database before starting app\n```javascript\nconst healthCheck = async () => {\n  try {\n    await db.raw('SELECT 1');\n    console.log('Database connected');\n  } catch (err) {\n    console.error('Database connection failed:', err);\n    process.exit(1);\n  }\n};\n```\n\n3. **Retry logic**: Auto-reconnect on temporary failures\n```javascript\nretry: {\n  max: 3,\n  backoff: 1000\n}\n```\n\n4. **Environment validation**: Check all required env vars exist\n```javascript\nconst required = ['DB_HOST', 'DB_PORT', 'DB_NAME', 'DB_USER', 'DB_PASSWORD'];\nfor (const key of required) {\n  if (!process.env[key]) {\n    throw new Error(`Missing required env var: ${key}`);\n  }\n}\n```\n\n## Related Questions\n- <mention-page url=\"...\">How do I configure database connection pooling?</mention-page>\n- <mention-page url=\"...\">Database backup and restore procedures</mention-page>\n- <mention-page url=\"...\">Database performance optimization</mention-page>\n\n## Last Updated\nOctober 14, 2025"
}]
```

#### FAQ Entry 3: General Troubleshooting

```text
Notion:notion-create-pages

parent: { data_source_id: "collection://faq-db-uuid" }

pages: [{
  properties: {
    "Question": "What's the first thing I should check when deployment fails?",
    "Category": "Troubleshooting",
    "Tags": "deployment, debugging, best-practices",
    "date:Last Reviewed:start": "2025-10-14",
    "date:Last Reviewed:is_datetime": 0
  },
  content: "## Short Answer\n\n**Always check the logs first.** Logs contain error messages that point you directly to the problem. Use `pm2 logs`, `docker logs`, or check your application's log files.\n\n## Detailed Explanation\n\nLogs are your first and most important debugging tool. They show:\n- Exact error messages\n- Stack traces\n- Timing information\n- Configuration issues\n- Dependency problems\n\nMost deployment issues can be diagnosed and fixed by reading the logs carefully.\n\n## How to Check Logs\n\n### PM2\n\n```bash\n# View all logs\npm2 logs\n\n# View logs for specific app\npm2 logs app-name\n\n# View only errors\npm2 logs --err\n\n# Follow logs in real-time\npm2 logs --lines 100\n```\n\n### Docker\n\n```bash\n# View logs\ndocker logs container-name\n\n# Follow logs\ndocker logs -f container-name\n\n# Last 100 lines\ndocker logs --tail 100 container-name\n\n# With timestamps\ndocker logs -t container-name\n```\n\n### Application Logs\n\n```bash\n# Tail application logs\ntail -f /var/log/app/app.log\ntail -f /var/log/app/error.log\n\n# Search logs for errors\ngrep -i error /var/log/app/*.log\n\n# View logs with context\ngrep -B 5 -A 5 \"ERROR\" app.log\n```\n\n## Systematic Troubleshooting Approach\n\n### 1. Check the logs\n- Read error messages carefully\n- Note the exact error type and message\n- Check timestamps to find when error occurred\n\n### 2. Verify configuration\n- Environment variables set correctly?\n- Configuration files present and valid?\n- Paths and file permissions correct?\n\n### 3. Check dependencies\n- All packages installed? (`node_modules` present?)\n- Correct versions installed?\n- Any native module compilation errors?\n\n### 4. Verify environment\n- Required services running (database, Redis, etc.)?\n- Ports available?\n- Network connectivity working?\n\n### 5. Test components individually\n- Can you connect to database manually?\n- Can you run application locally?\n- Do health check endpoints work?\n\n### 6. Check recent changes\n- What changed since last successful deployment?\n- New dependencies added?\n- Configuration modified?\n- Environment differences?\n\n## Common Error Patterns\n\n### \"Module not found\"\n```bash\n# Solution: Install dependencies\nnpm install\n# or\nnpm ci\n```\n\n### \"Permission denied\"\n```bash\n# Solution: Fix file permissions\nchmod +x start.sh\nsudo chown -R appuser:appuser /app\n```\n\n### \"Address already in use\"\n```bash\n# Solution: Kill process on port\nlsof -ti:3000 | xargs kill -9\n```\n\n### \"Cannot connect to...\"\n```bash\n# Solution: Verify service is running and reachable\ntelnet service-host port\n```\n\n## Debugging Tools\n\n### Log Aggregation\n- **PM2**: Built-in log management\n- **Docker**: Centralized logging with log drivers\n- **ELK Stack**: Elasticsearch, Logstash, Kibana for large scale\n- **CloudWatch**: For AWS deployments\n\n### Monitoring\n- **PM2 Monit**: `pm2 monit` for real-time metrics\n- **Docker Stats**: `docker stats` for resource usage\n- **System metrics**: `top`, `htop` for CPU/memory\n\n### Network Debugging\n```bash\n# Check open connections\nnetstat -tulpn\n\n# Check specific port\nlsof -i :3000\n\n# Test connectivity\ncurl http://localhost:3000/health\n```\n\n## When to Escalate\n\nEscalate to senior team member if:\n- Error message is cryptic and not Google-able\n- Problem persists after trying obvious solutions\n- Production is affected\n- Data integrity may be compromised\n- Security-related errors\n\n## Prevention\n\n1. **Automated health checks** before deployment\n2. **Staged deployments** (dev → staging → production)\n3. **Rollback plan** ready for quick recovery\n4. **Monitoring and alerts** for early problem detection\n5. **Deployment checklist** to ensure all steps followed\n\n## Related Questions\n- <mention-page url=\"...\">How do I set up log aggregation?</mention-page>\n- <mention-page url=\"...\">Deployment checklist and best practices</mention-page>\n- <mention-page url=\"...\">How to roll back a failed deployment?</mention-page>\n\n## Last Updated\nOctober 14, 2025"
}]
```

### Step 5: Update FAQ Index

```yaml
Notion:notion-fetch
id: "faq-index-page-id"
```

```yaml
Notion:notion-update-page
page_id: "faq-index-page-id"
command: "insert_content_after"
selection_with_ellipsis: "## Deployment & Troubleshooting..."
new_str: "
- <mention-page url=\"...\">Why does deployment fail with 'port already in use' error?</mention-page>
- <mention-page url=\"...\">Why do I get 'cannot connect to database' errors?</mention-page>
- <mention-page url=\"...\">What's the first thing I should check when deployment fails?</mention-page>
"
```

## Summary Provided to User

```markdown
## FAQ Entries Created ✅

### Conversation Captured
Transformed deployment troubleshooting conversation into 3 comprehensive FAQ entries.

### FAQ Entries Created

1. **Port Already in Use Error**
   - Problem: Deployment fails due to occupied port
   - Solutions: Kill process, use PM2, check node processes
   - Prevention: Use process managers, graceful shutdown
   - Link: <mention-page url="...">Port already in use FAQ</mention-page>

2. **Database Connection Errors**
   - Problem: Cannot connect to database during deployment
   - Troubleshooting: 4-step systematic approach
   - Solutions: By error type (credentials, network, limits)
   - Prevention: Connection pooling, health checks, retry logic
   - Link: <mention-page url="...">Database connection FAQ</mention-page>

3. **General Troubleshooting Approach**
   - Best practice: Always check logs first
   - Tools: PM2 logs, Docker logs, application logs
   - Systematic approach: 6-step debugging process
   - Common errors: Module not found, permissions, ports
   - Link: <mention-page url="...">Troubleshooting first steps FAQ</mention-page>

### Content Structure

Each FAQ includes:
✅ Short answer (quick reference)
✅ Detailed explanation (full context)
✅ Multiple solution options
✅ Code examples and commands
✅ Prevention strategies
✅ Links to related FAQs
✅ Last updated date

### Database Integration

- ✅ Added to FAQ database (not standalone pages)
- ✅ Properties set: Category (Troubleshooting), Tags, Date
- ✅ FAQ index page updated with links
- ✅ Cross-references between related questions

### Value Added

**From conversation** → **To structured knowledge**:
- Casual Q&A → Professional FAQ entries
- Implicit knowledge → Explicit documentation
- One-time help → Reusable resource
- 5-minute conversation → Permanent team knowledge

**Future benefit**:
- Team can self-serve on common issues
- New team members can onboard faster
- Reduces repeated questions
- Improves troubleshooting speed

**Knowledge captured and preserved! 📚**
```

## Key Features Demonstrated

### Content Extraction

- Identified distinct Q&A pairs from conversation
- Extracted commands, solutions, and best practices
- Preserved troubleshooting workflows

### FAQ Structure

- Short answer for quick reference
- Detailed explanation for understanding
- Multiple solutions (Options 1, 2, 3)
- Code examples with comments
- Prevention strategies
- Related questions linking

### Database Integration

- Created in FAQ database (not standalone)
- Set appropriate properties (category, tags, date)
- Updated index page with links
- Cross-referenced related questions

### Value Transformation

- Converted informal conversation to professional docs
- Added structure, examples, and prevention tips
- Made knowledge searchable and discoverable
- Created permanent team resource

Perfect for: FAQ creation, knowledge preservation, team documentation


---

<!-- merged from: example-technical-investigation.md -->

﻿---
name: Example: Technical Investigation
description: # Example: Technical Investigation
 
 ## User Request
---

# Example: Technical Investigation

## User Request (Example: Technical Investigation)

> "Research our current caching strategy and create a technical summary"

## Workflow Execution

### Step 1: Search for Caching Information

```yaml
Notion:notion-search
query: "caching strategy architecture"
query_type: "internal"
teamspace_id: "engineering-teamspace-id"
```

#### Results Found

- "System Architecture Overview" (Engineering)
- "Redis Implementation Guide" (Backend Docs)
- "Performance Optimization - Q3 2024" (Engineering)
- "API Caching Decision Record" (Architecture)

### Step 2: Fetch Relevant Pages

```yaml
Notion:notion-fetch
id: "system-architecture-page-url"
```

Extracted: Current caching architecture uses Redis for API responses, Memcached for session storage.

```yaml
Notion:notion-fetch
id: "redis-implementation-guide-url"
```

Extracted: Implementation details, TTL settings, invalidation strategies.

```yaml
Notion:notion-fetch
id: "api-caching-decision-record-url"
```

Extracted: Why Redis was chosen over alternatives, trade-offs considered.

### Step 3: Synthesize Findings

#### Key findings identified

- Two-tier caching: Redis (API responses) + Memcached (sessions)
- TTL strategy: 5 minutes for dynamic data, 1 hour for static
- Invalidation: Event-driven for critical updates
- Performance impact: 75% reduction in database load
- Known issues: Cache stampede on popular endpoints

### Step 4: Create Technical Summary

```text
Notion:notion-create-pages

parent: { page_id: "engineering-docs-parent-id" }

pages: [{
  properties: {
    "title": "Technical Summary: Caching Strategy - Oct 2025"
  },
  content: "[Structured technical summary using template]"
}]
```

## Output Document

```markdown
# Technical Summary: Caching Strategy - Oct 2025

## Executive Summary

Our caching infrastructure uses a two-tier approach with Redis for API response caching and Memcached for session management. This strategy has reduced database load by 75% and improved API response times from 200ms to 50ms average.

## Architecture Overview

### Layer 1: API Response Caching (Redis)
**Technology**: Redis 7.0 cluster (3 nodes)
**Purpose**: Cache GET endpoint responses
**TTL Strategy**:
- Dynamic content: 5 minutes
- Static content: 1 hour
- User-specific: 15 minutes

**Source**: <mention-page url="...">System Architecture Overview</mention-page>

### Layer 2: Session Storage (Memcached)
**Technology**: Memcached 1.6
**Purpose**: User session data, temporary state
**TTL**: 24 hours (session lifetime)

## Implementation Details

### Cache Key Format
```

api:v1:{endpoint}:{params_hash}
session:{user_id}:{session_id}

```text

### Invalidation Strategy
- **Event-driven**: Critical data changes trigger immediate invalidation
- **Time-based**: TTL expiration for non-critical data
- **Manual**: Admin tools for emergency cache clear

**Source**: <mention-page url="...">Redis Implementation Guide</mention-page>

## Decision Rationale

### Why Redis for API Caching?

**Pros**:
- Advanced data structures (sorted sets, hashes)
- Built-in TTL with automatic eviction
- Pub/sub for cache invalidation events
- Persistence options for durability

**Cons**:
- Higher memory usage than Memcached
- More complex cluster management

**Decision**: Chosen for flexibility and rich feature set needed for API caching.

**Source**: <mention-page url="...">API Caching Decision Record</mention-page>

### Why Memcached for Sessions?

**Pros**:
- Simpler, lighter weight
- Excellent for key-value storage
- Lower memory footprint

**Cons**:
- No persistence
- Limited data structures

**Decision**: Perfect fit for ephemeral session data where simplicity is valued.

## Performance Impact

| Metric | Before Caching | After Caching | Improvement |
| -------- | ---------------- | --------------- | ------------- |
| Avg Response Time | 200ms | 50ms | 75% faster |
| Database Load | 100% | 25% | 75% reduction |
| Cache Hit Rate | - | 85% | - |
| Peak RPS Handled | 1,000 | 4,000 | 4x increase |

**Source**: <mention-page url="...">Performance Optimization - Q3 2024</mention-page>

## Known Issues & Limitations

### Cache Stampede
**Issue**: When popular cache entries expire, multiple requests hit database simultaneously

**Mitigation**: Implemented probabilistic early expiration and request coalescing

**Status**: Reduced by 90% but not eliminated

### Stale Data Risk
**Issue**: Cached data can be up to TTL duration stale

**Mitigation**: Event-driven invalidation for critical data paths

**Status**: Acceptable trade-off for performance gains

## Monitoring & Observability

**Metrics tracked**:
- Cache hit/miss rates per endpoint
- Memory usage and eviction rates
- Response time distributions
- Invalidation event frequency

**Tools**: DataDog dashboards, CloudWatch alarms

## Future Considerations

1. **Edge Caching**: Evaluate CDN integration for static assets
2. **Cache Warming**: Pre-populate cache for predictable traffic spikes
3. **Adaptive TTLs**: Adjust TTLs based on data change frequency
4. **Regional Caching**: Multi-region cache replication for global performance

## Related Documentation

- <mention-page url="...">System Architecture Overview</mention-page>
- <mention-page url="...">Redis Implementation Guide</mention-page>
- <mention-page url="...">API Caching Decision Record</mention-page>
- <mention-page url="...">Performance Optimization - Q3 2024</mention-page>

## Appendix: Configuration Examples

### Redis Configuration
```yaml
maxmemory: 8gb
maxmemory-policy: allkeys-lru
tcp-keepalive: 60
```text

### Common Cache Operations

```python
# Set with TTL
cache.set(key, value, ttl=300)

# Get with fallback
value = cache.get(key) or fetch_from_db(key)

# Invalidate pattern
cache.delete_pattern("api:v1:users:*")
```text

```

## Key Success Factors

1. **Multiple source integration**: Combined architecture docs, implementation guides, and decision records
2. **Technical depth**: Included configuration, code examples, metrics
3. **Decision context**: Explained why choices were made, not just what
4. **Practical focus**: Real performance numbers and known issues
5. **Future-looking**: Noted areas for improvement
6. **Well-cited**: Every major point links back to source material

## Workflow Pattern Demonstrated

This example shows the complete research workflow:

- **Scoped search** (teamspace filter for engineering)
- **Multi-page synthesis** (4 different sources)
- **Technical template** (architecture-focused format)
- **Proper placement** (under engineering docs)
- **Comprehensive citations** (links to all sources)

---

<!-- merged from: editing-presentations.md -->

﻿---
name: Editing Presentations
description: # Editing Presentations
 
 ## Template-Based Workflow
---

# Editing Presentations

## Template-Based Workflow (Editing Presentations)

When using an existing presentation as a template:

1. **Analyze existing slides**:

   ```bash
   python scripts/thumbnail.py template.pptx
   python -m markitdown template.pptx
   ```

   Review `thumbnails.jpg` to see layouts, and markitdown output to see placeholder text.

2. **Plan slide mapping**: For each content section, choose a template slide.

   ⚠️ **USE VARIED LAYOUTS** — monotonous presentations are a common failure mode. Don't default to basic title + bullet slides. Actively seek out:
   - Multi-column layouts (2-column, 3-column)
   - Image + text combinations
   - Full-bleed images with text overlay
   - Quote or callout slides
   - Section dividers
   - Stat/number callouts
   - Icon grids or icon + text rows

   **Avoid:** Repeating the same text-heavy layout for every slide.

   Match content type to layout style (e.g., key points → bullet slide, team info → multi-column, testimonials → quote slide).

3. **Unpack**: `python scripts/office/unpack.py template.pptx unpacked/`

4. **Build presentation** (do this yourself, not with subagents):
   - Delete unwanted slides (remove from `<p:sldIdLst>`)
   - Duplicate slides you want to reuse (`add_slide.py`)
   - Reorder slides in `<p:sldIdLst>`
   - **Complete all structural changes before step 5**

5. **Edit content**: Update text in each `slide{N}.xml`.
   **Use subagents here if available** — slides are separate XML files, so subagents can edit in parallel.

6. **Clean**: `python scripts/clean.py unpacked/`

7. **Pack**: `python scripts/office/pack.py unpacked/ output.pptx --original template.pptx`

---

## Scripts

| Script | Purpose |
| -------- | --------- |
| `unpack.py` | Extract and pretty-print PPTX |
| `add_slide.py` | Duplicate slide or create from layout |
| `clean.py` | Remove orphaned files |
| `pack.py` | Repack with validation |
| `thumbnail.py` | Create visual grid of slides |

### unpack.py

```bash
python scripts/office/unpack.py input.pptx unpacked/
```

Extracts PPTX, pretty-prints XML, escapes smart quotes.

### add_slide.py

```bash
python scripts/add_slide.py unpacked/ slide2.xml      # Duplicate slide
python scripts/add_slide.py unpacked/ slideLayout2.xml # From layout
```

Prints `<p:sldId>` to add to `<p:sldIdLst>` at desired position.

### clean.py

```bash
python scripts/clean.py unpacked/
```

Removes slides not in `<p:sldIdLst>`, unreferenced media, orphaned rels.

### pack.py

```bash
python scripts/office/pack.py unpacked/ output.pptx --original input.pptx
```

Validates, repairs, condenses XML, re-encodes smart quotes.

### thumbnail.py

```bash
python scripts/thumbnail.py input.pptx [output_prefix] [--cols N]
```

Creates `thumbnails.jpg` with slide filenames as labels. Default 3 columns, max 12 per grid.

**Use for template analysis only** (choosing layouts). For visual QA, use `soffice` + `pdftoppm` to create full-resolution individual slide images—see SKILL.md.

---

## Slide Operations

Slide order is in `ppt/presentation.xml` → `<p:sldIdLst>`.

**Reorder**: Rearrange `<p:sldId>` elements.

**Delete**: Remove `<p:sldId>`, then run `clean.py`.

**Add**: Use `add_slide.py`. Never manually copy slide files—the script handles notes references, Content_Types.xml, and relationship IDs that manual copying misses.

---

## Editing Content

**Subagents:** If available, use them here (after completing step 4). Each slide is a separate XML file, so subagents can edit in parallel. In your prompt to subagents, include:

- The slide file path(s) to edit
- **"Use the Edit tool for all changes"**
- The formatting rules and common pitfalls below

For each slide:

1. Read the slide's XML
2. Identify ALL placeholder content—text, images, charts, icons, captions
3. Replace each placeholder with final content

**Use the Edit tool, not sed or Python scripts.** The Edit tool forces specificity about what to replace and where, yielding better reliability.

### Formatting Rules

- **Bold all headers, subheadings, and inline labels**: Use `b="1"` on `<a:rPr>`. This includes:
  - Slide titles
  - Section headers within a slide
  - Inline labels like (e.g.: "Status:", "Description:") at the start of a line
- **Never use unicode bullets (•)**: Use proper list formatting with `<a:buChar>` or `<a:buAutoNum>`
- **Bullet consistency**: Let bullets inherit from the layout. Only specify `<a:buChar>` or `<a:buNone>`.

---

## Common Pitfalls

### Template Adaptation

When source content has fewer items than the template:

- **Remove excess elements entirely** (images, shapes, text boxes), don't just clear text
- Check for orphaned visuals after clearing text content
- Run visual QA to catch mismatched counts

When replacing text with different length content:

- **Shorter replacements**: Usually safe
- **Longer replacements**: May overflow or wrap unexpectedly
- Test with visual QA after text changes
- Consider truncating or splitting content to fit the template's design constraints

**Template slots ≠ Source items**: If template has 4 team members but source has 3 users, delete the 4th member's entire group (image + text boxes), not just the text.

### Multi-Item Content

If source has multiple items (numbered lists, multiple sections), create separate `<a:p>` elements for each — **never concatenate into one string**.

**❌ WRONG** — all items in one paragraph:

```xml
<a:p>
  <a:r><a:rPr .../><a:t>Step 1: Do the first thing. Step 2: Do the second thing.</a:t></a:r>
</a:p>
```

**✅ CORRECT** — separate paragraphs with bold headers:

```xml
<a:p>
  <a:pPr algn="l"><a:lnSpc><a:spcPts val="3919"/></a:lnSpc></a:pPr>
  <a:r><a:rPr lang="en-US" sz="2799" b="1" .../><a:t>Step 1</a:t></a:r>
</a:p>
<a:p>
  <a:pPr algn="l"><a:lnSpc><a:spcPts val="3919"/></a:lnSpc></a:pPr>
  <a:r><a:rPr lang="en-US" sz="2799" .../><a:t>Do the first thing.</a:t></a:r>
</a:p>
<a:p>
  <a:pPr algn="l"><a:lnSpc><a:spcPts val="3919"/></a:lnSpc></a:pPr>
  <a:r><a:rPr lang="en-US" sz="2799" b="1" .../><a:t>Step 2</a:t></a:r>
</a:p>
<!-- continue pattern -->
```

Copy `<a:pPr>` from the original paragraph to preserve line spacing. Use `b="1"` on headers.

### Smart Quotes

Handled automatically by unpack/pack. But the Edit tool converts smart quotes to ASCII.

#### When adding new text with quotes, use XML entities

```xml
<a:t>the &#x201C;Agreement&#x201D;</a:t>
```

| Character | Name | Unicode | XML Entity |
| ----------- | ------ | --------- | ------------ |
| `“` | Left double quote | U+201C | `&#x201C;` |
| `”` | Right double quote | U+201D | `&#x201D;` |
| `‘` | Left single quote | U+2018 | `&#x2018;` |
| `’` | Right single quote | U+2019 | `&#x2019;` |

### Other

- **Whitespace**: Use `xml:space="preserve"` on `<a:t>` with leading/trailing spaces
- **XML parsing**: Use `defusedxml.minidom`, not `xml.etree.ElementTree` (corrupts namespaces)


---

<!-- merged from: specification-parsing.md -->

﻿---
name: Specification Parsing
description: # Specification Parsing
 
 ## Finding the Specification
---

# Specification Parsing

## Finding the Specification (Specification Parsing)

Before parsing, locate the spec page:

```yaml
1. Search for spec:
   Notion:notion-search
   query: "[Feature Name] spec" or "[Feature Name] specification"
   
2. Handle results:
   - If found → use page URL/ID
   - If multiple → ask user which one
   - If not found → ask user for URL/ID

Example:
Notion:notion-search
query: "User Profile API spec"
query_type: "internal"
```

## Reading Specifications

After finding the spec, fetch it with `Notion:notion-fetch`:

1. Read the full content
2. Identify key sections
3. Extract structured information
4. Note ambiguities or gaps

```yaml
Notion:notion-fetch
id: "spec-page-id-from-search"
```

## Common Spec Structures

### Requirements-Based Spec

```text
# Feature Spec
## Overview
[Feature description]

## Requirements
### Functional
- REQ-1: [Requirement]
- REQ-2: [Requirement]

### Non-Functional
- PERF-1: [Performance requirement]
- SEC-1: [Security requirement]

## Acceptance Criteria
- AC-1: [Criterion]
- AC-2: [Criterion]
```

Extract:

- List of functional requirements
- List of non-functional requirements
- List of acceptance criteria

### User Story Based Spec

```text
# Feature Spec
## User Stories
### As a [user type]
I want [goal]
So that [benefit]

**Acceptance Criteria**:
- [Criterion]
- [Criterion]
```

Extract:

- User personas
- Goals/capabilities needed
- Acceptance criteria per story

### Technical Design Doc

```text
# Technical Design
## Problem Statement
[Problem description]

## Proposed Solution
[Solution approach]

## Architecture
[Architecture details]

## Implementation Plan
[Implementation approach]
```

Extract:

- Problem being solved
- Proposed solution approach
- Architectural decisions
- Implementation guidance

### Product Requirements Document (PRD)

```text
# PRD: [Feature]
## Goals
[Business goals]

## User Needs
[User problems being solved]

## Features
[Feature list]

## Success Metrics
[How to measure success]
```

Extract:

- Business goals
- User needs
- Feature list
- Success metrics

## Extraction Strategies

### Requirement Identification

Look for:

- "Must", "Should", "Will" statements
- Numbered requirements (REQ-1, etc.)
- User stories (As a... I want...)
- Acceptance criteria sections
- Feature lists

### Categorization

Group requirements by:

**Functional**: What the system does

- User capabilities
- System behaviors
- Data operations

**Non-Functional**: How the system performs

- Performance targets
- Security requirements
- Scalability needs
- Availability requirements
- Compliance requirements

**Constraints**: Limitations

- Technical constraints
- Business constraints
- Timeline constraints

### Priority Extraction

Identify priority indicators:

- "Critical", "Must have", "P0"
- "Important", "Should have", "P1"
- "Nice to have", "Could have", "P2"
- "Future", "Won't have", "P3"

Map to implementation phases based on priority.

## Handling Ambiguity

### Unclear Requirements

When requirement is ambiguous:

```markdown
## Clarifications Needed

### [Requirement ID/Description]
**Current text**: "[Ambiguous requirement]"
**Question**: [What needs clarification]
**Impact**: [Why this matters for implementation]
**Assumed for now**: [Working assumption if any]
```

Create clarification task or add comment to spec.

### Missing Information

When critical info is missing:

```markdown
## Missing Information

- **[Topic]**: Spec doesn't specify [what's missing]
- **Impact**: Blocks [affected tasks]
- **Action**: Need to [how to resolve]
```

### Conflicting Requirements

When requirements conflict:

```markdown
## Conflicting Requirements

**Conflict**: REQ-1 says [X] but REQ-5 says [Y]
**Impact**: [Implementation impact]
**Resolution needed**: [Decision needed]
```

## Acceptance Criteria Parsing

### Explicit Criteria

Direct acceptance criteria:

```text
## Acceptance Criteria
- User can log in with email and password
- System sends confirmation email
- Session expires after 24 hours
```

Convert to checklist:

- [ ] User can log in with email and password
- [ ] System sends confirmation email
- [ ] Session expires after 24 hours

### Implicit Criteria

Derive from requirements:

```text
Requirement: "Users can upload files up to 100MB"

Implied acceptance criteria:
- [ ] Files up to 100MB upload successfully
- [ ] Files over 100MB are rejected with error message
- [ ] Progress indicator shows during upload
- [ ] Upload can be cancelled
```

### Testable Criteria

Ensure criteria are testable:

❌ **Not testable**: "System is fast"
✓ **Testable**: "Page loads in < 2 seconds"

❌ **Not testable**: "Users like the interface"
✓ **Testable**: "90% of test users complete task successfully"

## Technical Detail Extraction

### Architecture Information

Extract:

- System components
- Data models
- APIs/interfaces
- Integration points
- Technology choices

### Design Decisions

Note:

- Technology selections
- Architecture patterns
- Trade-offs made
- Rationale provided

### Implementation Guidance

Look for:

- Suggested approach
- Code examples
- Library recommendations
- Best practices mentioned

## Dependency Identification

### External Dependencies

From spec, identify:

- Third-party services required
- External APIs needed
- Infrastructure requirements
- Tool/library dependencies

### Internal Dependencies

Identify:

- Other features needed first
- Shared components required
- Team dependencies
- Data dependencies

### Timeline Dependencies

Note:

- Hard deadlines
- Milestone dependencies
- Sequencing requirements

## Scope Extraction

### In Scope

What's explicitly included:

- Features to build
- Use cases to support
- Users/personas to serve

### Out of Scope

What's explicitly excluded:

- Features deferred
- Use cases not supported
- Edge cases not handled

### Assumptions

What's assumed:

- Environment assumptions
- User assumptions
- System state assumptions

## Risk Identification

Extract risk information:

### Technical Risks

- Unproven technology
- Complex integration
- Performance concerns
- Scalability unknowns

### Business Risks

- Market timing
- Resource availability
- Dependency on others

### Mitigation Strategies

Note any mitigation approaches mentioned in spec.

## Spec Quality Assessment

Evaluate spec completeness:

✓ **Good spec**:

- Clear requirements
- Explicit acceptance criteria
- Priorities defined
- Risks identified
- Technical approach outlined

⚠️ **Incomplete spec**:

- Vague requirements
- Missing acceptance criteria
- Unclear priorities
- No risk analysis
- Technical details absent

Document gaps and create clarification tasks.

## Parsing Checklist

Before creating implementation plan:

☐ All functional requirements identified
☐ Non-functional requirements noted
☐ Acceptance criteria extracted
☐ Dependencies identified
☐ Risks noted
☐ Ambiguities documented
☐ Technical approach understood
☐ Scope is clear
☐ Priorities are defined