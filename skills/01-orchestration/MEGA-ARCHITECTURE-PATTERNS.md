---
name: MEGA-ARCHITECTURE-PATTERNS
description: Consolidated architecture patterns - app archetypes, stack selection, MVC, multi-tenant, integration patterns, dependency injection, program/pipeline patterns.
domain: architecture
triggers: architecture patterns, app archetypes, stack selection, mvc, multi-tenant, integration patterns, dependency injection, pipeline pattern, data state services
---

# MEGA-ARCHITECTURE-PATTERNS

Consolidated software architecture patterns: app archetypes, stack selection, MVC, multi-tenant patterns, integration patterns, DI/IoC, data-state-services, and program/pipeline design.


---

<!-- merged from: architecture-patterns.md -->

﻿---
name: Architecture Patterns
description: # Architecture Patterns
 
 ## Component Resources
---

# Architecture Patterns

## Component Resources (Architecture Patterns)

```typescript
class WorkerApp extends pulumi.ComponentResource {
    constructor(name: string, args: WorkerAppArgs, opts?) {
        super("custom:cloudflare:WorkerApp", name, {}, opts);
        const defaultOpts = {parent: this};

        this.kv = new cloudflare.WorkersKvNamespace(`${name}-kv`, {accountId: args.accountId, title: `${name}-kv`}, defaultOpts);
        this.worker = new cloudflare.WorkerScript(`${name}-worker`, {
            accountId: args.accountId, name: `${name}-worker`, content: args.workerCode,
            module: true, kvNamespaceBindings: [{name: "KV", namespaceId: this.kv.id}],
        }, defaultOpts);
        this.domain = new cloudflare.WorkersDomain(`${name}-domain`, {
            accountId: args.accountId, hostname: args.domain, service: this.worker.name,
        }, defaultOpts);
    }
}
```

## Full-Stack Worker App

```typescript
const kv = new cloudflare.WorkersKvNamespace("cache", {accountId, title: "api-cache"});
const db = new cloudflare.D1Database("db", {accountId, name: "app-database"});
const bucket = new cloudflare.R2Bucket("assets", {accountId, name: "app-assets"});

const apiWorker = new cloudflare.WorkerScript("api", {
    accountId, name: "api-worker", content: fs.readFileSync("./dist/api.js", "utf8"),
    module: true, kvNamespaceBindings: [{name: "CACHE", namespaceId: kv.id}],
    d1DatabaseBindings: [{name: "DB", databaseId: db.id}],
    r2BucketBindings: [{name: "ASSETS", bucketName: bucket.name}],
});
```

## Multi-Environment Setup

```typescript
const stack = pulumi.getStack();
const worker = new cloudflare.WorkerScript(`worker-${stack}`, {
    accountId, name: `my-worker-${stack}`, content: code,
    plainTextBindings: [{name: "ENVIRONMENT", text: stack}],
});
```

## Queue-Based Processing

```typescript
const queue = new cloudflare.Queue("processing-queue", {accountId, name: "image-processing"});

// Producer: API receives requests
const apiWorker = new cloudflare.WorkerScript("api", {
    accountId, name: "api-worker", content: apiCode,
    queueBindings: [{name: "PROCESSING_QUEUE", queue: queue.id}],
});

// Consumer: Process async
const processorWorker = new cloudflare.WorkerScript("processor", {
    accountId, name: "processor-worker", content: processorCode,
    queueConsumers: [{queue: queue.name, maxBatchSize: 10, maxRetries: 3, maxWaitTimeMs: 5000}],
    r2BucketBindings: [{name: "OUTPUT_BUCKET", bucketName: outputBucket.name}],
});
```

## Microservices with Service Bindings

```typescript
const authWorker = new cloudflare.WorkerScript("auth", {accountId, name: "auth-service", content: authCode});
const apiWorker = new cloudflare.WorkerScript("api", {
    accountId, name: "api-service", content: apiCode,
    serviceBindings: [{name: "AUTH", service: authWorker.name}],
});
```

## Event-Driven Architecture

```typescript
const eventQueue = new cloudflare.Queue("events", {accountId, name: "event-bus"});
const producer = new cloudflare.WorkerScript("producer", {
    accountId, name: "api-producer", content: producerCode,
    queueBindings: [{name: "EVENTS", queue: eventQueue.id}],
});
const consumer = new cloudflare.WorkerScript("consumer", {
    accountId, name: "email-consumer", content: consumerCode,
    queueConsumers: [{queue: eventQueue.name, maxBatchSize: 10}],
});
```

## v6.x Versioned Deployments (Blue-Green/Canary)

```typescript
const worker = new cloudflare.Worker("api", {accountId, name: "api-worker"});
const v1 = new cloudflare.WorkerVersion("v1", {accountId, workerId: worker.id, content: fs.readFileSync("./dist/v1.js", "utf8"), compatibilityDate: "2025-01-01"});
const v2 = new cloudflare.WorkerVersion("v2", {accountId, workerId: worker.id, content: fs.readFileSync("./dist/v2.js", "utf8"), compatibilityDate: "2025-01-01"});

// Gradual rollout: 10% v2, 90% v1
const deployment = new cloudflare.WorkersDeployment("canary", {
    accountId, workerId: worker.id,
    versions: [{versionId: v2.id, percentage: 10}, {versionId: v1.id, percentage: 90}],
    kvNamespaceBindings: [{name: "MY_KV", namespaceId: kv.id}],
});
```

**Use:** Canary releases, A/B testing, blue-green. Most apps use `WorkerScript` (auto-versioning).

## Wrangler.toml Generation (Bridge IaC with Local Dev)

Generate wrangler.toml from Pulumi config to keep local dev in sync:

```typescript
import * as command from "@pulumi/command";

const workerConfig = {
    name: "my-worker",
    compatibilityDate: "2025-01-01",
    compatibilityFlags: ["nodejs_compat"],
};

// Create resources
const kv = new cloudflare.WorkersKvNamespace("kv", {accountId, title: "my-kv"});
const db = new cloudflare.D1Database("db", {accountId, name: "my-db"});
const bucket = new cloudflare.R2Bucket("bucket", {accountId, name: "my-bucket"});

// Generate wrangler.toml after resources created
const wranglerGen = new command.local.Command("gen-wrangler", {
    create: pulumi.interpolate`cat > wrangler.toml <<EOF
name = "${workerConfig.name}"
main = "src/index.ts"
compatibility_date = "${workerConfig.compatibilityDate}"
compatibility_flags = ${JSON.stringify(workerConfig.compatibilityFlags)}

[[kv_namespaces]]
binding = "MY_KV"
id = "${kv.id}"

[[d1_databases]]
binding = "DB"
database_id = "${db.id}"
database_name = "${db.name}"

[[r2_buckets]]
binding = "MY_BUCKET"
bucket_name = "${bucket.name}"
EOF`,
}, {dependsOn: [kv, db, bucket]});

// Deploy worker after wrangler.toml generated
const worker = new cloudflare.WorkerScript("worker", {
    accountId, name: workerConfig.name, content: code,
    compatibilityDate: workerConfig.compatibilityDate,
    compatibilityFlags: workerConfig.compatibilityFlags,
    kvNamespaceBindings: [{name: "MY_KV", namespaceId: kv.id}],
    d1DatabaseBindings: [{name: "DB", databaseId: db.id}],
    r2BucketBindings: [{name: "MY_BUCKET", bucketName: bucket.name}],
}, {dependsOn: [wranglerGen]});
```

### Benefits

- `wrangler dev` uses same bindings as production
- No config drift between Pulumi and local dev
- Single source of truth (Pulumi config)

**Alternative:** Read wrangler.toml in Pulumi (reverse direction) if wrangler is source of truth

## Build + Deploy Pattern

```typescript
import * as command from "@pulumi/command";
const build = new command.local.Command("build", {create: "npm run build", dir: "./worker"});
const worker = new cloudflare.WorkerScript("worker", {
    accountId, name: "my-worker",
    content: build.stdout.apply(() => fs.readFileSync("./worker/dist/index.js", "utf8")),
}, {dependsOn: [build]});
```

## Content SHA Pattern (Force Updates)

Prevent false "no changes" detections:

```typescript
const version = Date.now().toString();
const worker = new cloudflare.WorkerScript("worker", {
    accountId, name: "my-worker", content: code,
    plainTextBindings: [{name: "VERSION", text: version}], // Forces deployment
});
```

---
See: [README.md](./README.md), [configuration.md](./configuration.md), [api.md](./api.md), [gotchas.md](./gotchas.md)


---

<!-- merged from: app-archetypes.md -->

﻿---
name: App Archetypes
description: # App Archetypes
 
 Load this reference before choosing a starting point for a new ChatGPT app. The goal is to keep the skill inside a small number of supported app shapes instead of inventing a custom structure for every prompt.
---

# App Archetypes

Load this reference before choosing a starting point for a new ChatGPT app. The goal is to keep the skill inside a small number of supported app shapes instead of inventing a custom structure for every prompt.

## Rule

Choose one primary archetype per request and state it.

Do not combine several archetypes unless the user explicitly asks for a hybrid app and the extra complexity is necessary.

## Archetypes

### `tool-only`

Use when:

- The user does not need an in-ChatGPT UI
- The task is mainly search, fetch, retrieval, or background actions

Default shape:

- MCP server only

Best starting point:

- Official docs and MCP server examples

Validation emphasis:

- `/mcp` route works
- tool schemas and annotations are correct
- no unnecessary UI resource is registered
- if the app is connector-like or sync-oriented, `search` and `fetch` should be the default read-only tools

### `vanilla-widget`

Use when:

- The user wants a small demo, workshop starter, or simple inline widget
- A single HTML widget is enough
- The user wants the fastest path to a working repo

Default shape:

- Root-level server plus `public/` widget assets

Best starting point:

- Apps SDK quickstart first
- Local fallback scaffold if the quickstart is not a good fit

Validation emphasis:

- bridge initialization
- `ui/notifications/tool-result`
- `tools/call` only when the widget is interactive

### `react-widget`

Use when:

- The user wants a polished UI
- The UI is clearly component-based
- The user mentions React, TypeScript frontend tooling, or richer design requirements

Default shape:

- Split `server/` + `web/` layout when the example already uses it

Best starting point:

- Official OpenAI examples

Validation emphasis:

- build output is wired into the server correctly
- bundle references resolve
- widget renders from `structuredContent`

### `interactive-decoupled`

Use when:

- The app has repeated user interaction
- The widget should stay mounted while tools are called repeatedly
- The app is a board, map, editor, game, dashboard, or other stateful experience

Default shape:

- Split `server/` + `web/`
- data tools plus render tools

Best starting point:

- Official OpenAI examples plus `references/interactive-state-sync-patterns.md`

Validation emphasis:

- tool retries are safe
- widget does not remount unnecessarily
- state sync is intentional
- UI tool calls work independently of model reruns

### `submission-ready`

Use when:

- The user asks for public launch, review readiness, or directory submission

Default shape:

- Smallest viable repo that still includes deployment and review requirements

Best starting point:

- Closest official example that matches the requested stack

Validation emphasis:

- `_meta.ui.domain`
- accurate CSP
- auth and review-safe flows
- submission prerequisites and artifacts

## Selection Heuristic

- If the prompt does not mention a UI, choose `tool-only`.
- If the prompt is about a knowledge source, sync app, connector-like integration, or deep research, strongly prefer `tool-only` plus the standard `search` and `fetch` tools unless the user clearly needs a widget.
- If the prompt asks for a simple demo or starter, choose `vanilla-widget`.
- If the prompt asks for a polished UI or React, choose `react-widget`.
- If the prompt implies long-lived client state or repeated interaction, choose `interactive-decoupled`.
- Only choose `submission-ready` when the user explicitly asks for launch or review-readiness work.


---

<!-- merged from: stack-selection.md -->

﻿---
name: Stack Selection
description: # Stack Selection
 
 Primary docs:
---

# Stack Selection

Primary docs:

- <https://learn.microsoft.com/aspnet/core/>
- <https://learn.microsoft.com/aspnet/core/blazor/>
- <https://learn.microsoft.com/aspnet/core/razor-pages/>
- <https://learn.microsoft.com/aspnet/core/mvc/overview>
- <https://learn.microsoft.com/aspnet/core/web-api/>
- <https://learn.microsoft.com/aspnet/core/fundamentals/minimal-apis>

## Default Version Choice

- Prefer the latest stable .NET and ASP.NET Core for new production work.
- As of March 2026, that means `net10.0` unless the repository or user request says otherwise.
- Treat ASP.NET Core 11 as preview. Do not adopt preview APIs by default.
- If the repository already targets `net8.0`, `net9.0`, or another framework, stay within that target unless the task is explicitly an upgrade.

## Template Short Names

The current .NET 10 SDK templates include:

- `dotnet new blazor`
- `dotnet new webapp`
- `dotnet new mvc`
- `dotnet new webapi`
- `dotnet new webapiaot`
- `dotnet new grpc`
- `dotnet new web`
- `dotnet new razorclasslib`

Verify template names with `dotnet new list` if the environment differs.

## Application Model Matrix

| Model | Prefer when | Watch out for | Typical starting point |
| --- | --- | --- | --- |
| Blazor Web App | Build full-stack .NET UI with SSR plus optional interactivity | Interactive server needs a live connection; WebAssembly increases payload size | `dotnet new blazor` |
| Razor Pages | Build page-focused CRUD, forms, dashboards, and line-of-business apps | Authorization cannot be applied per page handler; use MVC if handler-level control matters | `dotnet new webapp` |
| MVC | Build large server-rendered apps with clear controller/view separation, filters, and action-based patterns | More ceremony than Razor Pages for simple page flows | `dotnet new mvc` |
| Minimal APIs | Build focused HTTP APIs, internal services, lightweight backends, and small surface areas | Route handlers can become hard to manage if business logic or metadata grows without structure | `dotnet new webapi` or `dotnet new web` |
| Controller-based Web API | Build APIs that benefit from `[ApiController]`, content negotiation, filters, formatters, and mature controller conventions | More ceremony than Minimal APIs for small endpoints | `dotnet new webapi` |
| SignalR | Add server push, live updates, chat, collaborative UI, or notifications | Requires connection lifecycle management and scale-out planning | Add to an existing ASP.NET Core app |
| gRPC | Build service-to-service or streaming RPC over HTTP/2 | Browser support is different from ordinary JSON APIs; use gRPC-Web only when needed | `dotnet new grpc` |

## Fast Heuristics

- Choose Blazor Web App when the UI itself should be a .NET component model.
- Choose Razor Pages when the app is mostly page and form oriented.
- Choose MVC when actions, views, filters, and controller conventions are the center of the design.
- Choose Minimal APIs first for small to medium HTTP services.
- Switch to controllers when the API needs richer attribute-driven behavior, custom formatters, or strong alignment with existing MVC/Web API conventions.
- Keep the current app model in an existing codebase unless the mismatch is causing real complexity.

## Mixed-Model Guidance

ASP.NET Core can mix models in one host. Common combinations:

- Razor Pages or MVC for server-rendered UI plus Minimal APIs for AJAX or mobile endpoints
- Blazor Web App plus Minimal APIs for external integration endpoints
- MVC or Razor Pages plus SignalR for live updates
- Web API plus gRPC for internal service-to-service calls

Mix models only when it simplifies the public surface. Do not add a second app model just because ASP.NET Core allows it.


---

<!-- merged from: common-patterns.md -->

﻿---
name: Common Patterns
description: # Common Patterns
 
 Real-world patterns and examples for TCP Sockets in Cloudflare Workers.
---

# Common Patterns

Real-world patterns and examples for TCP Sockets in Cloudflare Workers.

```typescript
import { connect } from 'cloudflare:sockets';
```

## Basic Patterns

### Simple Request-Response

```typescript
const socket = connect({ hostname: "echo.example.com", port: 7 }, { secureTransport: "on" });
try {
  await socket.opened;
  const writer = socket.writable.getWriter();
  await writer.write(new TextEncoder().encode("Hello\n"));
  await writer.close();
  
  const reader = socket.readable.getReader();
  const { value } = await reader.read();
  return new Response(value);
} finally {
  await socket.close();
}
```

### Reading All Data

```typescript
async function readAll(socket: Socket): Promise<Uint8Array> {
  const reader = socket.readable.getReader();
  const chunks: Uint8Array[] = [];
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
  }
  const total = chunks.reduce((sum, c) => sum + c.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) { result.set(chunk, offset); offset += chunk.length; }
  return result;
}
```

### Streaming Response

```typescript
// Stream socket data directly to HTTP response
const socket = connect({ hostname: "stream.internal", port: 9000 }, { secureTransport: "on" });
const writer = socket.writable.getWriter();
await writer.write(new TextEncoder().encode("STREAM\n"));
await writer.close();
return new Response(socket.readable);
```

## Protocol Examples

### Redis RESP

```typescript
// Send: *2\r\n$3\r\nGET\r\n$<keylen>\r\n<key>\r\n
// Recv: $<len>\r\n<data>\r\n or $-1\r\n for null
const socket = connect({ hostname: "redis.internal", port: 6379 });
const writer = socket.writable.getWriter();
await writer.write(new TextEncoder().encode(`*2\r\n$3\r\nGET\r\n$3\r\nkey\r\n`));
```

### PostgreSQL

**Use [Hyperdrive](../hyperdrive/) for production.** Raw Postgres protocol is complex (startup, auth, query messages).

### MQTT

```typescript
const socket = connect({ hostname: "mqtt.broker", port: 1883 });
const writer = socket.writable.getWriter();
// CONNECT: 0x10 <len> 0x00 0x04 "MQTT" 0x04 <flags> ...
// PUBLISH: 0x30 <len> <topic_len> <topic> <message>
```

## Error Handling Patterns

### Retry with Backoff

```typescript
async function connectWithRetry(addr: SocketAddress, opts: SocketOptions, maxRetries = 3): Promise<Socket> {
  for (let i = 1; i <= maxRetries; i++) {
    try {
      const socket = connect(addr, opts);
      await socket.opened;
      return socket;
    } catch (error) {
      if (i === maxRetries) throw error;
      await new Promise(r => setTimeout(r, 1000 * Math.pow(2, i - 1))); // Exponential backoff
    }
  }
  throw new Error('Unreachable');
}
```

### Timeout

```typescript
async function connectWithTimeout(addr: SocketAddress, opts: SocketOptions, ms = 5000): Promise<Socket> {
  const socket = connect(addr, opts);
  const timeout = new Promise<never>((_, reject) => setTimeout(() => reject(new Error('Timeout')), ms));
  await Promise.race([socket.opened, timeout]);
  return socket;
}
```

### Fallback

```typescript
async function connectWithFallback(primary: string, fallback: string, port: number): Promise<Socket> {
  try {
    const socket = connect({ hostname: primary, port }, { secureTransport: "on" });
    await socket.opened;
    return socket;
  } catch {
    return connect({ hostname: fallback, port }, { secureTransport: "on" });
  }
}
```

## Security Patterns

### Destination Allowlist (Prevent SSRF)

```typescript
const ALLOWED_HOSTS = ['db.internal.company.net', 'api.internal.company.net', /^10\.0\.1\.\d+$/];

function isAllowed(hostname: string): boolean {
  return ALLOWED_HOSTS.some(p => p instanceof RegExp ? p.test(hostname) : p === hostname);
}

export default {
  async fetch(req: Request): Promise<Response> {
    const target = new URL(req.url).searchParams.get('host');
    if (!target || !isAllowed(target)) return new Response('Forbidden', { status: 403 });
    const socket = connect({ hostname: target, port: 443 });
    // Use socket...
  }
};
```

### Connection Pooling

```typescript
class SocketPool {
  private pool = new Map<string, Socket[]>();
  
  async acquire(hostname: string, port: number): Promise<Socket> {
    const key = `${hostname}:${port}`;
    const sockets = this.pool.get(key) || [];
    if (sockets.length > 0) return sockets.pop()!;
    const socket = connect({ hostname, port }, { secureTransport: "on" });
    await socket.opened;
    return socket;
  }
  
  release(hostname: string, port: number, socket: Socket): void {
    const key = `${hostname}:${port}`;
    const sockets = this.pool.get(key) || [];
    if (sockets.length < 3) { sockets.push(socket); this.pool.set(key, sockets); }
    else socket.close();
  }
}
```

## Multi-Protocol Gateway

```typescript
interface Protocol { name: string; defaultPort: number; test(host: string, port: number): Promise<string>; }

const PROTOCOLS: Record<string, Protocol> = {
  redis: {
    name: 'redis',
    defaultPort: 6379,
    async test(host, port) {
      const socket = connect({ hostname: host, port });
      try {
        const writer = socket.writable.getWriter();
        await writer.write(new TextEncoder().encode('*1\r\n$4\r\nPING\r\n'));
        writer.releaseLock();
        const reader = socket.readable.getReader();
        const { value } = await reader.read();
        return new TextDecoder().decode(value || new Uint8Array());
      } finally { await socket.close(); }
    }
  }
};

export default {
  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const proto = url.pathname.slice(1);  // /redis
    const host = url.searchParams.get('host');
    if (!host || !PROTOCOLS[proto]) return new Response('Invalid', { status: 400 });
    const result = await PROTOCOLS[proto].test(host, parseInt(url.searchParams.get('port') || '') || PROTOCOLS[proto].defaultPort);
    return new Response(result);
  }
};
```


---

<!-- merged from: patterns.md -->

﻿---
name: Patterns
description: # Patterns
 
 ## Secret Rotation
---

# Patterns

## Secret Rotation (Patterns)

Zero-downtime rotation with versioned naming (`api_key_v1`, `api_key_v2`):

```typescript
interface Env {
  PRIMARY_KEY: { get(): Promise<string> };
  FALLBACK_KEY?: { get(): Promise<string> };
}

async function fetchWithAuth(url: string, key: string) {
  return fetch(url, { headers: { "Authorization": `Bearer ${key}` } });
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    let resp = await fetchWithAuth("https://api.example.com", await env.PRIMARY_KEY.get());
    
    // Fallback during rotation
    if (!resp.ok && env.FALLBACK_KEY) {
      resp = await fetchWithAuth("https://api.example.com", await env.FALLBACK_KEY.get());
    }
    
    return resp;
  }
}
```

Workflow: Create `api_key_v2` → add fallback binding → deploy → swap primary → deploy → remove `v1`

## Encryption with KV

```typescript
interface Env {
  CACHE: KVNamespace;
  ENCRYPTION_KEY: { get(): Promise<string> };
}

async function encryptValue(value: string, key: string): Promise<string> {
  const enc = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey(
    "raw", enc.encode(key), { name: "AES-GCM" }, false, ["encrypt"]
  );
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const encrypted = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv }, keyMaterial, enc.encode(value)
  );
  
  const combined = new Uint8Array(iv.length + encrypted.byteLength);
  combined.set(iv);
  combined.set(new Uint8Array(encrypted), iv.length);
  return btoa(String.fromCharCode(...combined));
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const key = await env.ENCRYPTION_KEY.get();
    const encrypted = await encryptValue("sensitive-data", key);
    await env.CACHE.put("user:123:data", encrypted);
    return Response.json({ ok: true });
  }
}
```

## HMAC Signing

```typescript
interface Env {
  HMAC_SECRET: { get(): Promise<string> };
}

async function signRequest(data: string, secret: string): Promise<string> {
  const enc = new TextEncoder();
  const key = await crypto.subtle.importKey(
    "raw", enc.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"]
  );
  const sig = await crypto.subtle.sign("HMAC", key, enc.encode(data));
  return btoa(String.fromCharCode(...new Uint8Array(sig)));
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const secret = await env.HMAC_SECRET.get();
    const payload = await request.text();
    const signature = await signRequest(payload, secret);
    return Response.json({ signature });
  }
}
```

## Audit & Monitoring

```typescript
export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext) {
    const startTime = Date.now();
    try {
      const apiKey = await env.API_KEY.get();
      const resp = await fetch("https://api.example.com", {
        headers: { "Authorization": `Bearer ${apiKey}` }
      });
      
      ctx.waitUntil(
        fetch("https://log.example.com/log", {
          method: "POST",
          body: JSON.stringify({
            event: "secret_used",
            secret_name: "API_KEY",
            timestamp: new Date().toISOString(),
            duration_ms: Date.now() - startTime,
            success: resp.ok
          })
        })
      );
      return resp;
    } catch (error) {
      ctx.waitUntil(
        fetch("https://log.example.com/log", {
          method: "POST",
          body: JSON.stringify({
            event: "secret_access_failed",
            secret_name: "API_KEY",
            error: error instanceof Error ? error.message : "Unknown"
          })
        })
      );
      return new Response("Error", { status: 500 });
    }
  }
}
```

## Migration from Worker Secrets

Change `env.SECRET` (direct) to `await env.SECRET.get()` (async).

Steps:

1. Create in Secrets Store: `wrangler secrets-store secret create <store-id> --name API_KEY --scopes workers --remote`
2. Add binding to `wrangler.jsonc`: `{"binding": "API_KEY", "store_id": "abc123", "secret_name": "api_key"}`
3. Update code: `const key = await env.API_KEY.get();`
4. Test staging, deploy
5. Remove old: `wrangler secret delete API_KEY`

## Sharing Across Workers

Same secret, different binding names:

```jsonc
// worker-1: binding="SHARED_DB", secret_name="postgres_url"
// worker-2: binding="DB_CONN", secret_name="postgres_url"
```

## JSON Secret Parsing

Store structured config as JSON secrets:

```typescript
interface Env {
  DB_CONFIG: { get(): Promise<string> };
}

interface DbConfig {
  host: string;
  port: number;
  username: string;
  password: string;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    try {
      const configStr = await env.DB_CONFIG.get();
      const config: DbConfig = JSON.parse(configStr);
      
      // Use parsed config
      const dbUrl = `postgres://${config.username}:${config.password}@${config.host}:${config.port}`;
      
      return Response.json({ connected: true });
    } catch (error) {
      if (error instanceof SyntaxError) {
        return new Response("Invalid config JSON", { status: 500 });
      }
      throw error;
    }
  }
}
```

Store JSON secret:

```bash
echo '{"host":"db.example.com","port":5432,"username":"app","password":"secret"}' | \
  wrangler secrets-store secret create <store-id> \
    --name DB_CONFIG --scopes workers --remote
```

## Integration

### Service Bindings

Auth Worker signs JWT with Secrets Store; API Worker verifies via service binding.

See: [workers](../workers/) for service binding patterns.

See: [api.md](./api.md), [gotchas.md](./gotchas.md)


---

<!-- merged from: patterns-use-cases.md -->

﻿---
name: Patterns & Use Cases
description: # Patterns & Use Cases
 
 ## Architecture
---

# Patterns & Use Cases

## Architecture (Patterns & Use Cases)

```text
Client (WebRTC) <---> CF Edge <---> Backend (HTTP)
                           |
                    CF Backbone (310+ DCs)
                           |
                    Other Edges <---> Other Clients
```

Anycast: Last-mile <50ms (95%), no region select, NACK shield, distributed consensus

Cascading trees auto-scale to millions:

```text
Publisher -> Edge A -> Edge B -> Sub1
                    \-> Edge C -> Sub2,3
```

## Use Cases

**1:1:** A creates session+publishes, B creates+subscribes to A+publishes, A subscribes to B
**N:N:** All create session+publish, backend broadcasts track IDs, all subscribe to others
**1:N:** Publisher creates+publishes, viewers each create+subscribe (no fan-out limit)
**Breakout:** Same PeerConnection! Backend closes/adds tracks, no recreation

## PartyTracks (Recommended)

Observable-based client with automatic device/network handling:

```typescript
import {PartyTracks} from 'partytracks';

// Create client
const pt = new PartyTracks({
  apiUrl: '/api/calls',
  sessionId: 'my-session',
  onTrack: (track, peer) => {
    const video = document.getElementById(`video-${peer.id}`) as HTMLVideoElement;
    video.srcObject = new MediaStream([track]);
  }
});

// Publish camera (push API)
const camera = await pt.getCamera(); // Auto-requests permissions, handles device changes
await pt.publishTrack(camera, {trackName: 'my-camera'});

// Subscribe to remote track (pull API)
await pt.subscribeToTrack({trackName: 'remote-camera', sessionId: 'other-session'});

// React hook example
import {useObservableAsValue} from 'observable-hooks';

function VideoCall() {
  const localTracks = useObservableAsValue(pt.localTracks$);
  const remoteTracks = useObservableAsValue(pt.remoteTracks$);
  
  return <div>{/* Render tracks */}</div>;
}

// Screenshare
const screen = await pt.getScreenshare();
await pt.publishTrack(screen, {trackName: 'my-screen'});

// Handle device changes (automatic)
// PartyTracks detects device changes (e.g., Bluetooth headset) and renegotiates
```

## Backend

Express:

```js
app.post('/api/new-session', async (req, res) => {
  const r = await fetch(`${CALLS_API}/apps/${process.env.CALLS_APP_ID}/sessions/new`,
    {method: 'POST', headers: {'Authorization': `Bearer ${process.env.CALLS_APP_SECRET}`}});
  res.json(await r.json());
});
```

Workers: Same pattern, use `env.CALLS_APP_ID` and `env.CALLS_APP_SECRET`

DO Presence: See configuration.md for boilerplate

## Audio Level Detection

```typescript
// Attach analyzer to audio track
function attachAudioLevelDetector(track: MediaStreamTrack) {
  const ctx = new AudioContext();
  const analyzer = ctx.createAnalyser();
  const src = ctx.createMediaStreamSource(new MediaStream([track]));
  src.connect(analyzer);
  
  const data = new Uint8Array(analyzer.frequencyBinCount);
  const checkLevel = () => {
    analyzer.getByteFrequencyData(data);
    const level = data.reduce((a, b) => a + b) / data.length;
    if (level > 30) console.log('Speaking:', level); // Trigger UI update
    requestAnimationFrame(checkLevel);
  };
  checkLevel();
}
```

## Connection Quality Monitoring

```typescript
pc.getStats().then(stats => {
  stats.forEach(report => {
    if (report.type === 'inbound-rtp' && report.kind === 'video') {
      const {packetsLost, packetsReceived, jitter} = report;
      const lossRate = packetsLost / (packetsLost + packetsReceived);
      if (lossRate > 0.05) console.warn('High packet loss:', lossRate);
      if (jitter > 100) console.warn('High jitter:', jitter);
    }
  });
});
```

## Stage Management (Limit Visible Participants)

```typescript
// Subscribe to top 6 active speakers only
let activeSubscriptions = new Set<string>();

function updateStage(topSpeakers: string[]) {
  const toAdd = topSpeakers.filter(id => !activeSubscriptions.has(id)).slice(0, 6);
  const toRemove = [...activeSubscriptions].filter(id => !topSpeakers.includes(id));
  
  toRemove.forEach(id => {
    pc.getSenders().find(s => s.track?.id === id)?.track?.stop();
    activeSubscriptions.delete(id);
  });
  
  toAdd.forEach(async id => {
    await fetch(`/api/subscribe`, {method: 'POST', body: JSON.stringify({trackId: id})});
    activeSubscriptions.add(id);
  });
}
```

## Advanced

Bandwidth mgmt:

```ts
const s = pc.getSenders().find(s => s.track?.kind === 'video');
const p = s.getParameters();
if (!p.encodings) p.encodings = [{}];
p.encodings[0].maxBitrate = 1200000; p.encodings[0].maxFramerate = 24;
await s.setParameters(p);
```

Simulcast (CF auto-forwards best layer):

```ts
pc.addTransceiver('video', {direction: 'sendonly', sendEncodings: [
  {rid: 'high', maxBitrate: 1200000},
  {rid: 'med', maxBitrate: 600000, scaleResolutionDownBy: 2},
  {rid: 'low', maxBitrate: 200000, scaleResolutionDownBy: 4}
]});
```

DataChannel:

```ts
const dc = pc.createDataChannel('chat', {ordered: true, maxRetransmits: 3});
dc.onopen = () => dc.send(JSON.stringify({type: 'chat', text: 'Hi'}));
dc.onmessage = (e) => console.log('RX:', JSON.parse(e.data));
```

**WHIP/WHEP:** For streaming interop (OBS → SFU, SFU → video players), use WHIP (ingest) and WHEP (egress) protocols. See Cloudflare Stream integration docs.

Integrations: R2 for recording `env.R2_BUCKET.put(...)`, Queues for analytics

Perf: 100-250ms connect, ~50ms latency (95%), 200-400ms glass-to-glass, no participant limit (client: 10-50 tracks)


---

<!-- merged from: multi-tenant-patterns.md -->

﻿---
name: Multi-Tenant Patterns
description: # Multi-Tenant Patterns
 
 ## Billing by Plan
---

# Multi-Tenant Patterns

## Billing by Plan (Multi-Tenant Patterns)

```typescript
interface Env {
  DISPATCHER: DispatchNamespace;
  CUSTOMERS_KV: KVNamespace;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const userWorkerName = new URL(request.url).hostname.split(".")[0];
    const customerPlan = await env.CUSTOMERS_KV.get(userWorkerName);
    
    const plans = {
      enterprise: { cpuMs: 50, subRequests: 50 },
      pro: { cpuMs: 20, subRequests: 20 },
      free: { cpuMs: 10, subRequests: 5 },
    };
    const limits = plans[customerPlan as keyof typeof plans] || plans.free;
    
    const userWorker = env.DISPATCHER.get(userWorkerName, {}, { limits });
    return await userWorker.fetch(request);
  },
};
```

## Resource Isolation

**Complete isolation:** Create unique resources per customer

- KV namespace per customer
- D1 database per customer
- R2 bucket per customer

```typescript
const bindings = [{
  type: "kv_namespace",
  name: "USER_KV",
  namespace_id: `customer-${customerId}-kv`
}];
```

## Hostname Routing

### Wildcard Route (Recommended)

Configure `*/*` route on SaaS domain → dispatch Worker

#### Benefits

- Supports subdomains + custom vanity domains
- No per-route limits (regular Workers limited to 100 routes)
- Programmatic control
- Works with any DNS proxy settings

#### Setup

1. Cloudflare for SaaS custom hostnames
2. Fallback origin (dummy `A 192.0.2.0` if Worker is origin)
3. DNS CNAME to SaaS domain
4. `*/*` route → dispatch Worker
5. Routing logic in dispatch Worker

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const hostname = new URL(request.url).hostname;
    const hostnameData = await env.ROUTING_KV.get(`hostname:${hostname}`, { type: "json" });
    
    if (!hostnameData?.workerName) {
      return new Response("Hostname not configured", { status: 404 });
    }
    
    const userWorker = env.DISPATCHER.get(hostnameData.workerName);
    return await userWorker.fetch(request);
  },
};
```

### Subdomain-Only

1. Wildcard DNS: `*.saas.com` → origin
2. Route: `*.saas.com/*` → dispatch Worker
3. Extract subdomain for routing

### Orange-to-Orange (O2O) Behavior

When customers use Cloudflare and CNAME to your Workers domain:

| Scenario | Behavior | Route Pattern |
| ---------- | ---------- | --------------- |
| Customer not on Cloudflare | Standard routing | `*/*` or `*.domain.com/*` |
| Customer on Cloudflare (proxied CNAME) | Invokes Worker at edge | `*/*` required |
| Customer on Cloudflare (DNS-only CNAME) | Standard routing | Any route works |

**Recommendation:** Always use `*/*` wildcard for consistent O2O behavior.

### Custom Metadata Routing

For Cloudflare for SaaS: Store worker name in custom hostname `custom_metadata`, retrieve in dispatch worker to route requests. Requires custom hostnames as subdomains of your domain.

## Observability

### Logpush

- Enable on dispatch Worker → captures all user Worker logs
- Filter by `Outcome` or `Script Name`

### Tail Workers

- Real-time logs with custom formatting
- Receives HTTP status, `console.log()`, exceptions, diagnostics

### Analytics Engine

```typescript
// Track violations
env.ANALYTICS.writeDataPoint({
  indexes: [customerName],
  blobs: ["cpu_limit_exceeded"],
});
```

### GraphQL

```graphql
query {
  viewer {
    accounts(filter: {accountTag: $accountId}) {
      workersInvocationsAdaptive(filter: {dispatchNamespaceName: "production"}) {
        sum { requests errors cpuTime }
      }
    }
  }
}
```

## Use Case Implementations

### AI Code Execution

```typescript
async function deployGeneratedCode(name: string, code: string) {
  const file = new File([code], `${name}.mjs`, { type: "application/javascript+module" });
  await client.workersForPlatforms.dispatch.namespaces.scripts.update("production", name, {
    account_id: accountId,
    metadata: { main_module: `${name}.mjs`, tags: [name, "ai-generated"] },
    files: [file],
  });
}

// Short limits for untrusted code
const userWorker = env.DISPATCHER.get(sessionId, {}, { limits: { cpuMs: 5, subRequests: 3 } });
```

**VibeSDK:** For AI-powered code generation + deployment platforms, see [VibeSDK](https://github.com/cloudflare/vibesdk) - handles AI generation, sandbox execution, live preview, and deployment.

Reference: [AI Vibe Coding Platform Architecture](https://developers.cloudflare.com/reference-architecture/diagrams/ai/ai-vibe-coding-platform/)

### Edge Functions Platform

```typescript
// Route: /customer-id/function-name
const [customerId, functionName] = new URL(request.url).pathname.split("/").filter(Boolean);
const workerName = `${customerId}-${functionName}`;
const userWorker = env.DISPATCHER.get(workerName);
```

### Website Builder

- Deploy static assets + Worker code
- See [api.md](./api.md#static-assets) for full implementation
- Salt hashes for asset isolation

## Best Practices

### Architecture

- One namespace per environment (production, staging)
- Platform logic in dispatch Worker (auth, rate limiting, validation)
- Isolation automatic (no shared cache, untrusted mode)

### Routing

- Use `*/*` wildcard routes
- Store mappings in KV
- Handle missing Workers gracefully

### Limits & Security

- Set custom limits by plan
- Track violations with Analytics Engine
- Use outbound Workers for egress control
- Sanitize responses

### Tags

- Tag all Workers: customer ID, plan, environment
- Enable bulk operations
- Filter efficiently

See [README.md](./README.md), [configuration.md](./configuration.md), [api.md](./api.md), [gotchas.md](./gotchas.md)


---

<!-- merged from: mvc.md -->

﻿---
name: MVC
description: # MVC
 
 Primary docs:
---

# MVC

Primary docs:

- <https://learn.microsoft.com/aspnet/core/mvc/overview>
- <https://learn.microsoft.com/aspnet/core/mvc/controllers/>
- <https://learn.microsoft.com/aspnet/core/mvc/views/>

## Choose MVC When Actions And Views Matter

Prefer MVC when the application benefits from explicit controllers, action-based routing, filters, view models, and a strong separation between orchestration and presentation.

This is often the right fit for:

- large server-rendered sites
- applications with many cross-cutting filters or action conventions
- applications that mix views and APIs in the same controller layer
- teams already organized around controllers and views

## Core Shape

Enable MVC with views using:

- `builder.Services.AddControllersWithViews();`
- `app.MapControllerRoute(...)`

Keep views focused on presentation. Keep controllers focused on HTTP orchestration. Put business rules in services.

## Controller Guidance

- Derive from `Controller` when the controller returns views
- Keep actions small and explicit
- Use model binding and validation instead of manual request parsing
- Return view models, not EF entities, to views
- Use POST-Redirect-GET for form submissions

## View Guidance

- Use layouts, partial views, and Tag Helpers to keep markup consistent
- Keep complex display logic out of Razor markup when it becomes hard to follow
- Use strongly typed view models
- Avoid coupling views directly to persistence models

## Structure And Scale

- Use areas for large bounded sections such as Admin or BackOffice
- Keep route conventions explicit
- Apply filters when behavior truly belongs at the MVC layer
- Avoid giant god controllers; split by cohesive feature or resource

## Choosing MVC Over Razor Pages

Prefer MVC over Razor Pages when:

- multiple related actions share controller-level behavior
- handler-level authorization or action filters matter
- URL and action design are more natural than page-file routing


---

<!-- merged from: integration-patterns.md -->

﻿---
name: Integration Patterns
description: # Integration Patterns
 
 ## Enable Argo + Tiered Cache
---

# Integration Patterns

## Enable Argo + Tiered Cache (Integration Patterns)

```typescript
async function enableOptimalPerformance(client: Cloudflare, zoneId: string) {
  await Promise.all([
    client.argo.smartRouting.edit({ zone_id: zoneId, value: 'on' }),
    client.argo.tieredCaching.edit({ zone_id: zoneId, value: 'on' }),
  ]);
}
```

**Flow:** Visitor → Edge (Lower-Tier) → [Cache Miss] → Upper-Tier → [Cache Miss + Argo] → Origin

**Impact:** Argo ~30% latency reduction + Tiered Cache 50-80% origin offload

## Usage Analytics (GraphQL)

```graphql
query ArgoAnalytics($zoneTag: string!) {
  viewer {
    zones(filter: { zoneTag: $zoneTag }) {
      httpRequestsAdaptiveGroups(limit: 1000) {
        sum { argoBytes, bytes }
      }
    }
  }
}
```

**Billing:** ~$0.10/GB. DDoS-mitigated and WAF-blocked traffic NOT charged.

## Spectrum TCP Integration

Enable Argo for non-HTTP traffic (databases, game servers, IoT):

```typescript
// Update existing app
await client.spectrum.apps.update(appId, { zone_id: zoneId, argo_smart_routing: true });

// Create new app with Argo
await client.spectrum.apps.create({
  zone_id: zoneId,
  dns: { type: 'CNAME', name: 'tcp.example.com' },
  origin_direct: ['tcp://origin.example.com:3306'],
  protocol: 'tcp/3306',
  argo_smart_routing: true,
});
```

**Use cases:** MySQL/PostgreSQL (3306/5432), game servers, MQTT (1883), SSH (22)

## Pre-Flight Validation

```typescript
async function validateArgoEligibility(client: Cloudflare, zoneId: string) {
  const status = await client.argo.smartRouting.get({ zone_id: zoneId });
  const zone = await client.zones.get({ zone_id: zoneId });
  
  const issues: string[] = [];
  if (!status.editable) issues.push('Zone not editable');
  if (['free', 'pro'].includes(zone.plan.legacy_id)) issues.push('Requires Business+ plan');
  if (zone.status !== 'active') issues.push('Zone not active');
  
  return { canEnable: issues.length === 0, issues };
}
```

## Post-Enable Verification

```typescript
async function verifyArgoEnabled(client: Cloudflare, zoneId: string): Promise<boolean> {
  await new Promise(r => setTimeout(r, 2000)); // Wait for propagation
  const status = await client.argo.smartRouting.get({ zone_id: zoneId });
  return status.value === 'on';
}
```

## Full Setup Pattern

```typescript
async function setupArgo(client: Cloudflare, zoneId: string) {
  // 1. Validate
  const { canEnable, issues } = await validateArgoEligibility(client, zoneId);
  if (!canEnable) throw new Error(issues.join(', '));
  
  // 2. Enable both features
  await Promise.all([
    client.argo.smartRouting.edit({ zone_id: zoneId, value: 'on' }),
    client.argo.tieredCaching.edit({ zone_id: zoneId, value: 'on' }),
  ]);
  
  // 3. Verify
  const [argo, cache] = await Promise.all([
    client.argo.smartRouting.get({ zone_id: zoneId }),
    client.argo.tieredCaching.get({ zone_id: zoneId }),
  ]);
  
  return { argo: argo.value === 'on', tieredCache: cache.value === 'on' };
}
```

**When to combine:** High-traffic sites (>1TB/mo), global users, cacheable content.


---

<!-- merged from: data-state-and-services.md -->

﻿---
name: Data, State, And Services
description: # Data, State, And Services
 
 Primary docs:
---

# Data, State, And Services

Primary docs:

- <https://learn.microsoft.com/aspnet/core/data/>
- <https://learn.microsoft.com/aspnet/core/fundamentals/dependency-injection>
- <https://learn.microsoft.com/aspnet/core/fundamentals/http-requests>
- <https://learn.microsoft.com/aspnet/core/fundamentals/app-state>

## Dependency Injection Defaults

- Register infrastructure and business services in `Program.cs`
- Inject dependencies through constructors by default
- Keep scoped services request-bound
- Avoid resolving scoped services from singletons
- Use keyed or named patterns only when there is a real need for multiple implementations

## EF Core And DbContext

Use EF Core for common relational data access patterns unless the repository already uses another data layer.

Default guidance:

- register `DbContext` with `AddDbContext`
- treat `DbContext` as scoped
- keep queries and transactions in services, not UI code
- use migrations intentionally
- keep entities out of public API contracts and UI view models

Use `IDbContextFactory<TContext>` when the execution model is not request-scoped, such as:

- Blazor components with longer-lived scopes
- background services
- explicit factory-driven data work

## Options And Configuration

- Bind structured configuration into options classes
- validate options early when bad configuration should fail fast
- keep configuration access close to the service that owns it
- avoid scattering raw configuration keys across the codebase

## Outbound HTTP

Use `IHttpClientFactory` for outbound HTTP calls.

Prefer:

- named clients for distinct external systems
- typed clients for richer integrations
- delegating handlers for retries, headers, or telemetry concerns

Avoid manual `new HttpClient()` patterns scattered through request handlers.

## App State

Use the smallest state mechanism that fits:

- query string or route values for transparent request state
- form posts for user input
- TempData for short-lived redirect-friendly messages
- session only when necessary and with an understanding of its server-side and scaling implications

Do not treat session as the primary application data store.

## Caching And State Boundaries

- Keep cached data derivable from a durable source
- Separate cache shape from persistence shape when it improves safety or performance
- Revisit session, in-memory cache, and singleton state when the app scales to multiple instances


---

<!-- merged from: define-generic-context-interfaces-for-dependency-injection.md -->

﻿---
name: Define Generic Context Interfaces for Dependency Injection
description: ## Define Generic Context Interfaces for Dependency Injection
 
 Define a **generic interface** for your component context with three parts:
tags: composition, context, state, typescript, dependency-injection
---

## Define Generic Context Interfaces for Dependency Injection

Define a **generic interface** for your component context with three parts:
`state`, `actions`, and `meta`. This interface is a contract that any provider
can implement—enabling the same UI components to work with completely different
state implementations.

**Core principle:** Lift state, compose internals, make state
dependency-injectable.

### Incorrect (UI coupled to specific state implementation)

```tsx
function ComposerInput() {
  // Tightly coupled to a specific hook
  const { input, setInput } = useChannelComposerState()
  return <TextInput value={input} onChangeText={setInput} />
}
```

#### Correct (generic interface enables dependency injection)

```tsx
// Define a GENERIC interface that any provider can implement
interface ComposerState {
  input: string
  attachments: Attachment[]
  isSubmitting: boolean
}

interface ComposerActions {
  update: (updater: (state: ComposerState) => ComposerState) => void
  submit: () => void
}

interface ComposerMeta {
  inputRef: React.RefObject<TextInput>
}

interface ComposerContextValue {
  state: ComposerState
  actions: ComposerActions
  meta: ComposerMeta
}

const ComposerContext = createContext<ComposerContextValue | null>(null)
```

#### UI components consume the interface, not the implementation

```tsx
function ComposerInput() {
  const {
    state,
    actions: { update },
    meta,
  } = use(ComposerContext)

  // This component works with ANY provider that implements the interface
  return (
    <TextInput
      ref={meta.inputRef}
      value={state.input}
      onChangeText={(text) => update((s) => ({ ...s, input: text }))}
    />
  )
}
```

#### Different providers implement the same interface

```tsx
// Provider A: Local state for ephemeral forms
function ForwardMessageProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState(initialState)
  const inputRef = useRef(null)
  const submit = useForwardMessage()

  return (
    <ComposerContext
      value={{
        state,
        actions: { update: setState, submit },
        meta: { inputRef },
      }}
    >
      {children}
    </ComposerContext>
  )
}

// Provider B: Global synced state for channels
function ChannelProvider({ channelId, children }: Props) {
  const { state, update, submit } = useGlobalChannel(channelId)
  const inputRef = useRef(null)

  return (
    <ComposerContext
      value={{
        state,
        actions: { update, submit },
        meta: { inputRef },
      }}
    >
      {children}
    </ComposerContext>
  )
}
```

#### The same composed UI works with both

```tsx
// Works with ForwardMessageProvider (local state)
<ForwardMessageProvider>
  <Composer.Frame>
    <Composer.Input />
    <Composer.Submit />
  </Composer.Frame>
</ForwardMessageProvider>

// Works with ChannelProvider (global synced state)
<ChannelProvider channelId="abc">
  <Composer.Frame>
    <Composer.Input />
    <Composer.Submit />
  </Composer.Frame>
</ChannelProvider>
```

#### Custom UI outside the component can access state and actions

The provider boundary is what matters—not the visual nesting. Components that
need shared state don't have to be inside the `Composer.Frame`. They just need
to be within the provider.

```tsx
function ForwardMessageDialog() {
  return (
    <ForwardMessageProvider>
      <Dialog>
        {/* The composer UI */}
        <Composer.Frame>
          <Composer.Input placeholder="Add a message, if you'd like." />
          <Composer.Footer>
            <Composer.Formatting />
            <Composer.Emojis />
          </Composer.Footer>
        </Composer.Frame>

        {/* Custom UI OUTSIDE the composer, but INSIDE the provider */}
        <MessagePreview />

        {/* Actions at the bottom of the dialog */}
        <DialogActions>
          <CancelButton />
          <ForwardButton />
        </DialogActions>
      </Dialog>
    </ForwardMessageProvider>
  )
}

// This button lives OUTSIDE Composer.Frame but can still submit based on its context!
function ForwardButton() {
  const {
    actions: { submit },
  } = use(ComposerContext)
  return <Button onPress={submit}>Forward</Button>
}

// This preview lives OUTSIDE Composer.Frame but can read composer's state!
function MessagePreview() {
  const { state } = use(ComposerContext)
  return <Preview message={state.input} attachments={state.attachments} />
}
```

The `ForwardButton` and `MessagePreview` are not visually inside the composer
box, but they can still access its state and actions. This is the power of
lifting state into providers.

The UI is reusable bits you compose together. The state is dependency-injected
by the provider. Swap the provider, keep the UI.


---

<!-- merged from: program-and-pipeline.md -->

﻿---
name: Program And Pipeline
description: # Program And Pipeline
 
 Primary docs:
---

# Program And Pipeline

Primary docs:

- <https://learn.microsoft.com/aspnet/core/fundamentals/>
- <https://learn.microsoft.com/aspnet/core/fundamentals/minimal-apis/webapplication>
- <https://learn.microsoft.com/aspnet/core/fundamentals/middleware/>
- <https://learn.microsoft.com/aspnet/core/fundamentals/configuration/>

## Startup Shape

Prefer the modern hosting model:

1. Create `var builder = WebApplication.CreateBuilder(args);`
2. Register services on `builder.Services`
3. Build `var app = builder.Build();`
4. Configure middleware in the correct order
5. Map endpoints
6. Call `app.Run();`

Use older `Startup` patterns only when the repository already uses them or the task is migration.

## Service Registration

- Register framework services explicitly: Razor Pages, controllers, Razor components, authentication, authorization, health checks, rate limiting, response compression, output caching, EF Core, and `IHttpClientFactory`
- Keep business logic in services instead of controllers, page models, or route handlers
- Use constructor injection as the default
- Use options classes for structured configuration
- Choose lifetimes intentionally:
  - singleton: stateless or shared infrastructure
  - scoped: request-bound work such as `DbContext`
  - transient: lightweight stateless services

## Configuration Defaults

`WebApplication.CreateBuilder` already loads configuration from common providers such as:

- `appsettings.json`
- environment-specific `appsettings.{Environment}.json`
- environment variables
- command-line arguments

For secrets:

- use Secret Manager in development
- use a secure external store in production
- do not commit secrets to source control

## Middleware Order

Middleware order is a frequent source of broken behavior. Favor this shape and adjust only with a concrete reason:

1. Forwarded headers if behind a proxy or load balancer
2. Exception handling and HSTS for non-development environments
3. HTTPS redirection
4. Static files
5. Routing when explicit routing middleware is needed
6. CORS when endpoints require it
7. Authentication
8. Authorization
9. Endpoint-specific middleware such as rate limiting or session as required
10. Endpoint mapping with `MapRazorPages`, `MapControllers`, `MapGet`, `MapHub`, or `MapGrpcService`

Important ordering rules:

- Call `UseAuthentication()` before `UseAuthorization()`
- Keep proxy/header processing before auth, redirects, and link generation
- Do not insert custom middleware randomly between auth and authorization without a reason
- In Minimal API apps, explicit `UseRouting()` is usually unnecessary unless you need to control order

## Routing And Endpoints

- Prefer endpoint routing everywhere
- Use route groups for larger Minimal API surfaces
- Keep MVC and API routes explicit and predictable
- Use areas only when the application is large enough to benefit from bounded sections
- Keep endpoint names stable when generating links or integrating with clients

## Error Handling

- Use centralized exception handling instead of scattered `try/catch` blocks for ordinary request failures
- Prefer ProblemDetails-style responses for APIs
- Keep the developer exception page limited to development
- Separate user-facing failures from internal exception details

## Logging And Diagnostics

- Use `ILogger<T>` from DI
- Log structured values, not concatenated strings
- Put correlation and request diagnostics in middleware or infrastructure, not business logic
- Enable HTTP logging only when the scenario warrants it and avoid leaking sensitive data

## Static Assets And Web Root

- Keep public assets in `wwwroot`
- Treat the web root as publicly readable content
- Prevent publishing local-only static content through project file rules when needed
- Use Razor Class Libraries for reusable UI assets across apps

## Architectural Defaults

- Keep `Program.cs` readable; extract feature registration to extension methods when it starts accumulating unrelated concerns
- Prefer vertical slices or feature folders over giant "Controllers", "Services", and "Repositories" buckets with weak boundaries
- Keep framework configuration close to the host and business logic out of it