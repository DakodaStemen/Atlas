---
name: mcp-client-patterns
description: Comprehensive reference for building and integrating MCP (Model Context Protocol) clients. Covers the full protocol lifecycle, all transport types, tool discovery and invocation, resource reading, prompt templates, server-initiated sampling, and security practices — with working Python and TypeScript code patterns.
domain: ai-ml
category: mcp
tags: [MCP, Model-Context-Protocol, client, tool-discovery, sampling, stdio, SSE, HTTP, JSON-RPC, LLM-integration]
triggers:
  - "build mcp client"
  - "mcp client integration"
  - "connect to mcp server"
  - "tools/list tools/call"
  - "mcp sampling"
  - "mcp transport"
  - "model context protocol client"
  - "mcp session lifecycle"
  - "mcp tool discovery"
  - "mcp resource reading"
---

# MCP Client Integration Patterns

Spec version: **2025-11-25**. All message shapes here are authoritative for that version.

---

## 1. Architecture: Hosts, Clients, and Servers

MCP distinguishes three roles:

- **Host**: the LLM application (e.g. Claude Desktop, a custom chatbot). It owns the user session and enforces consent.
- **Client**: a connector embedded in the host that manages one connection to one server. A host can instantiate many clients.
- **Server**: an independent process that exposes tools, resources, and/or prompts. A server can serve many clients simultaneously.

The protocol is transport-agnostic JSON-RPC 2.0. Every message is UTF-8. Three message types exist:

```json
// Request (must have id, id must not be null or reused in session)
{ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} }

// Result response
{ "jsonrpc": "2.0", "id": 1, "result": { ... } }

// Error response
{ "jsonrpc": "2.0", "id": 1, "error": { "code": -32602, "message": "..." } }

// Notification (no id, no response expected)
{ "jsonrpc": "2.0", "method": "notifications/initialized" }
```

---

## 2. Transports

### 2.1 stdio

The client launches the server as a subprocess. Messages travel over stdin/stdout, newline-delimited. stderr is for server logs only — the client may capture or ignore it but must not treat stderr output as an error signal.

Shutdown: client closes the server's stdin, waits for exit, sends SIGTERM if needed, then SIGKILL.

```text
Client                   Server subprocess
  |---launch process------->|
  |---write to stdin------->|   (JSON-RPC message, newline-terminated)
  |<--read from stdout------|   (JSON-RPC message, newline-terminated)
  |<--read from stderr------|   (logs, ignored for protocol purposes)
  |---close stdin---------->|   (begins shutdown)
```

**When to use**: local tools, CLI integrations, any scenario where the client controls the server process. Prefer stdio whenever possible — it is simpler and avoids network authentication concerns.

### 2.2 Streamable HTTP (current standard, replaces HTTP+SSE from 2024-11-05)

The server exposes a single HTTP endpoint (e.g. `https://example.com/mcp`) supporting both POST and GET.

**Sending messages to server**: every client message is a new HTTP POST. The client must include:

```text
Accept: application/json, text/event-stream
MCP-Protocol-Version: 2025-11-25
MCP-Session-Id: <session-id>   (after initialization)
```

The server responds with either `Content-Type: application/json` (single response) or `Content-Type: text/event-stream` (SSE stream for streaming responses and server-push).

**Listening for server-initiated messages**: client issues an HTTP GET to the MCP endpoint with `Accept: text/event-stream`. The server can push JSON-RPC requests and notifications on this stream.

**Session management**: on a successful `InitializeResult`, the server may return `MCP-Session-Id` in the response header. The client must include that header on all subsequent requests. If the server returns 404 for a request with a session ID, the client must re-initialize without a session ID.

Session termination: client sends `HTTP DELETE` to the MCP endpoint with the session ID header.

**Resumability**: servers may attach `id` fields to SSE events. On reconnect, the client sends `Last-Event-ID` in a GET request; the server may replay missed events.

**DNS rebinding protection**: servers must validate the `Origin` header and reject invalid origins with HTTP 403. Servers should bind locally to 127.0.0.1, not 0.0.0.0.

### 2.3 Legacy HTTP+SSE (protocol version 2024-11-05)

Deprecated in favor of Streamable HTTP. When a client wants to support older servers: try POST first; if it fails with 400/404/405, issue a GET and look for an `endpoint` SSE event to discover the POST URL.

### 2.4 Custom transports

Implementations may add custom transports as long as they preserve the JSON-RPC message format and lifecycle requirements.

---

## 3. Session Lifecycle

### 3.1 Initialization handshake

The client sends `initialize` first. No other requests (except pings) may be sent until the server has responded.

```json
// Client -> Server
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "roots": { "listChanged": true },
      "sampling": {},
      "elicitation": {}
    },
    "clientInfo": {
      "name": "MyClient",
      "version": "1.0.0"
    }
  }
}

// Server -> Client
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-11-25",
    "capabilities": {
      "tools": { "listChanged": true },
      "resources": { "subscribe": true, "listChanged": true },
      "prompts": { "listChanged": true },
      "logging": {}
    },
    "serverInfo": {
      "name": "ExampleServer",
      "version": "1.0.0"
    },
    "instructions": "Optional guidance for the client"
  }
}

// Client -> Server (must be sent after initialize response before any other requests)
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized"
}
```

**Version negotiation**: client sends the latest version it supports. Server responds with the version it will use. If the client does not support that version, it must disconnect.

**Capability negotiation**: both sides only use capabilities declared during initialization. Check `result.capabilities` before calling any optional method.

### 3.2 Operation phase

Normal request/response cycles. Both sides must respect the negotiated protocol version and capabilities.

### 3.3 Timeouts and cancellation

All requests should have timeouts. When a timeout fires, send a cancellation notification:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/cancelled",
  "params": {
    "requestId": 42,
    "reason": "Client timeout"
  }
}
```

Progress notifications from the server (for long-running operations) may reset the timeout clock, but a maximum overall timeout should always apply.

### 3.4 Shutdown

- **stdio**: close stdin, wait, SIGTERM, SIGKILL.
- **HTTP**: close the HTTP connection(s) and send `HTTP DELETE` with session ID.

---

## 4. Tool Discovery and Invocation

Tools are **model-controlled**: the LLM decides when to call them. The client handles the protocol; the host provides consent UI.

### 4.1 Listing tools

```json
// Request (supports cursor-based pagination)
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": { "cursor": "optional-cursor-value" }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "get_weather",
        "title": "Weather Information Provider",
        "description": "Get current weather for a location",
        "inputSchema": {
          "type": "object",
          "properties": {
            "location": { "type": "string", "description": "City name or zip code" }
          },
          "required": ["location"]
        },
        "outputSchema": {
          "type": "object",
          "properties": {
            "temperature": { "type": "number" },
            "conditions": { "type": "string" }
          }
        }
      }
    ],
    "nextCursor": "next-page-cursor"
  }
}
```

Tool fields:

- `name`: 1–128 chars, case-sensitive, characters `[A-Za-z0-9_\-.]`, no spaces. Must be unique within the server.
- `title`: optional human-readable display name.
- `description`: human-readable, used by the LLM to decide when to call the tool.
- `inputSchema`: JSON Schema 2020-12 by default. For no-parameter tools use `{"type": "object", "additionalProperties": false}`.
- `outputSchema`: optional; if present, servers must return structured results conforming to it, and clients should validate.
- `annotations`: optional hints about behavior (treat as untrusted unless server is trusted).

Paginate with `nextCursor` until no cursor is returned.

### 4.2 Calling a tool

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "get_weather",
    "arguments": { "location": "New York" }
  }
}

// Success response
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      { "type": "text", "text": "Temperature: 72°F, partly cloudy" }
    ],
    "isError": false
  }
}

// Tool execution error (still a result, not a JSON-RPC error)
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      { "type": "text", "text": "Invalid date: must be in the future." }
    ],
    "isError": true
  }
}

// Protocol error (malformed request, unknown tool)
{
  "jsonrpc": "2.0",
  "id": 3,
  "error": { "code": -32602, "message": "Unknown tool: invalid_tool_name" }
}
```

#### Two-tier error model

- `isError: true` in the result = tool execution failed (API error, bad input, business logic). Feed this back to the LLM for self-correction and retry.
- JSON-RPC error response = protocol-level failure (unknown method, malformed params). Less likely to be recoverable by the model.

### 4.3 Content types in tool results

```json
// Text
{ "type": "text", "text": "plain text" }

// Image (base64-encoded)
{ "type": "image", "data": "base64...", "mimeType": "image/png" }

// Audio (base64-encoded)
{ "type": "audio", "data": "base64...", "mimeType": "audio/wav" }

// Resource link (reference to a resource the client can fetch)
{ "type": "resource_link", "uri": "file:///project/main.rs", "name": "main.rs", "mimeType": "text/x-rust" }

// Embedded resource (resource content inlined)
{
  "type": "resource",
  "resource": {
    "uri": "file:///project/main.rs",
    "mimeType": "text/x-rust",
    "text": "fn main() { ... }"
  }
}

// Structured (machine-readable, alongside text for backwards compat)
// Returned in result.structuredContent as a JSON object
```

### 4.4 Tool list change notification

When the server declares `tools: { listChanged: true }`, it sends this when tools change. The client should re-run `tools/list`.

```json
{ "jsonrpc": "2.0", "method": "notifications/tools/list_changed" }
```

---

## 5. Resource Reading

Resources are **application-controlled**: the host decides which resources to attach to the model context.

### 5.1 Listing resources

```json
// Request (supports cursor-based pagination)
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "resources/list",
  "params": { "cursor": "optional-cursor" }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "resources": [
      {
        "uri": "file:///project/src/main.rs",
        "name": "main.rs",
        "title": "Application Entry Point",
        "description": "Primary application entry point",
        "mimeType": "text/x-rust",
        "size": 1024
      }
    ],
    "nextCursor": "next-page-cursor"
  }
}
```

### 5.2 Reading a resource

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "resources/read",
  "params": { "uri": "file:///project/src/main.rs" }
}

// Response — text content
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "contents": [
      {
        "uri": "file:///project/src/main.rs",
        "mimeType": "text/x-rust",
        "text": "fn main() {\n    println!(\"Hello world!\");\n}"
      }
    ]
  }
}

// Response — binary content
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "contents": [
      {
        "uri": "file:///image.png",
        "mimeType": "image/png",
        "blob": "base64-encoded-data"
      }
    ]
  }
}
```

### 5.3 Resource URI templates

```json
// Request
{ "jsonrpc": "2.0", "id": 6, "method": "resources/templates/list" }

// Response
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "resourceTemplates": [
      {
        "uriTemplate": "file:///{path}",
        "name": "Project Files",
        "description": "Access files in the project directory",
        "mimeType": "application/octet-stream"
      }
    ]
  }
}
```

URI templates follow RFC 6570. Arguments may be auto-completed via the `completion/complete` API.

### 5.4 Subscriptions and change notifications

If the server declares `resources: { subscribe: true }`:

```json
// Subscribe
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "resources/subscribe",
  "params": { "uri": "file:///project/src/main.rs" }
}

// Server notification on change
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": { "uri": "file:///project/src/main.rs" }
}
```

After receiving `resources/updated`, re-read the resource with `resources/read`.

### 5.5 Resource annotations

Annotations help clients prioritize context:

```json
{
  "uri": "file:///README.md",
  "name": "README.md",
  "mimeType": "text/markdown",
  "annotations": {
    "audience": ["user", "assistant"],
    "priority": 0.8,
    "lastModified": "2025-01-12T15:00:58Z"
  }
}
```

- `audience`: `"user"` (display to human), `"assistant"` (include in model context), or both.
- `priority`: 0.0–1.0. 1.0 = required, 0.0 = entirely optional.
- `lastModified`: ISO 8601 timestamp.

### 5.6 Common URI schemes

- `file://` — filesystem or filesystem-like resources
- `https://` — use only when the client can fetch the resource directly without the server
- `git://` — git version control integration
- Custom schemes must conform to RFC 3986

### 5.7 Resource error codes

- `-32002`: Resource not found
- `-32603`: Internal server error

---

## 6. Prompt Templates

Prompts are **user-controlled**: users explicitly select them (e.g. slash commands).

### 6.1 Listing prompts

```json
// Request
{ "jsonrpc": "2.0", "id": 8, "method": "prompts/list", "params": { "cursor": "..." } }

// Response
{
  "jsonrpc": "2.0",
  "id": 8,
  "result": {
    "prompts": [
      {
        "name": "code_review",
        "title": "Request Code Review",
        "description": "Analyze code quality and suggest improvements",
        "arguments": [
          { "name": "code", "description": "The code to review", "required": true }
        ]
      }
    ],
    "nextCursor": "next-page-cursor"
  }
}
```

### 6.2 Getting a prompt

```json
// Request
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "prompts/get",
  "params": {
    "name": "code_review",
    "arguments": { "code": "def hello():\n    print('world')" }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": 9,
  "result": {
    "description": "Code review prompt",
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "Please review this Python code:\ndef hello():\n    print('world')"
        }
      }
    ]
  }
}
```

Prompt messages support the same content types as tool results: `text`, `image`, `audio`, and embedded `resource`. Messages have `role` of either `"user"` or `"assistant"`.

Servers should validate all arguments and return `-32602` for missing required arguments or invalid prompt names.

---

## 7. Sampling (Server-Initiated LLM Calls)

Sampling is a **client capability** — the server sends `sampling/createMessage` to ask the client's LLM to generate a completion. This allows servers to implement agentic behavior without holding API keys.

The client **must** always gate sampling through a human-in-the-loop: show the request to the user, allow them to modify it, show the response, allow approval before returning it to the server.

### 7.1 Client capability declaration

```json
{
  "capabilities": {
    "sampling": {},
    "sampling": { "tools": {} }    // also declare this to receive tool-enabled sampling requests
  }
}
```

### 7.2 Basic sampling request

```json
// Server -> Client
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "sampling/createMessage",
  "params": {
    "messages": [
      {
        "role": "user",
        "content": { "type": "text", "text": "What is the capital of France?" }
      }
    ],
    "modelPreferences": {
      "hints": [
        { "name": "claude-3-sonnet" },
        { "name": "claude" }
      ],
      "costPriority": 0.3,
      "speedPriority": 0.8,
      "intelligencePriority": 0.5
    },
    "systemPrompt": "You are a helpful assistant.",
    "maxTokens": 100
  }
}

// Client -> Server
{
  "jsonrpc": "2.0",
  "id": 10,
  "result": {
    "role": "assistant",
    "content": { "type": "text", "text": "The capital of France is Paris." },
    "model": "claude-3-sonnet-20240307",
    "stopReason": "endTurn"
  }
}
```

### 7.3 Model preferences

Servers cannot name a specific model (the client may not have access to it). Instead they express priorities (all values 0.0–1.0):

- `costPriority`: higher = prefer cheaper models
- `speedPriority`: higher = prefer lower-latency models
- `intelligencePriority`: higher = prefer more capable models
- `hints`: ordered list of name substrings (e.g. `"claude-3-sonnet"`). The client may map to equivalent models from other providers. Hints are advisory.

### 7.4 Sampling with tools (agentic loops)

When the client declares `sampling.tools` capability, servers may include a `tools` array and `toolChoice` in the sampling request. The LLM can then call tools, receive results, and continue — all within the sampling flow.

```json
// Server -> Client: request with tools
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "sampling/createMessage",
  "params": {
    "messages": [{ "role": "user", "content": { "type": "text", "text": "Weather in Paris and London?" } }],
    "tools": [
      {
        "name": "get_weather",
        "description": "Get current weather for a city",
        "inputSchema": {
          "type": "object",
          "properties": { "city": { "type": "string" } },
          "required": ["city"]
        }
      }
    ],
    "toolChoice": { "mode": "auto" },
    "maxTokens": 1000
  }
}

// Client -> Server: LLM wants to call tools
{
  "jsonrpc": "2.0",
  "id": 11,
  "result": {
    "role": "assistant",
    "content": [
      { "type": "tool_use", "id": "call_abc123", "name": "get_weather", "input": { "city": "Paris" } },
      { "type": "tool_use", "id": "call_def456", "name": "get_weather", "input": { "city": "London" } }
    ],
    "model": "claude-3-sonnet-20240307",
    "stopReason": "toolUse"
  }
}
```

The server then executes the tools and sends a follow-up `sampling/createMessage` with tool results appended as user messages. Key constraint: **a user message containing `tool_result` items must contain only tool results** — no mixing with text or image content.

`toolChoice` modes:

- `{ "mode": "auto" }`: model decides (default)
- `{ "mode": "required" }`: model must use at least one tool
- `{ "mode": "none" }`: model must not use tools (use on final iteration to force a text response)

### 7.5 Sampling error codes

- `-1`: user rejected the sampling request
- `-32602`: invalid params (tool result missing, or tool results mixed with other content)

---

## 8. Roots (Client-Exposed Filesystem Boundaries)

Clients may expose filesystem roots to tell the server where it is allowed to operate. Servers request roots; clients respond.

```json
// Server -> Client
{ "jsonrpc": "2.0", "id": 12, "method": "roots/list" }

// Client -> Server
{
  "jsonrpc": "2.0",
  "id": 12,
  "result": {
    "roots": [
      { "uri": "file:///home/user/projects/myproject", "name": "My Project" }
    ]
  }
}

// Client -> Server when roots change
{ "jsonrpc": "2.0", "method": "notifications/roots/list_changed" }
```

Root URIs must be `file://` URIs. The client must validate all root URIs to prevent path traversal.

---

## 9. Building a Custom MCP Client

### 9.1 Python (official SDK)

```python
import asyncio
from contextlib import AsyncExitStack
from typing import Optional

from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client
from anthropic import Anthropic

class MCPClient:
    def __init__(self):
        self.session: Optional[ClientSession] = None
        self.exit_stack = AsyncExitStack()
        self.anthropic = Anthropic()

    async def connect_to_server(self, command: str, args: list[str]):
        server_params = StdioServerParameters(command=command, args=args, env=None)
        stdio_transport = await self.exit_stack.enter_async_context(
            stdio_client(server_params)
        )
        read, write = stdio_transport
        self.session = await self.exit_stack.enter_async_context(
            ClientSession(read, write)
        )
        await self.session.initialize()

    async def list_tools(self):
        response = await self.session.list_tools()
        return response.tools

    async def call_tool(self, name: str, arguments: dict):
        result = await self.session.call_tool(name, arguments)
        if result.isError:
            # Feed error content back to LLM for self-correction
            pass
        return result.content

    async def list_resources(self):
        response = await self.session.list_resources()
        return response.resources

    async def read_resource(self, uri: str):
        response = await self.session.read_resource(uri)
        return response.contents

    async def list_prompts(self):
        response = await self.session.list_prompts()
        return response.prompts

    async def get_prompt(self, name: str, arguments: dict):
        response = await self.session.get_prompt(name, arguments)
        return response.messages

    async def process_query_with_tools(self, query: str) -> str:
        tools = await self.list_tools()
        available_tools = [
            {"name": t.name, "description": t.description, "input_schema": t.inputSchema}
            for t in tools
        ]

        messages = [{"role": "user", "content": query}]
        response = self.anthropic.messages.create(
            model="claude-sonnet-4-20250514",
            max_tokens=1000,
            messages=messages,
            tools=available_tools
        )

        final_text = []
        assistant_content = []

        for block in response.content:
            if block.type == "text":
                final_text.append(block.text)
                assistant_content.append(block)
            elif block.type == "tool_use":
                assistant_content.append(block)
                result = await self.call_tool(block.name, block.input)

                messages.append({"role": "assistant", "content": assistant_content})
                messages.append({
                    "role": "user",
                    "content": [{"type": "tool_result", "tool_use_id": block.id, "content": result}]
                })

                follow_up = self.anthropic.messages.create(
                    model="claude-sonnet-4-20250514",
                    max_tokens=1000,
                    messages=messages,
                    tools=available_tools
                )
                final_text.append(follow_up.content[0].text)

        return "\n".join(final_text)

    async def cleanup(self):
        await self.exit_stack.aclose()


async def main():
    client = MCPClient()
    try:
        await client.connect_to_server("python", ["server.py"])
        result = await client.process_query_with_tools("What files are in the project?")
        print(result)
    finally:
        await client.cleanup()

asyncio.run(main())
```

Install: `uv add mcp anthropic`

For HTTP transport, use `mcp.client.streamable_http.streamablehttp_client` instead of `stdio_client`.

### 9.2 TypeScript (official SDK)

```typescript
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

// --- stdio transport ---
async function connectStdio(command: string, args: string[]): Promise<Client> {
  const transport = new StdioClientTransport({ command, args });
  const client = new Client(
    { name: "my-client", version: "1.0.0" },
    {
      capabilities: {
        sampling: {},
        roots: { listChanged: true },
      },
    }
  );
  await client.connect(transport);
  return client;
}

// --- Streamable HTTP transport ---
async function connectHTTP(url: string): Promise<Client> {
  const transport = new StreamableHTTPClientTransport(new URL(url));
  const client = new Client(
    { name: "my-client", version: "1.0.0" },
    { capabilities: { sampling: {} } }
  );
  await client.connect(transport);
  return client;
}

// --- Tool discovery ---
async function discoverTools(client: Client) {
  const { tools } = await client.listTools();
  // Paginate if nextCursor is present
  return tools;
}

// --- Tool invocation ---
async function invokeTool(client: Client, name: string, args: Record<string, unknown>) {
  const result = await client.callTool({ name, arguments: args });
  if (result.isError) {
    // Return error content to LLM for retry
    return { error: true, content: result.content };
  }
  return { error: false, content: result.content };
}

// --- Resource operations ---
async function readFile(client: Client, uri: string) {
  const { contents } = await client.readResource({ uri });
  return contents;
}

// --- Prompt retrieval ---
async function getPrompt(client: Client, name: string, args: Record<string, string>) {
  const { messages } = await client.getPrompt({ name, arguments: args });
  return messages;
}

// --- Cleanup ---
async function disconnect(client: Client) {
  await client.close();
}
```

Install: `npm install @modelcontextprotocol/sdk`

---

## 10. Pagination Pattern

`tools/list`, `resources/list`, `prompts/list`, and `resources/templates/list` all support cursor-based pagination. The pattern is the same for all:

```python
# Python
all_tools = []
cursor = None
while True:
    response = await session.list_tools(cursor=cursor)
    all_tools.extend(response.tools)
    if not response.nextCursor:
        break
    cursor = response.nextCursor
```

```typescript
// TypeScript
const allTools = [];
let cursor: string | undefined;
do {
  const response = await client.listTools({ cursor });
  allTools.push(...response.tools);
  cursor = response.nextCursor;
} while (cursor);
```

---

## 11. Error Handling Reference

### Standard JSON-RPC error codes

| Code | Meaning | When used |
| --------- | ---------------------- | ----------- |
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Not a valid JSON-RPC object |
| -32601 | Method not found | Unknown method or unsupported capability |
| -32602 | Invalid params | Malformed parameters |
| -32603 | Internal error | Server-side failure |

### MCP-specific codes

| Code | Meaning |
| --------- | ---------------------- |
| -32002 | Resource not found |
| -1 | User rejected sampling request |

### Tool execution errors

These arrive as `result.isError = true` in `tools/call` responses. Pass the content back to the LLM — it can self-correct and retry with different arguments.

### Initialization errors

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Unsupported protocol version",
    "data": { "supported": ["2024-11-05"], "requested": "2025-11-25" }
  }
}
```

Handle: check the supported versions in `data`, try again with a compatible version, or disconnect if incompatible.

---

## 12. Security Considerations

### Consent and user control

- Users must explicitly consent to data access and tool invocations.
- The host must show the user which tools and resources are accessible.
- Present confirmation before any tool call that performs a mutation or sends data externally.
- For sampling: show the user the prompt before sending, and the response before returning it to the server.

### Tool safety

- Treat tool `annotations` as untrusted unless the server is explicitly trusted.
- Validate all tool inputs server-side before execution.
- Implement rate limits on tool invocations.
- Log all tool calls for audit purposes.
- Set request timeouts; use cancellation notifications on timeout.

### Resource safety

- Validate all resource URIs server-side. Reject path traversal attempts.
- Enforce access controls before serving resource content.
- Do not expose resources the user has not consented to sharing.

### HTTP transport security

- Always validate the `Origin` header. Return 403 on invalid origins (prevents DNS rebinding).
- Bind local servers to 127.0.0.1, not 0.0.0.0.
- Treat session IDs as secrets. Rotate on re-initialization.
- Use HTTPS in production.
- Include `MCP-Protocol-Version` header on all requests after initialization.
- Use cryptographically random session IDs (UUID v4, JWT, or hash).

### Sampling security

- Clients should implement rate limiting for sampling requests.
- Users must be able to approve, modify, or reject any sampling request.
- The protocol intentionally limits server visibility into the actual prompts sent.
- When tools are used in sampling, enforce a maximum iteration cap to prevent runaway agentic loops.

### Icon handling

- Reject icon URIs with unsafe schemes (`javascript:`, `file:`, `ftp:`, `ws:`).
- Fetch icons without credentials (no cookies, no Authorization headers).
- Verify icon origin matches server origin.
- Sanitize SVG content before rendering (can contain embedded JavaScript).
- Maintain a strict allowlist of accepted MIME types.

---

## 13. Capability Quick Reference

| Capability | Side | Enables |
| --- | --- | --- |
| `tools` | Server | `tools/list`, `tools/call`, `notifications/tools/list_changed` |
| `resources` | Server | `resources/list`, `resources/read`, `resources/templates/list`, optional subscribe/listChanged |
| `prompts` | Server | `prompts/list`, `prompts/get`, `notifications/prompts/list_changed` |
| `logging` | Server | Server can emit log messages to client |
| `completions` | Server | Argument auto-completion via `completion/complete` |
| `sampling` | Client | Server can call `sampling/createMessage` |
| `sampling.tools` | Client | Server can include tools in sampling requests |
| `roots` | Client | Server can call `roots/list`, client sends `notifications/roots/list_changed` |
| `elicitation` | Client | Server can request additional user input |

Only use capabilities that were negotiated in the `initialize` handshake. Calling a method for a capability the other side did not declare results in a `-32601` Method Not Found error.
