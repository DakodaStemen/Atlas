---
name: "MEGA-AI-TOOLS (Part 2)"
description: "Consolidated AI tool patterns - ChatGPT Apps SDK, image generation (imagegen), video generation (Sora), screenshot capture, tool use concepts/TypeScript, CLI references, web game development, search-and-fetch standard. - Part 2"
---


## Server-Side Tools: Code Execution

The code execution tool lets Claude run code in a secure, sandboxed container. Unlike user-defined tools, server-side tools run on Anthropic's infrastructure — you don't execute anything client-side. Just include the tool definition and Claude handles the rest.

### Key Facts

- Runs in an isolated container (1 CPU, 5 GiB RAM, 5 GiB disk)
- No internet access (fully sandboxed)
- Python 3.11 with data science libraries pre-installed
- Containers persist for 30 days and can be reused across requests
- Free when used with web search/web fetch tools; otherwise $0.05/hour after 1,550 free hours/month per organization

### Tool Definition

The tool requires no schema — just declare it in the `tools` array:

```json
{
  "type": "code_execution_20260120",
  "name": "code_execution"
}
```

Claude automatically gains access to `bash_code_execution` (run shell commands) and `text_editor_code_execution` (create/view/edit files).

### Pre-installed Python Libraries

- **Data science**: pandas, numpy, scipy, scikit-learn, statsmodels
- **Visualization**: matplotlib, seaborn
- **File processing**: openpyxl, xlsxwriter, pillow, pypdf, pdfplumber, python-docx, python-pptx
- **Math**: sympy, mpmath
- **Utilities**: tqdm, python-dateutil, pytz, sqlite3

Additional packages can be installed at runtime via `pip install`.

### Supported File Types for Upload

| Type | Extensions |
| ------ | ---------------------------------- |
| Data | CSV, Excel (.xlsx/.xls), JSON, XML |
| Images | JPEG, PNG, GIF, WebP |
| Text | .txt, .md, .py, .js, etc. |

### Container Reuse

Reuse containers across requests to maintain state (files, installed packages, variables). Extract the `container_id` from the first response and pass it to subsequent requests.

### Response Structure

The response contains interleaved text and tool result blocks:

- `text` — Claude's explanation
- `server_tool_use` — What Claude is doing
- `bash_code_execution_tool_result` — Code execution output (check `return_code` for success/failure)
- `text_editor_code_execution_tool_result` — File operation results

> **Security:** Always sanitize filenames with `os.path.basename()` / `path.basename()` before writing downloaded files to disk to prevent path traversal attacks. Write files to a dedicated output directory.

---

## Server-Side Tools: Web Search and Web Fetch

Web search and web fetch let Claude search the web and retrieve page content. They run server-side — just include the tool definitions and Claude handles queries, fetching, and result processing automatically.

### Tool Definitions

```json
[
  { "type": "web_search_20260209", "name": "web_search" },
  { "type": "web_fetch_20260209", "name": "web_fetch" }
]
```

### Dynamic Filtering (Opus 4.6 / Sonnet 4.6)

The `web_search_20260209` and `web_fetch_20260209` versions support **dynamic filtering** — Claude writes and executes code to filter search results before they reach the context window, improving accuracy and token efficiency. Dynamic filtering is built into these tool versions and activates automatically; you do not need to separately declare the `code_execution` tool or pass any beta header.

```json
{
  "tools": [
    { "type": "web_search_20260209", "name": "web_search" },
    { "type": "web_fetch_20260209", "name": "web_fetch" }
  ]
}
```

Without dynamic filtering, the previous `web_search_20250305` version is also available.

> **Note:** Only include the standalone `code_execution` tool when your application needs code execution for its own purposes (data analysis, file processing, visualization) independent of web search. Including it alongside `_20260209` web tools creates a second execution environment that can confuse the model.

---

## Server-Side Tools: Programmatic Tool Calling

Programmatic tool calling lets Claude execute complex multi-tool workflows in code, keeping intermediate results out of the context window. Claude writes code that calls your tools directly, reducing token usage for multi-step operations.

For full documentation, use WebFetch:

- URL: `https://platform.claude.com/docs/en/agents-and-tools/tool-use/programmatic-tool-calling`

---

## Server-Side Tools: Tool Search

The tool search tool lets Claude dynamically discover tools from large libraries without loading all definitions into the context window. Useful when you have many tools but only a few are relevant to any given query.

For full documentation, use WebFetch:

- URL: `https://platform.claude.com/docs/en/agents-and-tools/tool-use/tool-search-tool`

---

## Tool Use Examples

You can provide sample tool calls directly in your tool definitions to demonstrate usage patterns and reduce parameter errors. This helps Claude understand how to correctly format tool inputs, especially for tools with complex schemas.

For full documentation, use WebFetch:

- URL: `https://platform.claude.com/docs/en/agents-and-tools/tool-use/implement-tool-use`

---

## Server-Side Tools: Computer Use

Computer use lets Claude interact with a desktop environment (screenshots, mouse, keyboard). It can be Anthropic-hosted (server-side, like code execution) or self-hosted (you provide the environment and execute actions client-side).

For full documentation, use WebFetch:

- URL: `https://platform.claude.com/docs/en/agents-and-tools/computer-use/overview`

---

## Client-Side Tools: Memory

The memory tool enables Claude to store and retrieve information across conversations through a memory file directory. Claude can create, read, update, and delete files that persist between sessions.

### Key Facts (Client-Side Tools: Memory)

- Client-side tool — you control storage via your implementation
- Supports commands: `view`, `create`, `str_replace`, `insert`, `delete`, `rename`
- Operates on files in a `/memories` directory
- The SDKs provide helper classes/functions for implementing the memory backend

> **Security:** Never store API keys, passwords, tokens, or other secrets in memory files. Be cautious with personally identifiable information (PII) — check data privacy regulations (GDPR, CCPA) before persisting user data. The reference implementations have no built-in access control; in multi-user systems, implement per-user memory directories and authentication in your tool handlers.

For full implementation examples, use WebFetch:

- Docs: `https://platform.claude.com/docs/en/agents-and-tools/tool-use/memory-tool.md`

---

## Structured Outputs

Structured outputs constrain Claude's responses to follow a specific JSON schema, guaranteeing valid, parseable output. This is not a separate tool — it enhances the Messages API response format and/or tool parameter validation.

Two features are available:

- **JSON outputs** (`output_config.format`): Control Claude's response format
- **Strict tool use** (`strict: true`): Guarantee valid tool parameter schemas

**Supported models:** Claude Opus 4.6, Claude Sonnet 4.6, and Claude Haiku 4.5. Legacy models (Claude Opus 4.5, Claude Opus 4.1) also support structured outputs.

> **Recommended:** Use `client.messages.parse()` which automatically validates responses against your schema. When using `messages.create()` directly, use `output_config: {format: {...}}`. The `output_format` convenience parameter is also accepted by some SDK methods (e.g., `.parse()`), but `output_config.format` is the canonical API-level parameter.

### JSON Schema Limitations

#### Supported

- Basic types: object, array, string, integer, number, boolean, null
- `enum`, `const`, `anyOf`, `allOf`, `$ref`/`$def`
- String formats: `date-time`, `time`, `date`, `duration`, `email`, `hostname`, `uri`, `ipv4`, `ipv6`, `uuid`
- `additionalProperties: false` (required for all objects)

#### Not supported

- Recursive schemas
- Numerical constraints (`minimum`, `maximum`, `multipleOf`)
- String constraints (`minLength`, `maxLength`)
- Complex array constraints
- `additionalProperties` set to anything other than `false`

The Python and TypeScript SDKs automatically handle unsupported constraints by removing them from the schema sent to the API and validating them client-side.

### Important Notes

- **First request latency**: New schemas incur a one-time compilation cost. Subsequent requests with the same schema use a 24-hour cache.
- **Refusals**: If Claude refuses for safety reasons (`stop_reason: "refusal"`), the output may not match your schema.
- **Token limits**: If `stop_reason: "max_tokens"`, output may be incomplete. Increase `max_tokens`.
- **Incompatible with**: Citations (returns 400 error), message prefilling.
- **Works with**: Batches API, streaming, token counting, extended thinking.

---

## Tips for Effective Tool Use

1. **Provide detailed descriptions**: Claude relies heavily on descriptions to understand when and how to use tools
2. **Use specific tool names**: `get_current_weather` is better than `weather`
3. **Validate inputs**: Always validate tool inputs before execution
4. **Handle errors gracefully**: Return informative error messages so Claude can adapt
5. **Limit tool count**: Too many tools can confuse the model — keep the set focused
6. **Test tool interactions**: Verify Claude uses tools correctly in various scenarios

For detailed tool use documentation, use WebFetch:

- URL: `https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview`


---

<!-- merged from: tool-use-typescript.md -->

﻿---
name: Tool Use — TypeScript
description: # Tool Use — TypeScript
 
 For conceptual overview (tool definitions, tool choice, tips), see [shared/tool-use-concepts.md](../../shared/tool-use-concepts.md).
---

# Tool Use — TypeScript

For conceptual overview (tool definitions, tool choice, tips), see [shared/tool-use-concepts.md](../../shared/tool-use-concepts.md).

## Tool Runner (Recommended)

**Beta:** The tool runner is in beta in the TypeScript SDK.

Use `betaZodTool` with Zod schemas to define tools with a `run` function, then pass them to `client.beta.messages.toolRunner()`:

```typescript
import Anthropic from "@anthropic-ai/sdk";
import { betaZodTool } from "@anthropic-ai/sdk/helpers/beta/zod";
import { z } from "zod";

const client = new Anthropic();

const getWeather = betaZodTool({
  name: "get_weather",
  description: "Get current weather for a location",
  inputSchema: z.object({
    location: z.string().describe("City and state, e.g., San Francisco, CA"),
    unit: z.enum(["celsius", "fahrenheit"]).optional(),
  }),
  run: async (input) => {
    // Your implementation here
    return `72°F and sunny in ${input.location}`;
  },
});

// The tool runner handles the agentic loop and returns the final message
const finalMessage = await client.beta.messages.toolRunner({
  model: "claude-opus-4-6",
  max_tokens: 4096,
  tools: [getWeather],
  messages: [{ role: "user", content: "What's the weather in Paris?" }],
});

console.log(finalMessage.content);
```

### Key benefits of the tool runner

- No manual loop — the SDK handles calling tools and feeding results back
- Type-safe tool inputs via Zod schemas
- Tool schemas are generated automatically from Zod definitions
- Iteration stops automatically when Claude has no more tool calls

---

## Manual Agentic Loop

Use this when you need fine-grained control (custom logging, conditional tool execution, streaming individual iterations, human-in-the-loop approval):

```typescript
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic();
const tools: Anthropic.Tool[] = [...]; // Your tool definitions
let messages: Anthropic.MessageParam[] = [{ role: "user", content: userInput }];

while (true) {
  const response = await client.messages.create({
    model: "claude-opus-4-6",
    max_tokens: 4096,
    tools: tools,
    messages: messages,
  });

  if (response.stop_reason === "end_turn") break;

  // Server-side tool hit iteration limit; re-send to continue
  if (response.stop_reason === "pause_turn") {
    messages = [
      { role: "user", content: userInput },
      { role: "assistant", content: response.content },
    ];
    continue;
  }

  const toolUseBlocks = response.content.filter(
    (b): b is Anthropic.ToolUseBlock => b.type === "tool_use",
  );

  messages.push({ role: "assistant", content: response.content });

  const toolResults: Anthropic.ToolResultBlockParam[] = [];
  for (const tool of toolUseBlocks) {
    const result = await executeTool(tool.name, tool.input);
    toolResults.push({
      type: "tool_result",
      tool_use_id: tool.id,
      content: result,
    });
  }

  messages.push({ role: "user", content: toolResults });
}
```

### Streaming Manual Loop

Use `client.messages.stream()` + `finalMessage()` instead of `.create()` when you need streaming within a manual loop. Text deltas are streamed on each iteration; `finalMessage()` collects the complete `Message` so you can inspect `stop_reason` and extract tool-use blocks:

```typescript
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic();
const tools: Anthropic.Tool[] = [...];
let messages: Anthropic.MessageParam[] = [{ role: "user", content: userInput }];

while (true) {
  const stream = client.messages.stream({
    model: "claude-opus-4-6",
    max_tokens: 4096,
    tools,
    messages,
  });

  // Stream text deltas on each iteration
  stream.on("text", (delta) => {
    process.stdout.write(delta);
  });

  // finalMessage() resolves with the complete Message — no need to
  // manually wire up .on("message") / .on("error") / .on("abort")
  const message = await stream.finalMessage();

  if (message.stop_reason === "end_turn") break;

  // Server-side tool hit iteration limit; re-send to continue
  if (message.stop_reason === "pause_turn") {
    messages = [
      { role: "user", content: userInput },
      { role: "assistant", content: message.content },
    ];
    continue;
  }

  const toolUseBlocks = message.content.filter(
    (b): b is Anthropic.ToolUseBlock => b.type === "tool_use",
  );

  messages.push({ role: "assistant", content: message.content });

  const toolResults: Anthropic.ToolResultBlockParam[] = [];
  for (const tool of toolUseBlocks) {
    const result = await executeTool(tool.name, tool.input);
    toolResults.push({
      type: "tool_result",
      tool_use_id: tool.id,
      content: result,
    });
  }

  messages.push({ role: "user", content: toolResults });
}
```

> **Important:** Don't wrap `.on()` events in `new Promise()` to collect the final message — use `stream.finalMessage()` instead. The SDK handles all error/abort/completion states internally.
>
> **Error handling in the loop:** Use the SDK's typed exceptions (e.g., `Anthropic.RateLimitError`, `Anthropic.APIError`) — see [Error Handling](./README.md#error-handling) for examples. Don't check error messages with string matching.
>
> **SDK types:** Use `Anthropic.MessageParam`, `Anthropic.Tool`, `Anthropic.ToolUseBlock`, `Anthropic.ToolResultBlockParam`, `Anthropic.Message`, etc. for all API-related data structures. Don't redefine equivalent interfaces.

---

## Handling Tool Results

```typescript
const response = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  tools: tools,
  messages: [{ role: "user", content: "What's the weather in Paris?" }],
});

for (const block of response.content) {
  if (block.type === "tool_use") {
    const result = await executeTool(block.name, block.input);

    const followup = await client.messages.create({
      model: "claude-opus-4-6",
      max_tokens: 1024,
      tools: tools,
      messages: [
        { role: "user", content: "What's the weather in Paris?" },
        { role: "assistant", content: response.content },
        {
          role: "user",
          content: [
            { type: "tool_result", tool_use_id: block.id, content: result },
          ],
        },
      ],
    });
  }
}
```

---

## Tool Choice

```typescript
const response = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  tools: tools,
  tool_choice: { type: "tool", name: "get_weather" },
  messages: [{ role: "user", content: "What's the weather in Paris?" }],
});
```

---

## Code Execution

### Basic Usage

```typescript
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic();

const response = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 4096,
  messages: [
    {
      role: "user",
      content:
        "Calculate the mean and standard deviation of [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]",
    },
  ],
  tools: [{ type: "code_execution_20260120", name: "code_execution" }],
});
```

### Upload Files for Analysis

```typescript
import Anthropic, { toFile } from "@anthropic-ai/sdk";
import { createReadStream } from "fs";

const client = new Anthropic();

// 1. Upload a file
const uploaded = await client.beta.files.upload({
  file: await toFile(createReadStream("sales_data.csv"), undefined, {
    type: "text/csv",
  }),
  betas: ["files-api-2025-04-14"],
});

// 2. Pass to code execution
// Code execution is GA; Files API is still beta (pass via RequestOptions)
const response = await client.messages.create(
  {
    model: "claude-opus-4-6",
    max_tokens: 4096,
    messages: [
      {
        role: "user",
        content: [
          {
            type: "text",
            text: "Analyze this sales data. Show trends and create a visualization.",
          },
          { type: "container_upload", file_id: uploaded.id },
        ],
      },
    ],
    tools: [{ type: "code_execution_20260120", name: "code_execution" }],
  },
  { headers: { "anthropic-beta": "files-api-2025-04-14" } },
);
```

### Retrieve Generated Files

```typescript
import path from "path";
import fs from "fs";

const OUTPUT_DIR = "./claude_outputs";
await fs.promises.mkdir(OUTPUT_DIR, { recursive: true });

for (const block of response.content) {
  if (block.type === "bash_code_execution_tool_result") {
    const result = block.content;
    if (result.type === "bash_code_execution_result" && result.content) {
      for (const fileRef of result.content) {
        if (fileRef.type === "bash_code_execution_output") {
          const metadata = await client.beta.files.retrieveMetadata(
            fileRef.file_id,
          );
          const response = await client.beta.files.download(fileRef.file_id);
          const fileBytes = Buffer.from(await response.arrayBuffer());
          const safeName = path.basename(metadata.filename);
          if (!safeName || safeName === "." || safeName === "..") {
            console.warn(`Skipping invalid filename: ${metadata.filename}`);
            continue;
          }
          const outputPath = path.join(OUTPUT_DIR, safeName);
          await fs.promises.writeFile(outputPath, fileBytes);
          console.log(`Saved: ${outputPath}`);
        }
      }
    }
  }
}
```

### Container Reuse

```typescript
// First request: set up environment
const response1 = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 4096,
  messages: [
    {
      role: "user",
      content: "Install tabulate and create data.json with sample user data",
    },
  ],
  tools: [{ type: "code_execution_20260120", name: "code_execution" }],
});

// Reuse container
const containerId = response1.container.id;

const response2 = await client.messages.create({
  container: containerId,
  model: "claude-opus-4-6",
  max_tokens: 4096,
  messages: [
    {
      role: "user",
      content: "Read data.json and display as a formatted table",
    },
  ],
  tools: [{ type: "code_execution_20260120", name: "code_execution" }],
});
```

---

## Memory Tool

### Basic Usage (Memory Tool)

```typescript
const response = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 2048,
  messages: [
    {
      role: "user",
      content: "Remember that my preferred language is TypeScript.",
    },
  ],
  tools: [{ type: "memory_20250818", name: "memory" }],
});
```

### SDK Memory Helper

Use `betaMemoryTool` with a `MemoryToolHandlers` implementation:

```typescript
import {
  betaMemoryTool,
  type MemoryToolHandlers,
} from "@anthropic-ai/sdk/helpers/beta/memory";

const handlers: MemoryToolHandlers = {
  async view(command) { ... },
  async create(command) { ... },
  async str_replace(command) { ... },
  async insert(command) { ... },
  async delete(command) { ... },
  async rename(command) { ... },
};

const memory = betaMemoryTool(handlers);

const runner = client.beta.messages.toolRunner({
  model: "claude-opus-4-6",
  max_tokens: 2048,
  tools: [memory],
  messages: [{ role: "user", content: "Remember my preferences" }],
});

for await (const message of runner) {
  console.log(message);
}
```

For full implementation examples, use WebFetch:

- `https://github.com/anthropics/anthropic-sdk-typescript/blob/main/examples/tools-helpers-memory.ts`

---

## Structured Outputs

### JSON Outputs (Zod — Recommended)

```typescript
import Anthropic from "@anthropic-ai/sdk";
import { z } from "zod";
import { zodOutputFormat } from "@anthropic-ai/sdk/helpers/zod";

const ContactInfoSchema = z.object({
  name: z.string(),
  email: z.string(),
  plan: z.string(),
  interests: z.array(z.string()),
  demo_requested: z.boolean(),
});

const client = new Anthropic();

const response = await client.messages.parse({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  messages: [
    {
      role: "user",
      content:
        "Extract: Jane Doe (jane@co.com) wants Enterprise, interested in API and SDKs, wants a demo.",
    },
  ],
  output_config: {
    format: zodOutputFormat(ContactInfoSchema),
  },
});

console.log(response.parsed_output.name); // "Jane Doe"
```

### Strict Tool Use

```typescript
const response = await client.messages.create({
  model: "claude-opus-4-6",
  max_tokens: 1024,
  messages: [
    {
      role: "user",
      content: "Book a flight to Tokyo for 2 passengers on March 15",
    },
  ],
  tools: [
    {
      name: "book_flight",
      description: "Book a flight to a destination",
      strict: true,
      input_schema: {
        type: "object",
        properties: {
          destination: { type: "string" },
          date: { type: "string", format: "date" },
          passengers: {
            type: "integer",
            enum: [1, 2, 3, 4, 5, 6, 7, 8],
          },
        },
        required: ["destination", "date", "passengers"],
        additionalProperties: false,
      },
    },
  ],
});
```


---

<!-- merged from: apps-sdk-docs-workflow.md -->

﻿---
name: Apps SDK Docs Workflow
description: # Apps SDK Docs Workflow
 
 Use this reference to keep code generation aligned with current OpenAI Apps SDK docs.
---

# Apps SDK Docs Workflow

Use this reference to keep code generation aligned with current OpenAI Apps SDK docs.

## Always Fetch These Pages (Baseline)

- `https://developers.openai.com/apps-sdk/build/mcp-server/`
- `https://developers.openai.com/apps-sdk/build/chatgpt-ui/`
- `https://developers.openai.com/apps-sdk/build/examples/`
- `https://developers.openai.com/apps-sdk/plan/tools/`
- `https://developers.openai.com/apps-sdk/reference/`

## Fetch Conditionally (Greenfield / First Pass)

- `https://developers.openai.com/apps-sdk/quickstart/` for first implementation scaffolds and happy-path wiring
- `https://developers.openai.com/apps-sdk/deploy/` when the task includes local ChatGPT testing via tunnel, hosting, or production deployment planning
- `https://developers.openai.com/apps-sdk/deploy/submission/` when the task includes public launch, app review, or publishing steps
- `https://developers.openai.com/apps-sdk/app-submission-guidelines/` when the task includes submission readiness, policy/reliability checks, or review-risk reduction

## Suggested `openai-docs` / MCP Queries

Use focused searches before fetching:

- `ChatGPT Apps SDK build MCP server register resource template resourceUri outputTemplate`
- `ChatGPT Apps SDK build ChatGPT UI MCP Apps bridge ui/notifications/tool-result`
- `ChatGPT Apps SDK examples React widget upload modal Pizzaz`
- `Apps SDK define tools annotations readOnlyHint destructiveHint openWorldHint`
- `Apps SDK reference tool descriptor _meta ui.resourceUri openai/outputTemplate`
- `ChatGPT Apps SDK quickstart build web component tools/call`
- `ChatGPT app company knowledge compatibility search fetch tools`
- `platform MCP search tool fetch tool schema`
- `ChatGPT Apps SDK deploy app local development tunnel ngrok refresh connector`
- `ChatGPT Apps SDK submit app review prerequisites app submission guidelines`

## Docs-Derived Checklist (Current Guidance)

### Archetype / Shape

- Classify the request into one primary app archetype before choosing examples or scaffolds
- Keep the repo shape consistent with that archetype instead of inventing a new structure for each prompt

### Server

- Register the widget resource/template with the MCP Apps UI MIME type (`text/html;profile=mcp-app`) or `RESOURCE_MIME_TYPE` when using `@modelcontextprotocol/ext-apps/server`
- Version template URIs when widget HTML or JS or CSS changes in a breaking way (treat URI as cache key)
- Set `_meta.ui.resourceUri` on render tools; optionally mirror `_meta["openai/outputTemplate"]` for ChatGPT compatibility
- Design tool handlers to be idempotent because the model may retry calls
- Keep `structuredContent` concise and move widget-only payloads to `_meta`

### Tool Design

- Plan one user intent per tool
- Use action-oriented names and precise descriptions
- Set tool impact hints accurately (`readOnlyHint`, `destructiveHint`, `openWorldHint`)
- Split data and render tools so that the model can fetch the data and look at it before choosing to render the widget UI or not
- Make the widget input a list of unique identifiers (e.g. `propertyIds` for a render property map widget that takes IDs returned from the fetch properties nearby tool) if you want to make sure the widget only renders 1p data; make the widget input semantically relevant if you want to allow the model to render the widget with generated data (e.g. `questionAndAnswerPairs` for a flashcards widget)
- For connector-like, data-only, sync-oriented, or company-knowledge-style apps, prefer the standard `search` and `fetch` tools by default

### UI

- Prefer the MCP Apps bridge (`ui/*` notifications + `tools/call`) for new apps
- Prefer `ui/message` for follow-up messaging in baseline examples; treat `window.openai.sendFollowUpMessage` as optional ChatGPT-specific compatibility
- Treat `window.openai` as compatibility plus optional ChatGPT extensions
- Render from `structuredContent` and treat host-delivered data as untrusted input
- Use `ui/update-model-context` only for UI state the model should reason about

### Starting Point Selection

- Check `apps-sdk/build/examples` and the official examples repo before generating a greenfield scaffold from scratch
- Prefer the smallest upstream example that matches the requested stack and interaction pattern
- Use the local fallback scaffold only when upstream examples are a poor fit or undesirable for the request

### Resource Metadata / Security

- Set `_meta.ui.csp.connectDomains` and `_meta.ui.csp.resourceDomains` exactly
- Avoid `frameDomains` unless iframe embedding is central to the experience
- Set `_meta.ui.domain` for submission-ready apps
- Always set `openai/widgetDescription` to inform the model what the widget is to be used for

### Developer Mode / Local Testing

- Run the MCP server locally on `http://localhost:<port>/mcp`
- Expose it with a public HTTPS tunnel for ChatGPT access during development
- Use the public URL + `/mcp` when adding the app in ChatGPT settings
- Include ChatGPT Developer Mode setup and app creation steps in implementation handoff
- Remind users to refresh the app after MCP tool/metadata changes
- Note terminology differences when relevant: some docs/screenshots may still say "connector" while product UI uses "app"

### Validation

- Validate against a minimum working repo contract, not just file creation
- Run the cheapest useful syntax or compile check first
- If feasible, confirm the local `/mcp` route responds before calling the result “working”
- If you cannot run a deeper check, say so explicitly
- If the app is connector-like or sync-oriented, verify the `search` and `fetch` tool shapes against the standard

### Production Hosting / Deploy

- Prefer a stable public HTTPS endpoint with reliable TLS and low-latency streaming `/mcp`
- Document platform-specific secrets handling and environment variables
- Include logging/metrics expectations for debugging production tool calls
- Re-test the hosted endpoint in ChatGPT Developer Mode before submission

### Submission / Review

- Read `deploy/submission` and `app-submission-guidelines` together (process + policy requirements)
- Check org verification and Owner-role prerequisites before generating submission steps
- Ensure the endpoint is public production infrastructure (not localhost/tunnel/testing URLs)
- Ensure CSP is defined and accurate for submission
- Prepare submission artifacts (metadata, screenshots, privacy policy/support contacts, test prompts/responses)
- If auth is required, prepare review-safe demo credentials and validate them outside internal networks

## Generation Pattern

1. Classify the app archetype.
2. Fetch docs with `$openai-docs`.
3. Check official examples before inventing a scaffold from scratch.
4. Summarize relevant constraints and metadata keys.
5. Propose tool plan and architecture.
6. Adapt the closest example or use the local fallback scaffold.
7. Generate or patch the server scaffold.
8. Generate or patch the widget scaffold.
9. Validate the repo against the minimum working contract.
10. Add local run + tunnel + ChatGPT Developer Mode app setup instructions.
11. Add hosting/deployment guidance when the task implies go-live.
12. Add submission/readiness steps when the user intends public distribution.
13. Call out compatibility aliases vs MCP Apps standard fields.

## Starter Scaffold Script

- Use `./scripts/scaffold_node_ext_apps.mjs <output-dir> --app-name <name>` only when the user wants a greenfield Node + `@modelcontextprotocol/ext-apps` starter and no upstream example is the better fit.
- If the file is not executable in the current environment, fall back to `node scripts/scaffold_node_ext_apps.mjs <output-dir> --app-name <name>`.
- The script generates `package.json`, `tsconfig.json`, `public/widget.html`, and `src/server.ts`.
- It intentionally uses the MCP Apps bridge by default, keeps follow-up messaging on `ui/message`, and limits `window.openai` to optional host signals/extensions.
- After generation, compare the output against the docs you fetched and adjust package versions, metadata, transport details, or URI/versioning if the docs changed.


---

<!-- merged from: develop-web-game.md -->

﻿---
name: "develop-web-game"
description: "Use when Codex is building or iterating on a web game (HTML/JS) and needs a reliable development + testing loop: implement small changes, run a Playwright-based test script with short input bursts and intentional pauses, inspect screenshots/text, and review console errors with render_game_to_text."
---

# Develop Web Game

Build games in small steps and validate every change. Treat each iteration as: implement → act → pause → observe → adjust.

## Skill paths (set once)

```bash
export CODEX_HOME="${CODEX_HOME:-$HOME/.codex}"
export WEB_GAME_CLIENT="$CODEX_HOME/skills/develop-web-game/scripts/web_game_playwright_client.js"
export WEB_GAME_ACTIONS="$CODEX_HOME/skills/develop-web-game/references/action_payloads.json"
```

User-scoped skills install under `$CODEX_HOME/skills` (default: `~/.codex/skills`).

## Workflow

1. **Pick a goal.** Define a single feature or behavior to implement.
2. **Implement small.** Make the smallest change that moves the game forward.
3. **Ensure integration points.** Provide a single canvas and `window.render_game_to_text` so the test loop can read state.
4. **Add `window.advanceTime(ms)`.** Strongly prefer a deterministic step hook so the Playwright script can advance frames reliably; without it, automated tests can be flaky.
5. **Initialize progress.md.** If `progress.md` exists, read it first and confirm the original user prompt is recorded at the top (prefix with `Original prompt:`). Also note any TODOs and suggestions left by the previous agent. If missing, create it and write `Original prompt: <prompt>` at the top before appending updates.
6. **Verify Playwright availability.** Ensure `playwright` is available (local dependency or global install). If unsure, check `npx` first.
7. **Run the Playwright test script.** You must run `$WEB_GAME_CLIENT` after each meaningful change; do not invent a new client unless required.
8. **Use the payload reference.** Base actions on `$WEB_GAME_ACTIONS` to avoid guessing keys.
9. **Inspect state.** Capture screenshots and text state after each burst.
10. **Inspect screenshots.** Open the latest screenshot, verify expected visuals, fix any issues, and rerun the script. Repeat until correct.
11. **Verify controls and state (multi-step focus).** Exhaustively exercise all important interactions. For each, think through the full multi-step sequence it implies (cause → intermediate states → outcome) and verify the entire chain works end-to-end. Confirm `render_game_to_text` reflects the same state shown on screen. If anything is off, fix and rerun.
    Examples of important interactions: move, jump, shoot/attack, interact/use, select/confirm/cancel in menus, pause/resume, restart, and any special abilities or puzzle actions defined by the request. Multi-step examples: shooting an enemy should reduce its health; when health reaches 0 it should disappear and update the score; collecting a key should unlock a door and allow level progression.
12. **Check errors.** Review console errors and fix the first new issue before continuing.
13. **Reset between scenarios.** Avoid cross-test state when validating distinct features.
14. **Iterate with small deltas.** Change one variable at a time (frames, inputs, timing, positions), then repeat steps 7–13 until stable.

Example command (actions required):

```bash
node "$WEB_GAME_CLIENT" --url http://localhost:5173 --actions-file "$WEB_GAME_ACTIONS" --click-selector "#start-btn" --iterations 3 --pause-ms 250
```

Example actions (inline JSON):

```json
{
  "steps": [
    { "buttons": ["left_mouse_button"], "frames": 2, "mouse_x": 120, "mouse_y": 80 },
    { "buttons": [], "frames": 6 },
    { "buttons": ["right"], "frames": 8 },
    { "buttons": ["space"], "frames": 4 }
  ]
}
```

## Test Checklist

Test any new features added for the request and any areas your logic changes could affect. Identify issues, fix them, and re-run the tests to confirm they’re resolved.

Examples of things to test:

- Primary movement/interaction inputs (e.g., move, jump, shoot, confirm/select).
- Win/lose or success/fail transitions.
- Score/health/resource changes.
- Boundary conditions (collisions, walls, screen edges).
- Menu/pause/start flow if present.
- Any special actions tied to the request (powerups, combos, abilities, puzzles, timers).

## Test Artifacts to Review

- Latest screenshots from the Playwright run.
- Latest `render_game_to_text` JSON output.
- Console error logs (fix the first new error before continuing).
You must actually open and visually inspect the latest screenshots after running the Playwright script, not just generate them. Ensure everything that should be visible on screen is actually visible. Go beyond the start screen and capture gameplay screenshots that cover all newly added features. Treat the screenshots as the source of truth; if something is missing, it is missing in the build. If you suspect a headless/WebGL capture issue, rerun the Playwright script in headed mode and re-check. Fix and rerun in a tight loop until the screenshots and text state look correct. Once fixes are verified, re-test all important interactions and controls, confirm they work, and ensure your changes did not introduce regressions. If they did, fix them and rerun everything in a loop until interactions, text state, and controls all work as expected. Be exhaustive in testing controls; broken games are not acceptable.

## Core Game Guidelines

### Canvas + Layout

- Prefer a single canvas centered in the window.

### Visuals

- Keep on-screen text minimal; show controls on a start/menu screen rather than overlaying them during play.
- Avoid overly dark scenes unless the design calls for it. Make key elements easy to see.
- Draw the background on the canvas itself instead of relying on CSS backgrounds.

### Text State Output (render_game_to_text)

Expose a `window.render_game_to_text` function that returns a concise JSON string representing the current game state. The text should include enough information to play the game without visuals.

Minimal pattern:

```js
function renderGameToText() {
  const payload = {
    mode: state.mode,
    player: { x: state.player.x, y: state.player.y, r: state.player.r },
    entities: state.entities.map((e) => ({ x: e.x, y: e.y, r: e.r })),
    score: state.score,
  };
  return JSON.stringify(payload);
}
window.render_game_to_text = renderGameToText;
```

Keep the payload succinct and biased toward on-screen/interactive elements. Prefer current, visible entities over full history.
Include a clear coordinate system note (origin and axis directions), and encode all player-relevant state: player position/velocity, active obstacles/enemies, collectibles, timers/cooldowns, score, and any mode/state flags needed to make correct decisions. Avoid large histories; only include what's currently relevant and visible.

### Time Stepping Hook

Provide a deterministic time-stepping hook so the Playwright client can advance the game in controlled increments. Expose `window.advanceTime(ms)` (or a thin wrapper that forwards to your game update loop) and have the game loop use it when present.
The Playwright test script uses this hook to step frames deterministically during automated testing.

Minimal pattern:

```js
window.advanceTime = (ms) => {
  const steps = Math.max(1, Math.round(ms / (1000 / 60)));
  for (let i = 0; i < steps; i++) update(1 / 60);
  render();
};
```

### Fullscreen Toggle

- Use a single key (prefer `f`) to toggle fullscreen on/off.
- Allow `Esc` to exit fullscreen.
- When fullscreen toggles, resize the canvas/rendering so visuals and input mapping stay correct.

## Progress Tracking

Create a `progress.md` file if it doesn't exist, and append TODOs, notes, gotchas, and loose ends as you go so another agent can pick up seamlessly.
If a `progress.md` file already exists, read it first, including the original user prompt at the top (you may be continuing another agent's work). Do not overwrite the original prompt; preserve it.
Update `progress.md` after each meaningful chunk of work (feature added, bug found, test run, or decision made).
At the end of your work, leave TODOs and suggestions for the next agent in `progress.md`.

## Playwright Prerequisites

- Prefer a local `playwright` dependency if the project already has it.
- If unsure whether Playwright is available, check for `npx`:

```text
  command -v npx >/dev/null 2>&1
  ```

- If `npx` is missing, install Node/npm and then install Playwright globally:

```bash
  npm install -g @playwright/mcp@latest
  ```

- Do not switch to `@playwright/test` unless explicitly asked; stick to the client script.

## Scripts

- `$WEB_GAME_CLIENT` (installed default: `$CODEX_HOME/skills/develop-web-game/scripts/web_game_playwright_client.js`) — Playwright-based action loop with virtual-time stepping, screenshot capture, and console error buffering. You must pass an action burst via `--actions-file`, `--actions-json`, or `--click`.

## References

- `$WEB_GAME_ACTIONS` (installed default: `$CODEX_HOME/skills/develop-web-game/references/action_payloads.json`) — example action payloads (keyboard + mouse, per-frame capture). Use these to build your burst.


---

<!-- merged from: upstream-example-workflow.md -->

﻿---
name: Upstream Example Workflow
description: # Upstream Example Workflow
 
 Load this reference when starting a greenfield ChatGPT app or when deciding whether to adapt an upstream example or use the local fallback scaffold.
---

# Upstream Example Workflow

Load this reference when starting a greenfield ChatGPT app or when deciding whether to adapt an upstream example or use the local fallback scaffold.

## Default Order

Prefer these starting points in order:

1. Official OpenAI Apps SDK examples
2. Version-matched `@modelcontextprotocol/ext-apps` examples
3. Local `scripts/scaffold_node_ext_apps.mjs` fallback

This keeps the skill aligned with current docs and maintained example code while still preserving a low-dependency fallback when examples are not a good fit.

## Choose The Right Source

### 1. Official OpenAI examples

Prefer these when:

- The app is clearly ChatGPT-facing
- The user wants a polished UI or React component
- The task involves file upload, modal flows, display-mode changes, or other ChatGPT extensions
- The docs/examples page already shows a similar interaction pattern

Typical sources:

- `https://developers.openai.com/apps-sdk/build/examples/`
- `https://github.com/openai/openai-apps-sdk-examples`
- `https://developers.openai.com/apps-sdk/quickstart/` for the smallest vanilla baseline

### 2. `@modelcontextprotocol/ext-apps` examples

Prefer these when:

- The user needs a lower-level MCP Apps baseline
- Portability across MCP Apps-compatible hosts matters more than ChatGPT-specific polish
- You want version-matched examples close to the installed `@modelcontextprotocol/ext-apps` package shape

This follows the same basic idea as the upstream `create-mcp-app` skill: use maintained examples as the starting point, then adapt them.

Typical examples from upstream flows:

- `examples/demo-vanilla-html`
- `examples/demo-react-simple`
- `examples/demo-connectors-api`

### 3. Local fallback scaffold

Use `scripts/scaffold_node_ext_apps.mjs` when:

- No close upstream example exists
- The user wants a tiny Node + vanilla HTML starter
- Network/example retrieval is undesirable
- You need a throwaway starter to patch quickly during a live coding task

Do not prefer the local scaffold just because it is available. It is the fallback, not the default.

## Adaptation Rules

- Copy the smallest matching example, not the entire showcase app.
- Remove unrelated demo tools, assets, and routes immediately.
- Keep the upstream file structure when it is already clean and docs-aligned.
- Reconcile the copied example with the current docs before finishing:
  - tool names and descriptions
  - annotations (`readOnlyHint`, `destructiveHint`, `openWorldHint`, `idempotentHint` when true)
  - `_meta.ui.resourceUri` and optional `_meta["openai/outputTemplate"]`
  - resource `_meta.ui.csp`, `_meta.ui.domain`, and `openai/widgetDescription`
  - URI versioning for template changes
  - local run/test instructions
- State which example you chose and why.
- If you rely on upstream code, note the source repo and branch/tag/commit when practical; avoid silently depending on a floating example shape for long-lived work.

## Minimal Selection Heuristic

- If the user asks for **React + polished UI**, start with official OpenAI examples.
- If the user asks for **vanilla HTML + tiny demo**, start with the quickstart example; use the local fallback scaffold only if the quickstart is still too opinionated or unavailable.
- If the user asks for **portable MCP Apps wiring**, start with `@modelcontextprotocol/ext-apps` examples.
- If the user already has an app, adapt their code directly instead of importing a new example.


---

<!-- merged from: search-and-fetch-standard.md -->

﻿---
name: Search And Fetch Standard
description: # Search And Fetch Standard
 
 Load this reference when the app is connector-like, data-only, sync-oriented, or meant to work well with company knowledge or deep research.
---

# Search And Fetch Standard

Load this reference when the app is connector-like, data-only, sync-oriented, or meant to work well with company knowledge or deep research.

## Default Rule

If the app is primarily a read-only knowledge source, do not invent custom equivalents to `search` and `fetch`.

Default to implementing the standard `search` and `fetch` tools exactly, then add other tools only if the use case clearly needs them.

## When This Applies

Use the standard by default when the request is about:

- a data-only app
- a sync app
- a company knowledge source
- deep research compatibility
- a connector-like integration over documents, tickets, wiki pages, CRM records, or similar read-only data

## Tool Requirements

### `search`

- Read-only tool
- Takes a single query string
- Returns exactly one MCP content item with `type: "text"`
- That text is a JSON-encoded object with:
  - `results`
  - each result has `id`, `title`, and `url`

### `fetch`

- Read-only tool
- Takes a single document/item id string
- Returns exactly one MCP content item with `type: "text"`
- That text is a JSON-encoded object with:
  - `id`
  - `title`
  - `text`
  - `url`
  - optional `metadata`

## Implementation Rules

- Match the schema exactly when the app is intended for company knowledge or deep research compatibility.
- Use canonical `url` values for citations.
- Mark these tools as read-only.
- Prefer these names exactly: `search` and `fetch`.
- If you add other read-only tools, they should complement the standard rather than replace it.

## Validation Checks

When `search` and `fetch` are relevant, verify:

- both tools exist
- they are read-only
- their input shapes match the standard
- their returned payloads are wrapped as one `content` item with JSON-encoded `text`
- result URLs are canonical enough for citation use

## Source

This standard is described in:

- `https://developers.openai.com/apps-sdk/build/mcp-server/#company-knowledge-compatibility`
- `https://platform.openai.com/docs/mcp`