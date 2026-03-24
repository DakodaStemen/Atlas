---
name: "C3 CLI Reference (Part 4)"
description: "# C3 CLI Reference - Part 4"
---


## Compute Bindings

```jsonc
{
  "services": [{ 
    "binding": "MY_SERVICE", 
    "service": "other-worker",
    "environment": "production"  // Optional: target specific env
  }],
  "ai": { "binding": "AI" },
  "browser": { "binding": "BROWSER" },
  "workflows": [{ "binding": "MY_WORKFLOW", "name": "my-workflow" }]
}
```

### Create workflows

```bash
npx wrangler workflows create my-workflow
```

## Platform Bindings

```jsonc
{
  "analytics_engine_datasets": [{ "binding": "ANALYTICS" }],
  "mtls_certificates": [{ "binding": "MY_CERT", "certificate_id": "..." }],
  "hyperdrive": [{ "binding": "HYPERDRIVE", "id": "..." }],
  "unsafe": {
    "bindings": [{ "name": "RATE_LIMITER", "type": "ratelimit", "namespace_id": "..." }]
  }
}
```

## Configuration Bindings

```jsonc
{
  "vars": {
    "API_URL": "https://api.example.com",
    "MAX_RETRIES": "3"
  },
  "text_blobs": { "MY_TEXT": "./data/template.html" },
  "data_blobs": { "MY_DATA": "./data/config.bin" },
  "wasm_modules": { "MY_WASM": "./build/module.wasm" }
}
```

### Secrets (never in config)

```bash
npx wrangler secret put API_KEY
```

## Environment-Specific Configuration

```jsonc
{
  "name": "my-worker",
  "vars": { "ENV": "production" },
  "kv_namespaces": [{ "binding": "CACHE", "id": "prod-kv-id" }],
  
  "env": {
    "staging": {
      "vars": { "ENV": "staging" },
      "kv_namespaces": [{ "binding": "CACHE", "id": "staging-kv-id" }]
    }
  }
}
```

### Deploy

```bash
npx wrangler deploy              # Production
npx wrangler deploy --env staging
```

## Local Development

```jsonc
{
  "kv_namespaces": [{
    "binding": "MY_KV",
    "id": "prod-id",
    "preview_id": "dev-id"  // Used in wrangler dev
  }]
}
```

### Or use remote

```bash
npx wrangler dev --remote  # Uses production bindings
```

## Complete Example

```jsonc
{
  "$schema": "./node_modules/wrangler/config-schema.json",
  "name": "my-app",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01",
  
  "vars": { "API_URL": "https://api.example.com" },
  "kv_namespaces": [{ "binding": "CACHE", "id": "abc123" }],
  "r2_buckets": [{ "binding": "ASSETS", "bucket_name": "my-assets" }],
  "d1_databases": [{ "binding": "DB", "database_name": "my-db", "database_id": "xyz789" }],
  "services": [{ "binding": "AUTH", "service": "auth-worker" }],
  "ai": { "binding": "AI" }
}
```

## Binding-Specific Configuration

### Durable Objects with Class Export

```jsonc
{
  "durable_objects": {
    "bindings": [
      { "name": "COUNTER", "class_name": "Counter", "script_name": "my-worker" }
    ]
  }
}
```

```typescript
// In same Worker or script_name Worker
export class Counter {
  constructor(private state: DurableObjectState, private env: Env) {}
  async fetch(request: Request) { /* ... */ }
}
```

### Queue Consumers

```jsonc
{
  "queues": {
    "producers": [{ "binding": "MY_QUEUE", "queue": "my-queue" }],
    "consumers": [{ "queue": "my-queue", "max_batch_size": 10 }]
  }
}
```

Queue consumer handler: `export default { async queue(batch, env) { /* process batch.messages */ } }`

## Key Points

- **64 binding limit** (all types combined)
- **Secrets**: Always use `wrangler secret put`, never commit
- **Types**: Run `npx wrangler types` after config changes
- **Environments**: Use `env` field for staging/production variants
- **Development**: Use `preview_id` or `--remote` flag
- **IDs vs Names**: Some bindings use `id` (KV, D1), others use `name` (R2, Queues)

## See Also

- [Wrangler Configuration](https://developers.cloudflare.com/workers/wrangler/configuration/)


---

<!-- merged from: binding-patterns-and-best-practices.md -->

﻿---
name: Binding Patterns and Best Practices
description: # Binding Patterns and Best Practices
 
 ## Service Binding Patterns
---

# Binding Patterns and Best Practices

## Service Binding Patterns (Binding Patterns and Best Practices)

### RPC via Service Bindings

```typescript
// auth-worker
export default {
  async fetch(request: Request, env: Env) {
    const token = request.headers.get('Authorization');
    return new Response(JSON.stringify({ valid: await validateToken(token) }));
  }
}

// api-worker
const response = await env.AUTH_SERVICE.fetch(
  new Request('https://fake-host/validate', {
    headers: { 'Authorization': token }
  })
);
```

**Why RPC?** Zero latency (same datacenter), no DNS, free, type-safe.

#### HTTP vs Service

```typescript
// ❌ HTTP (slow, paid, cross-region latency)
await fetch('https://auth-worker.example.com/validate');

// ✅ Service binding (fast, free, same isolate)
await env.AUTH_SERVICE.fetch(new Request('https://fake-host/validate'));
```

**URL doesn't matter:** Service bindings ignore hostname/protocol, routing happens via binding name.

### Typed Service RPC

```typescript
// shared-types.ts
export interface AuthRequest { token: string; }
export interface AuthResponse { valid: boolean; userId?: string; }

// auth-worker
export default {
  async fetch(request: Request): Promise<Response> {
    const body: AuthRequest = await request.json();
    const response: AuthResponse = { valid: true, userId: '123' };
    return Response.json(response);
  }
}

// api-worker
const response = await env.AUTH_SERVICE.fetch(
  new Request('https://fake/validate', {
    method: 'POST',
    body: JSON.stringify({ token } satisfies AuthRequest)
  })
);
const data: AuthResponse = await response.json();
```

## Secrets Management

```bash
# Set secret
npx wrangler secret put API_KEY
cat api-key.txt | npx wrangler secret put API_KEY
npx wrangler secret put API_KEY --env staging
```

```typescript
// Use secret
const response = await fetch('https://api.example.com', {
  headers: { 'Authorization': `Bearer ${env.API_KEY}` }
});
```

### Never commit secrets

```jsonc
// ❌ NEVER
{ "vars": { "API_KEY": "sk_live_abc123" } }
```

## Testing with Mock Bindings

### Vitest Mock

```typescript
import { vi } from 'vitest';

const mockKV: KVNamespace = {
  get: vi.fn(async (key) => key === 'test' ? 'value' : null),
  put: vi.fn(async () => {}),
  delete: vi.fn(async () => {}),
  list: vi.fn(async () => ({ keys: [], list_complete: true, cursor: '' })),
  getWithMetadata: vi.fn(),
} as unknown as KVNamespace;

const mockEnv: Env = { MY_KV: mockKV };
const mockCtx: ExecutionContext = {
  waitUntil: vi.fn(),
  passThroughOnException: vi.fn(),
};

const response = await worker.fetch(
  new Request('http://localhost/test'),
  mockEnv,
  mockCtx
);
```

## Binding Access Patterns

### Lazy Access

```typescript
// ✅ Access only when needed
if (url.pathname === '/cached') {
  const cached = await env.MY_KV.get('data');
  if (cached) return new Response(cached);
}
```

### Parallel Access

```typescript
// ✅ Parallelize independent calls
const [user, config, cache] = await Promise.all([
  env.DB.prepare('SELECT * FROM users WHERE id = ?').bind(userId).first(),
  env.MY_KV.get('config'),
  env.CACHE.get('data')
]);
```

## Storage Selection

### KV: CDN-Backed Reads

```typescript
const config = await env.MY_KV.get('app-config', { type: 'json' });
```

**Use when:** Read-heavy, <25MB, global distribution, eventual consistency OK  
**Latency:** <10ms reads (cached), writes eventually consistent (60s)

### D1: Relational Queries

```typescript
const results = await env.DB.prepare(`
  SELECT u.name, COUNT(o.id) FROM users u
  LEFT JOIN orders o ON u.id = o.user_id GROUP BY u.id
`).all();
```

**Use when:** Relational data, JOINs, ACID transactions  
**Limits:** 10GB database size, 100k rows per query

### R2: Large Objects

```typescript
const object = await env.MY_BUCKET.get('large-file.zip');
return new Response(object.body);
```

**Use when:** Files >25MB, S3-compatible API needed  
**Limits:** 5TB per object, unlimited storage

### Durable Objects: Coordination

```typescript
const id = env.COUNTER.idFromName('global');
const stub = env.COUNTER.get(id);
await stub.fetch(new Request('https://fake/increment'));
```

**Use when:** Strong consistency, real-time coordination, WebSocket state  
**Guarantees:** Single-threaded execution, transactional storage

## Anti-Patterns

**❌ Hardcoding credentials:** `const apiKey = 'sk_live_abc123'`  
**✅** `npx wrangler secret put API_KEY`

**❌ Using REST API:** `fetch('https://api.cloudflare.com/.../kv/...')`  
**✅** `env.MY_KV.get('key')`

**❌ Polling storage:** `setInterval(() => env.KV.get('config'), 1000)`  
**✅** Use Durable Objects for real-time state

**❌ Large data in vars:** `{ "vars": { "HUGE_CONFIG": "..." } }` (5KB max)  
**✅** `env.MY_KV.put('config', data)`

**❌ Caching env globally:** `const apiKey = env.API_KEY` outside fetch()  
**✅** Access `env.API_KEY` per-request inside fetch()

## See Also

- [Service Bindings Docs](https://developers.cloudflare.com/workers/runtime-apis/bindings/service-bindings/)
- [Miniflare Testing](https://miniflare.dev/)


---

<!-- merged from: resource-configuration.md -->

﻿---
name: Resource Configuration
description: # Resource Configuration
 
 ## Workers (cloudflare.WorkerScript)
---

# Resource Configuration

## Workers (cloudflare.WorkerScript) (Resource Configuration)

```typescript
import * as cloudflare from "@pulumi/cloudflare";
import * as fs from "fs";

const worker = new cloudflare.WorkerScript("my-worker", {
    accountId: accountId,
    name: "my-worker",
    content: fs.readFileSync("./dist/worker.js", "utf8"),
    module: true, // ES modules
    compatibilityDate: "2025-01-01",
    compatibilityFlags: ["nodejs_compat"],
    
    // v6.x: Observability
    logpush: true, // Enable Workers Logpush
    tailConsumers: [{service: "log-consumer"}], // Stream logs to Worker
    
    // v6.x: Placement
    placement: {mode: "smart"}, // Smart placement for latency optimization
    
    // Bindings
    kvNamespaceBindings: [{name: "MY_KV", namespaceId: kv.id}],
    r2BucketBindings: [{name: "MY_BUCKET", bucketName: bucket.name}],
    d1DatabaseBindings: [{name: "DB", databaseId: db.id}],
    queueBindings: [{name: "MY_QUEUE", queue: queue.id}],
    serviceBindings: [{name: "OTHER_SERVICE", service: other.name}],
    plainTextBindings: [{name: "ENV_VAR", text: "value"}],
    secretTextBindings: [{name: "API_KEY", text: secret}],
    
    // v6.x: Advanced bindings
    analyticsEngineBindings: [{name: "ANALYTICS", dataset: "my-dataset"}],
    browserBinding: {name: "BROWSER"}, // Browser Rendering
    aiBinding: {name: "AI"}, // Workers AI
    hyperdriveBindings: [{name: "HYPERDRIVE", id: hyperdriveConfig.id}],
});
```

## Workers KV (cloudflare.WorkersKvNamespace)

```typescript
const kv = new cloudflare.WorkersKvNamespace("my-kv", {
    accountId: accountId,
    title: "my-kv-namespace",
});

// Write values
const kvValue = new cloudflare.WorkersKvValue("config", {
    accountId: accountId,
    namespaceId: kv.id,
    key: "config",
    value: JSON.stringify({foo: "bar"}),
});
```

## R2 Buckets (cloudflare.R2Bucket)

```typescript
const bucket = new cloudflare.R2Bucket("my-bucket", {
    accountId: accountId,
    name: "my-bucket",
    location: "auto", // or "wnam", etc.
});
```

## D1 Databases (cloudflare.D1Database)

```typescript
const db = new cloudflare.D1Database("my-db", {accountId, name: "my-database"});

// Migrations via wrangler
import * as command from "@pulumi/command";
const migration = new command.local.Command("d1-migration", {
    create: pulumi.interpolate`wrangler d1 execute ${db.name} --file ./schema.sql`,
}, {dependsOn: [db]});
```

## Queues (cloudflare.Queue)

```typescript
const queue = new cloudflare.Queue("my-queue", {accountId, name: "my-queue"});

// Producer
const producer = new cloudflare.WorkerScript("producer", {
    accountId, name: "producer", content: code,
    queueBindings: [{name: "MY_QUEUE", queue: queue.id}],
});

// Consumer
const consumer = new cloudflare.WorkerScript("consumer", {
    accountId, name: "consumer", content: code,
    queueConsumers: [{queue: queue.name, maxBatchSize: 10, maxRetries: 3}],
});
```

## Pages Projects (cloudflare.PagesProject)

```typescript
const pages = new cloudflare.PagesProject("my-site", {
    accountId, name: "my-site", productionBranch: "main",
    buildConfig: {buildCommand: "npm run build", destinationDir: "dist"},
    source: {
        type: "github",
        config: {owner: "my-org", repoName: "my-repo", productionBranch: "main"},
    },
    deploymentConfigs: {
        production: {
            environmentVariables: {NODE_VERSION: "18"},
            kvNamespaces: {MY_KV: kv.id},
            d1Databases: {DB: db.id},
        },
    },
});
```

## DNS Records (cloudflare.DnsRecord)

```typescript
const zone = cloudflare.getZone({name: "example.com"});
const record = new cloudflare.DnsRecord("www", {
    zoneId: zone.then(z => z.id), name: "www", type: "A",
    content: "192.0.2.1", ttl: 3600, proxied: true,
});
```

## Workers Domains/Routes

```typescript
// Route (pattern-based)
const route = new cloudflare.WorkerRoute("my-route", {
    zoneId: zoneId,
    pattern: "example.com/api/*",
    scriptName: worker.name,
});

// Domain (dedicated subdomain)
const domain = new cloudflare.WorkersDomain("my-domain", {
    accountId: accountId,
    hostname: "api.example.com",
    service: worker.name,
    zoneId: zoneId,
});
```

## Assets Configuration (v6.x)

Serve static assets from Workers:

```typescript
const worker = new cloudflare.WorkerScript("app", {
    accountId: accountId,
    name: "my-app",
    content: code,
    assets: {
        path: "./public", // Local directory
        // Assets uploaded and served from Workers
    },
});
```

## v6.x Versioned Deployments (Advanced)

For gradual rollouts, use 3-resource pattern:

```typescript
// 1. Worker (container for versions)
const worker = new cloudflare.Worker("api", {
    accountId: accountId,
    name: "api-worker",
});

// 2. Version (immutable code + config)
const version = new cloudflare.WorkerVersion("v1", {
    accountId: accountId,
    workerId: worker.id,
    content: fs.readFileSync("./dist/worker.js", "utf8"),
    compatibilityDate: "2025-01-01",
    compatibilityFlags: ["nodejs_compat"],
    // Note: Bindings configured at deployment level
});

// 3. Deployment (version + bindings + traffic split)
const deployment = new cloudflare.WorkersDeployment("prod", {
    accountId: accountId,
    workerId: worker.id,
    versionId: version.id,
    // Bindings applied to deployment
    kvNamespaceBindings: [{name: "MY_KV", namespaceId: kv.id}],
});
```

**When to use:** Blue-green deployments, canary releases, gradual rollouts  
**When NOT to use:** Simple single-version deployments (use WorkerScript)

---
See: [README.md](./README.md), [api.md](./api.md), [patterns.md](./patterns.md), [gotchas.md](./gotchas.md)


---

<!-- merged from: configuration.md -->

﻿---
name: Configuration
description: # Configuration
 
 Setup and configuration for TCP Sockets in Cloudflare Workers.
---

# Configuration

Setup and configuration for TCP Sockets in Cloudflare Workers.

## Wrangler Configuration

### Basic Setup

TCP Sockets are available by default in Workers runtime. No special configuration required in `wrangler.jsonc`:

```jsonc
{
  "name": "private-network-worker",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01"
}
```

### Environment Variables

Store connection details as env vars:

```jsonc
{
  "vars": { "DB_HOST": "10.0.1.50", "DB_PORT": "5432" }
}
```

```typescript
interface Env { DB_HOST: string; DB_PORT: string; }

export default {
  async fetch(req: Request, env: Env): Promise<Response> {
    const socket = connect({ hostname: env.DB_HOST, port: parseInt(env.DB_PORT) });
  }
};
```

### Per-Environment Configuration

```jsonc
{
  "vars": { "DB_HOST": "localhost" },
  "env": {
    "staging": { "vars": { "DB_HOST": "staging-db.internal.net" } },
    "production": { "vars": { "DB_HOST": "prod-db.internal.net" } }
  }
}
```

Deploy: `wrangler deploy --env staging` or `wrangler deploy --env production`

## Integration with Cloudflare Tunnel

To connect Workers to private networks, combine TCP Sockets with Cloudflare Tunnel:

```text
Worker (TCP Socket) → Tunnel hostname → cloudflared → Private Network
```

### Quick Setup

1. **Install cloudflared** on a server inside your private network
2. **Create tunnel**: `cloudflared tunnel create my-private-network`
3. **Configure routing** in `config.yml`:

```yaml
tunnel: <TUNNEL_ID>
credentials-file: /path/to/<TUNNEL_ID>.json
ingress:
  - hostname: db.internal.example.com
    service: tcp://10.0.1.50:5432
  - service: http_status:404  # Required catch-all
```

1. **Run tunnel**: `cloudflared tunnel run my-private-network`
2. **Connect from Worker**:

```typescript
const socket = connect(
  { hostname: "db.internal.example.com", port: 5432 },  // Tunnel hostname
  { secureTransport: "on" }
);
```

For detailed Tunnel setup, see [Tunnel configuration reference](../tunnel/configuration.md).

## Smart Placement Integration

Reduce latency by auto-placing Workers near backends:

```jsonc
{ "placement": { "mode": "smart" } }
```

Workers automatically relocate closer to TCP socket destinations after observing connection latency. See [Smart Placement reference](../smart-placement/).

## Secrets Management

Store sensitive credentials as secrets (not in wrangler.jsonc):

```bash
wrangler secret put DB_PASSWORD  # Enter value when prompted
```

Access in Worker via `env.DB_PASSWORD`. Use in protocol handshake or authentication.

## Local Development

Test with `wrangler dev`. Note: Local mode may not access private networks. Use public endpoints or mock servers for development:

```typescript
const config = process.env.NODE_ENV === 'dev' 
  ? { hostname: 'localhost', port: 5432 }  // Mock
  : { hostname: 'db.internal.example.com', port: 5432 };  // Production
```

## Connection String Patterns

Parse connection strings to extract host and port:

```typescript
function parseConnectionString(connStr: string): SocketAddress {
  const url = new URL(connStr); // e.g., "postgres://10.0.1.50:5432/mydb"
  return { hostname: url.hostname, port: parseInt(url.port) || 5432 };
}
```

## Hyperdrive Integration

For PostgreSQL/MySQL, prefer Hyperdrive over raw TCP sockets (includes connection pooling):

```jsonc
{ "hyperdrive": [{ "binding": "DB", "id": "<HYPERDRIVE_ID>" }] }
```

See [Hyperdrive reference](../hyperdrive/) for complete setup.

## Compatibility

TCP Sockets available in all modern Workers. Use current date: `"compatibility_date": "2025-01-01"`. No special flags required.

## Related Configuration

- **[Tunnel Configuration](../tunnel/configuration.md)** - Detailed cloudflared setup
- **[Smart Placement](../smart-placement/configuration.md)** - Placement mode options
- **[Hyperdrive](../hyperdrive/configuration.md)** - Database connection pooling setup


---

<!-- merged from: configuration-deployment.md -->

﻿---
name: Configuration & Deployment
description: # Configuration & Deployment
 
 ## Dashboard Setup
---

# Configuration & Deployment

## Dashboard Setup (Configuration & Deployment)

1. Navigate to <https://dash.cloudflare.com/?to=/:account/calls>
2. Click "Create Application" (or use existing app)
3. Copy `CALLS_APP_ID` from dashboard
4. Generate and copy `CALLS_APP_SECRET` (treat as sensitive credential)
5. Use credentials in Wrangler config or environment variables below

## Dependencies

**Backend (Workers):** Built-in fetch API, no additional packages required

### Client (PartyTracks)

```bash
npm install partytracks @cloudflare/calls
```

#### Client (React + PartyTracks)

```bash
npm install partytracks @cloudflare/calls observable-hooks
# Observable hooks: useObservableAsValue, useValueAsObservable
```

**Client (Raw API):** Native browser WebRTC API only

## Wrangler Setup

```jsonc
{
  "name": "my-calls-app",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01", // Use current date for new projects
  "vars": {
    "CALLS_APP_ID": "your-app-id",
    "MAX_WEBCAM_BITRATE": "1200000",
    "MAX_WEBCAM_FRAMERATE": "24",
    "MAX_WEBCAM_QUALITY_LEVEL": "1080"
  },
  // Set secret: wrangler secret put CALLS_APP_SECRET
  "durable_objects": {
    "bindings": [
      {
        "name": "ROOM",
        "class_name": "Room"
      }
    ]
  }
}
```

## Deploy

```bash
wrangler login
wrangler secret put CALLS_APP_SECRET
wrangler deploy
```

## Environment Variables

### Required

- `CALLS_APP_ID`: From dashboard
- `CALLS_APP_SECRET`: From dashboard (secret)

#### Optional

- `MAX_WEBCAM_BITRATE` (default: 1200000)
- `MAX_WEBCAM_FRAMERATE` (default: 24)
- `MAX_WEBCAM_QUALITY_LEVEL` (default: 1080)
- `TURN_SERVICE_ID`: TURN service
- `TURN_SERVICE_TOKEN`: TURN auth (secret)

## TURN Configuration

```javascript
const pc = new RTCPeerConnection({
  iceServers: [
    { urls: 'stun:stun.cloudflare.com:3478' },
    {
      urls: [
        'turn:turn.cloudflare.com:3478?transport=udp',
        'turn:turn.cloudflare.com:3478?transport=tcp',
        'turns:turn.cloudflare.com:5349?transport=tcp'
      ],
      username: turnUsername,
      credential: turnCredential
    }
  ],
  bundlePolicy: 'max-bundle', // Recommended: reduces overhead
  iceTransportPolicy: 'all'    // Use 'relay' to force TURN (testing only)
});
```

**Ports:** 3478 (UDP/TCP), 53 (UDP), 80 (TCP), 443 (TLS), 5349 (TLS)

**When to use TURN:** Required for restrictive corporate firewalls/networks that block UDP. ~5-10% of connections fallback to TURN. STUN works for most users.

**ICE candidate filtering:** Cloudflare handles candidate filtering automatically. No need to manually filter candidates.

## Durable Object Boilerplate

Minimal presence system:

```typescript
export class Room {
  private sessions = new Map<string, {userId: string, tracks: string[]}>();

  async fetch(req: Request) {
    const {pathname} = new URL(req.url);
    const body = await req.json();
    
    if (pathname === '/join') {
      this.sessions.set(body.sessionId, {userId: body.userId, tracks: []});
      return Response.json({participants: this.sessions.size});
    }
    
    if (pathname === '/publish') {
      this.sessions.get(body.sessionId)?.tracks.push(...body.tracks);
      // Broadcast to others via WebSocket (not shown)
      return new Response('OK');
    }
    
    return new Response('Not found', {status: 404});
  }
}
```

## Environment Validation

Check credentials before first API call:

```typescript
if (!env.CALLS_APP_ID || !env.CALLS_APP_SECRET) {
  throw new Error('CALLS_APP_ID and CALLS_APP_SECRET required');
}
```


---

<!-- merged from: configuration-setup.md -->

﻿---
name: Configuration & Setup
description: # Configuration & Setup
 
 ## Installation
---

# Configuration & Setup

## Installation (Configuration & Setup)

```bash
npm install @cloudflare/puppeteer  # or @cloudflare/playwright
```

**Use Cloudflare packages** - standard `puppeteer`/`playwright` won't work in Workers.

## wrangler.json

```json
{
  "name": "browser-worker",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01",
  "compatibility_flags": ["nodejs_compat"],
  "browser": {
    "binding": "MYBROWSER"
  }
}
```

**Required:** `nodejs_compat` flag and `browser.binding`.

## TypeScript

```typescript
interface Env {
  MYBROWSER: Fetcher;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // ...
  }
} satisfies ExportedHandler<Env>;
```

## Development

```bash
wrangler dev --remote  # --remote required for browser binding
```

**Local mode does NOT support Browser Rendering** - must use `--remote`.

## REST API

No wrangler config needed. Get API token with "Browser Rendering - Edit" permission.

```bash
curl -X POST \
  'https://api.cloudflare.com/client/v4/accounts/{accountId}/browser-rendering/screenshot' \
  -H 'Authorization: Bearer TOKEN' \
  -d '{"url": "https://example.com"}' --output screenshot.png
```

## Requirements

| Requirement | Value |
| ------------- | ------- |
| Node.js compatibility | `nodejs_compat` flag |
| Compatibility date | 2023-03-01+ |
| Module format | ES modules only |
| Browser | Chromium 119+ (no Firefox/Safari) |

**Not supported:** WebGL, WebRTC, extensions, `file://` protocol, Service Worker syntax.

## Troubleshooting

| Error | Solution |
| ------- | ---------- |
| `MYBROWSER is undefined` | Use `wrangler dev --remote` |
| `nodejs_compat not enabled` | Add to `compatibility_flags` |
| `Module not found` | `npm install @cloudflare/puppeteer` |
| `Browser Rendering not available` | Enable in dashboard |


---

<!-- merged from: troubleshooting.md -->

﻿---
name: Troubleshooting
description: # Troubleshooting
 
 ## Job fails with size or seconds errors
---

# Troubleshooting

## Job fails with size or seconds errors (Troubleshooting)

- Cause: size not supported by model, or seconds not in 4/8/12.
- Fix: match size to model; use only "4", "8", or "12" seconds (see `references/video-api.md`).
- If you see `invalid_type` for seconds, update `scripts/sora.py` or pass a string value for `--seconds`.

## openai SDK not installed

- Cause: running `python "$SORA_CLI" ...` without the OpenAI SDK available.
- Fix: run with `uv run --with openai python "$SORA_CLI" ...` instead of using pip directly.

## uv cache permission error

- Cause: uv cache directory is not writable in CI or sandboxed environments.
- Fix: set `UV_CACHE_DIR=/tmp/uv-cache` (or another writable path) before running `uv`.

## Prompt shell escaping issues

- Cause: multi-line prompts or quotes break the shell.
- Fix: use `--prompt-file prompt.txt` (see `references/cli.md` for an example).

## Prompt looks double-wrapped ("Primary request: Use case: ...")

- Cause: you structured the prompt manually but left CLI augmentation on.
- Fix: add `--no-augment` when passing a structured prompt file, or use the CLI fields (`--use-case`, `--scene`, etc.) instead of pre-formatting.

## Input reference rejected

- Cause: file is not jpg/png/webp, or has a human face, or dimensions do not match target size.
- Fix: convert to jpg/png/webp, remove faces, and resize to match `--size`.

## Download fails or returns expired URL

- Cause: download URLs expire after about 1 hour.
- Fix: re-download while the link is fresh; save to your own storage.

## Video completes but looks unstable or flickers

- Cause: multiple actions or aggressive camera motion.
- Fix: reduce to one main action and one camera move; keep beats simple; add constraints like "avoid flicker" or "stable motion".

## Text is unreadable

- Cause: text too long, too small, or moving.
- Fix: shorten text, increase size, keep camera locked-off, and avoid fast motion.

## Remix drifts from the original

- Cause: too many changes requested at once.
- Fix: state invariants explicitly ("same shot and camera move") and change only one element per remix.

## Job stuck in queued/in_progress for a long time

- Cause: temporary queue delays.
- Fix: increase poll timeout, or retry later; avoid high concurrency if you are rate-limited.

## create-and-poll times out in CI/sandbox

- Cause: long-running CLI commands can exceed CI time limits.
- Fix: run `create` (capture the ID) and then `poll` separately, or set `--timeout`.

## Audio or voiceover missing / incorrect

- Cause: audio wasn't explicitly requested, or the dialogue/audio cue was too long or vague.
- Fix: add a clear `Audio:` line and a short `Dialogue:` block.

## Cleanup blocked by sandbox policy

- Cause: some environments block `rm`.
- Fix: skip cleanup, or truncate files instead of deleting.


---

<!-- merged from: troubleshooting-best-practices.md -->

﻿---
name: Troubleshooting & Best Practices
description: # Troubleshooting & Best Practices
 
 ## Common Errors
---

# Troubleshooting & Best Practices

## Common Errors (Troubleshooting & Best Practices)

### "No bundler/build step" - Pulumi uploads raw code

**Problem:** Worker fails with "Cannot use import statement outside a module"  
**Cause:** Pulumi doesn't bundle Worker code - uploads exactly what you provide  
**Solution:** Build Worker BEFORE Pulumi deploy

```typescript
// WRONG: Pulumi won't bundle this
const worker = new cloudflare.WorkerScript("worker", {
    content: fs.readFileSync("./src/index.ts", "utf8"), // Raw TS file
});

// RIGHT: Build first, then deploy
import * as command from "@pulumi/command";
const build = new command.local.Command("build", {
    create: "npm run build",
    dir: "./worker",
});
const worker = new cloudflare.WorkerScript("worker", {
    content: build.stdout.apply(() => fs.readFileSync("./worker/dist/index.js", "utf8")),
}, {dependsOn: [build]});
```

### "wrangler.toml not consumed" - Config drift

**Problem:** Local wrangler dev works, Pulumi deploy fails  
**Cause:** Pulumi ignores wrangler.toml - must duplicate config  
**Solution:** Generate wrangler.toml from Pulumi or keep synced manually

```typescript
// Pattern: Export Pulumi config to wrangler.toml
const workerConfig = {
    name: "my-worker",
    compatibilityDate: "2025-01-01",
    compatibilityFlags: ["nodejs_compat"],
};

new command.local.Command("generate-wrangler", {
    create: pulumi.interpolate`cat > wrangler.toml <<EOF
name = "${workerConfig.name}"
compatibility_date = "${workerConfig.compatibilityDate}"
compatibility_flags = ${JSON.stringify(workerConfig.compatibilityFlags)}
EOF`,
});
```

### "False no-changes detection" - Content SHA unchanged

**Problem:** Worker code updated, Pulumi says "no changes"  
**Cause:** Content hash identical (whitespace/comment-only change)  
**Solution:** Add build timestamp or version to force update

```typescript
const version = Date.now().toString();
const worker = new cloudflare.WorkerScript("worker", {
    content: code,
    plainTextBindings: [{name: "VERSION", text: version}], // Forces new deployment
});
```

### "D1 migrations don't run on pulumi up"

**Problem:** Database schema not applied after D1 database created  
**Cause:** Pulumi creates database but doesn't run migrations  
**Solution:** Use Command resource with dependsOn

```typescript
const db = new cloudflare.D1Database("db", {accountId, name: "mydb"});

// Run migrations after DB created
const migration = new command.local.Command("migrate", {
    create: pulumi.interpolate`wrangler d1 execute ${db.name} --file ./schema.sql`,
}, {dependsOn: [db]});

// Worker depends on migrations
const worker = new cloudflare.WorkerScript("worker", {
    d1DatabaseBindings: [{name: "DB", databaseId: db.id}],
}, {dependsOn: [migration]});
```

### "Missing required property 'accountId'"

**Problem:** `Error: Missing required property 'accountId'`  
**Cause:** Account ID not provided in resource configuration  
**Solution:** Add to stack config

```yaml
# Pulumi.<stack>.yaml
config:
  cloudflare:accountId: "abc123..."
```

### "Binding name mismatch"

**Problem:** Worker fails with "env.MY_KV is undefined"  
**Cause:** Binding name in Pulumi != name in Worker code  
**Solution:** Match exactly (case-sensitive)

```typescript
// Pulumi
kvNamespaceBindings: [{name: "MY_KV", namespaceId: kv.id}]

// Worker code
export default { async fetch(request, env) { await env.MY_KV.get("key"); }}
```

### "API token permissions insufficient"

**Problem:** `Error: authentication error (10000)`  
**Cause:** Token lacks required permissions  
**Solution:** Grant token permissions: Account.Workers Scripts:Edit, Account.Account Settings:Read

### "Resource not found after import"

**Problem:** Imported resource shows as changed on next `pulumi up`  
**Cause:** State mismatch between actual resource and Pulumi config  
**Solution:** Check property names/types match exactly

```bash
pulumi import cloudflare:index/workerScript:WorkerScript my-worker <account_id>/<worker_name>
pulumi preview # If shows changes, adjust Pulumi code to match actual resource
```

### "v6.x Worker versioning confusion"

**Problem:** Worker deployed but not receiving traffic  
**Cause:** v6.x requires Worker + WorkerVersion + WorkersDeployment (3 resources)  
**Solution:** Use WorkerScript (auto-versioning) OR full versioning pattern

```typescript
// SIMPLE: WorkerScript auto-versions (default behavior)
const worker = new cloudflare.WorkerScript("worker", {
    accountId, name: "my-worker", content: code,
});

// ADVANCED: Manual versioning for gradual rollouts (v6.x)
const worker = new cloudflare.Worker("worker", {accountId, name: "my-worker"});
const version = new cloudflare.WorkerVersion("v1", {
    accountId, workerId: worker.id, content: code, compatibilityDate: "2025-01-01",
});
const deployment = new cloudflare.WorkersDeployment("prod", {
    accountId, workerId: worker.id, versionId: version.id,
});
```

## Best Practices

1. **Always set compatibilityDate** - Locks Worker behavior, prevents breaking changes
2. **Build before deploy** - Pulumi doesn't bundle; use Command resource or CI build step
3. **Match binding names** - Case-sensitive, must match between Pulumi and Worker code
4. **Use dependsOn for migrations** - Ensure D1 migrations run before Worker deploys
5. **Version Worker content** - Add VERSION binding to force redeployment on content changes
6. **Store secrets in stack config** - Use `pulumi config set --secret` for API keys

## Limits

| Resource | Limit | Notes |
| ---------- | ------- | ------- |
| Worker script size | 10 MB | Includes all dependencies, after compression |
| Worker CPU time | 50ms (free), 30s (paid) | Per request |
| KV keys per namespace | Unlimited | 1000 ops/sec write, 100k ops/sec read |
| R2 storage | Unlimited | Class A ops: 1M/mo free, Class B: 10M/mo free |
| D1 databases | 50,000 per account | Free: 10 per account, 5 GB each |
| Queues | 10,000 per account | Free: 1M ops/day |
| Pages projects | 500 per account | Free: 100 projects |
| API requests | Varies by plan | ~1200 req/5min on free |

## Resources

- **Pulumi Registry:** <https://www.pulumi.com/registry/packages/cloudflare/>
- **API Docs:** <https://www.pulumi.com/registry/packages/cloudflare/api-docs/>
- **GitHub:** <https://github.com/pulumi/pulumi-cloudflare>
- **Cloudflare Docs:** <https://developers.cloudflare.com/>
- **Workers Docs:** <https://developers.cloudflare.com/workers/>

---
See: [README.md](./README.md), [configuration.md](./configuration.md), [api.md](./api.md), [patterns.md](./patterns.md)


---

<!-- merged from: basic-troubleshooting-deploy-time-and-startup.md -->

﻿---
name: Basic troubleshooting (deploy-time and startup)
description: # Basic troubleshooting (deploy-time and startup)
 
 Use this when a deploy fails, the service crashes on start, or health checks time out.
---

# Basic troubleshooting (deploy-time and startup)

Use this when a deploy fails, the service crashes on start, or health checks time out.
Keep fixes minimal and redeploy after each change.

## 1) Classify the failure

- **Build failure**: errors in build logs, missing dependencies, build command issues.
- **Startup failure**: app exits quickly, crashes, or cannot bind to `$PORT`.
- **Runtime/health failure**: service is live but health checks fail or 5xx errors.

## 2) Quick checks by class

### Build failure

- Confirm the build command is correct for the runtime.
- Ensure required dependencies are present in `package.json`, `requirements.txt`, etc.
- Check for missing build-time env vars.

#### Startup failure

- Confirm the start command and working directory.
- Ensure port binding is `0.0.0.0:$PORT`.
- Check for missing runtime env vars (secrets, DB URLs).

#### Runtime/health failure

- Verify the health endpoint path and response.
- Confirm the app is actually listening on `$PORT`.
- Check database connectivity and migrations.

## 3) Map error signatures to fixes

Use [error-patterns.md](error-patterns.md) for a compact catalog of common log messages.

## 4) If still blocked

Gather the latest build logs and runtime error logs, then consider the optional
`render-debug` skill for deeper diagnostics (metrics, DB checks, expanded patterns).


---

<!-- merged from: control-which-queues-a-worker-listens-to.md -->

﻿---
name: Control Which Queues a Worker Listens To
description: ## Control Which Queues a Worker Listens To
 
 Configure `listenQueues` in DBOS configuration to make a process only dequeue from specific queues. This enables heterogeneous worker pools.
tags: queue, listen, worker, process, configuration
---

## Control Which Queues a Worker Listens To

Configure `listenQueues` in DBOS configuration to make a process only dequeue from specific queues. This enables heterogeneous worker pools.

### Incorrect (all workers process all queues)

```typescript
import { DBOS, WorkflowQueue } from "@dbos-inc/dbos-sdk";

const cpuQueue = new WorkflowQueue("cpu_queue");
const gpuQueue = new WorkflowQueue("gpu_queue");

// Every worker processes both CPU and GPU tasks
// GPU tasks on CPU workers will fail or be slow!
DBOS.setConfig({
  name: "my-app",
  systemDatabaseUrl: process.env.DBOS_SYSTEM_DATABASE_URL,
});
await DBOS.launch();
```

#### Correct (selective queue listening)

```typescript
import { DBOS, WorkflowQueue } from "@dbos-inc/dbos-sdk";

const cpuQueue = new WorkflowQueue("cpu_queue");
const gpuQueue = new WorkflowQueue("gpu_queue");

async function main() {
  const workerType = process.env.WORKER_TYPE; // "cpu" or "gpu"

  const config: any = {
    name: "my-app",
    systemDatabaseUrl: process.env.DBOS_SYSTEM_DATABASE_URL,
  };

  if (workerType === "gpu") {
    config.listenQueues = [gpuQueue];
  } else if (workerType === "cpu") {
    config.listenQueues = [cpuQueue];
  }

  DBOS.setConfig(config);
  await DBOS.launch();
}
```

`listenQueues` only controls dequeuing. A CPU worker can still enqueue tasks onto the GPU queue:

```typescript
// From a CPU worker, enqueue onto the GPU queue
await DBOS.startWorkflow(gpuTask, { queueName: gpuQueue.name })("data");
```

Reference: [Explicit Queue Listening](https://docs.dbos.dev/typescript/tutorials/queue-tutorial#explicit-queue-listening)


---

<!-- merged from: apply-principle-of-least-privilege.md -->

﻿---
name: Apply Principle of Least Privilege
description: ## Apply Principle of Least Privilege
 
 Grant only the minimum permissions required. Never use superuser for application queries.
tags: privileges, security, roles, permissions
---

## Apply Principle of Least Privilege

Grant only the minimum permissions required. Never use superuser for application queries.

### Incorrect (overly broad permissions)

```sql
-- Application uses superuser connection
-- Or grants ALL to application role
grant all privileges on all tables in schema public to app_user;
grant all privileges on all sequences in schema public to app_user;

-- Any SQL injection becomes catastrophic
-- drop table users; cascades to everything
```

#### Correct (minimal, specific grants)

```sql
-- Create role with no default privileges
create role app_readonly nologin;

-- Grant only SELECT on specific tables
grant usage on schema public to app_readonly;
grant select on public.products, public.categories to app_readonly;

-- Create role for writes with limited scope
create role app_writer nologin;
grant usage on schema public to app_writer;
grant select, insert, update on public.orders to app_writer;
grant usage on sequence orders_id_seq to app_writer;
-- No DELETE permission

-- Login role inherits from these
create role app_user login password 'xxx';
grant app_writer to app_user;
```

Revoke public defaults:

```sql
-- Revoke default public access
revoke all on schema public from public;
revoke all on all tables in schema public from public;
```

Reference: [Roles and Privileges](https://supabase.com/blog/postgres-roles-and-privileges)


---

<!-- merged from: versioning-and-upgrades.md -->

﻿---
name: Versioning And Upgrades
description: # Versioning And Upgrades
 
 Primary docs:
---

# Versioning And Upgrades

Primary docs:

- <https://learn.microsoft.com/aspnet/core/release-notes/>
- <https://learn.microsoft.com/aspnet/core/release-notes/aspnetcore-10.0>
- <https://learn.microsoft.com/aspnet/core/release-notes/aspnetcore-9.0>
- <https://github.com/dotnet/AspNetCore.Docs/tree/main/aspnetcore/breaking-changes>

## Versioning Default

- For new production apps in March 2026, prefer `net10.0`
- For existing apps, match the repository's target framework unless the task is explicitly an upgrade
- Before using a new API, confirm it exists in the target framework

## Upgrade Workflow

1. Identify the current target framework and SDK
2. Read the "What's new" and breaking-changes pages for each version hop
3. Compile and resolve obsoletions intentionally
4. Re-run integration tests and auth flows
5. Re-test deployment-specific behavior such as proxies, cookies, and static assets

## High-Value Breaking-Change Checks

When moving to ASP.NET Core 10, watch for:

- cookie login redirects disabled for known API endpoints
- `WithOpenApi` deprecation
- `WebHostBuilder`, `IWebHost`, and `WebHost` obsolescence
- Razor runtime compilation obsolescence

When moving to ASP.NET Core 9, watch for:

- `ValidateOnBuild` and `ValidateScopes` enabled in development when using `HostBuilder`
- middleware constructor expectations and DI validation changes

When moving to ASP.NET Core 8, watch for:

- Minimal API `IFormFile` antiforgery requirements
- `AddRateLimiter()` and `AddHttpLogging()` requirements when corresponding middleware is used

## Migration Principles

- Prefer migration to the modern hosting model when touching startup extensively
- Remove compatibility shims only after tests confirm behavior
- Avoid mixing new framework idioms with old startup architecture in a half-migrated state
- Keep one authoritative target framework in project files unless multi-targeting is deliberate

## Preview Feature Rule

Do not introduce preview-only APIs or docs guidance unless the user explicitly asks for preview adoption or the repository is already on preview SDKs.


---

<!-- merged from: features-capabilities.md -->

﻿---
name: Features & Capabilities
description: # Features & Capabilities
 
 ## Caching
---

# Features & Capabilities

## Caching (Features & Capabilities)

Dashboard: Settings → Cache Responses → Enable

```typescript
// Custom TTL (1 hour)
headers: { 'cf-aig-cache-ttl': '3600' }

// Skip cache
headers: { 'cf-aig-skip-cache': 'true' }

// Custom cache key
headers: { 'cf-aig-cache-key': 'greeting-en' }
```

### Limits:**TTL 60s - 30 days.**Does NOT work with streaming

## Rate Limiting

Dashboard: Settings → Rate-limiting → Enable

- **Fixed window:** Resets at intervals
- **Sliding window:** Rolling window (more accurate)
- Returns `429` when exceeded

## Guardrails

Dashboard: Settings → Guardrails → Enable

Filter prompts/responses for inappropriate content. Actions: Flag (log) or Block (reject).

## Data Loss Prevention (DLP)

Dashboard: Settings → DLP → Enable

Detect PII (emails, SSNs, credit cards). Actions: Flag, Block, or Redact.

## Billing Modes

| Mode | Description | Setup |
| ------ | ------------- | ------- |
| **Unified Billing** | Pay through Cloudflare, no provider keys | Use `cf-aig-authorization` header only |
| **BYOK** | Store provider keys in dashboard | Add keys in Provider Keys section |
| **Pass-through** | Send provider key with each request | Include provider's auth header |

## Zero Data Retention

Dashboard: Settings → Privacy → Zero Data Retention

No prompts/responses stored. Request counts and costs still tracked.

## Logging

Dashboard: Settings → Logs → Enable (up to 10M logs)

Each entry: prompt, response, provider, model, tokens, cost, duration, cache status, metadata.

```typescript
// Skip logging for request
headers: { 'cf-aig-collect-log': 'false' }
```

**Export:** Use Logpush to S3, GCS, Datadog, Splunk, etc.

## Custom Cost Tracking

For models not in Cloudflare's pricing database:

Dashboard: Gateway → Settings → Custom Costs

Or via API: set `model`, `input_cost`, `output_cost`.

## Supported Providers (22+)

| Provider | Unified API | Notes |
| ---------- | ------------- | ------- |
| OpenAI | `openai/gpt-4o` | Full support |
| Anthropic | `anthropic/claude-sonnet-4-5` | Full support |
| Google AI | `google-ai-studio/gemini-2.0-flash` | Full support |
| Workers AI | `workersai/@cf/meta/llama-3` | Native |
| Azure OpenAI | `azure-openai/*` | Deployment names |
| AWS Bedrock | Provider endpoint only | `/bedrock/*` |
| Groq | `groq/*` | Fast inference |
| Mistral, Cohere, Perplexity, xAI, DeepSeek, Cerebras | Full support | - |

## Best Practices

1. Enable caching for deterministic prompts
2. Set rate limits to prevent abuse
3. Use guardrails for user-facing AI
4. Enable DLP for sensitive data
5. Use unified billing or BYOK for simpler key management
6. Enable logging for debugging
7. Use zero data retention when privacy required


---

<!-- merged from: framework-integration.md -->

﻿---
name: Framework Integration
description: # Framework Integration
 
 **Web Analytics is dashboard-only** - no programmatic API. This covers beacon integration.
---

# Framework Integration

**Web Analytics is dashboard-only** - no programmatic API. This covers beacon integration.

## Basic HTML

```html
<script defer src='https://static.cloudflareinsights.com/beacon.min.js' 
        data-cf-beacon='{"token": "YOUR_TOKEN", "spa": true}'></script>
```

Place before closing `</body>` tag.

## Framework Examples

| Framework | Location | Notes |
| ----------- | ---------- | ------- |
| React/Vite | `public/index.html` | Add `spa: true` |
| Next.js App Router | `app/layout.tsx` | Use `<Script strategy="afterInteractive">` |
| Next.js Pages | `pages/_document.tsx` | Use `<Script>` |
| Nuxt 3 | `app.vue` with `useHead()` | Or use plugin |
| Vue 3/Vite | `index.html` | Add `spa: true` |
| Gatsby | `gatsby-browser.js` | `onClientEntry` hook |
| SvelteKit | `src/app.html` | Before `</body>` |
| Astro | Layout component | Before `</body>` |
| Angular | `src/index.html` | Add `spa: true` |
| Docusaurus | `docusaurus.config.js` | In `scripts` array |

## Configuration

```json
{
  "token": "YOUR_TOKEN",
  "spa": true
}
```

**Use `spa: true` for:** React Router, Vue Router, Next.js, Nuxt, Gatsby, SvelteKit, Angular

**Use `spa: false` for:** Traditional server-rendered (PHP, Django, Rails, WordPress)

## CSP Headers

```text
script-src 'self' https://static.cloudflareinsights.com;
connect-src 'self' https://cloudflareinsights.com;
```

## GDPR Consent

```typescript
// Load conditionally based on consent
if (localStorage.getItem('analytics-consent') === 'true') {
  const script = document.createElement('script');
  script.src = 'https://static.cloudflareinsights.com/beacon.min.js';
  script.defer = true;
  script.setAttribute('data-cf-beacon', '{"token": "YOUR_TOKEN", "spa": true}');
  document.body.appendChild(script);
}
```