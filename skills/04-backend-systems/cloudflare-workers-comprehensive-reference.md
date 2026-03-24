---
name: Cloudflare Workers Comprehensive Reference
description: High-density reference for Cloudflare Workers development. Covers runtime (ES modules, wrangler.jsonc), storage (KV, D1, R2, Durable Objects), messaging (Queues, Crons, Workflows), specialized workers (Email, Tail, Snippets), AI (Workers AI, Vectorize), networking (TCP, TURN), and observability. Use this as the primary technical cue for any Cloudflare platform task.
---

# Cloudflare Workers Comprehensive Reference

## 1. Core Runtime & Configuration

### Handler Patterns (ES Modules)
```typescript
export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    // 1. FRESH BINDINGS: Always access env per-request. NEVER cache env in global scope.
    const value = await env.MY_KV.get("key");
    
    // 2. ASYNC TASKS: Use ctx.waitUntil for background work (logs, analytics).
    ctx.waitUntil(logToDB(request)); 
    
    return new Response("OK");
  },
  async scheduled(controller: ScheduledController, env: Env, ctx: ExecutionContext) { /* Cron */ },
  async queue(batch: MessageBatch, env: Env, ctx: ExecutionContext) { /* Queues */ },
  async email(message: ForwardableEmailMessage, env: Env, ctx: ExecutionContext) { /* Email */ }
};
```

### Configuration (wrangler.jsonc)
- **compatibility_date:** MUST set to current date for new features (e.g., RPC, D1 Sessions).
- **compatibility_flags:** Use `nodejs_compat_v2` for Node.js built-ins (Buffer, crypto).
- **Smart Placement:** `{ "placement": { "mode": "smart" } }` optimizes compute near data sources. **Gotcha:** Slows static assets; split into separate Workers.

### TypeScript Type Generation
```bash
npx wrangler types # Generates .wrangler/types/runtime.d.ts from config
```
Add to `tsconfig.json` `include` and `compilerOptions.types`.

---

## 2. Storage & State

### KV (Key-Value)
- **Speed:** Read-optimized (low latency, high volume). Stale reads possible (60s propagation).
- **Operations:** `get(key, {type: "json"})`, `put(key, val, {expirationTtl: 3600})`, `delete(key)`, `list({prefix: "user:"})`.
- **Bulk:** `env.KV.get(["k1", "k2"])` (max 100 keys) is 1 operation.
- **Limits:** 25MB value size, 1 write/sec per key.

### D1 (SQL Database)
- **Querying:** `prepare(sql).bind(params).all()`, `.first()`, `.run()` (write), `.raw()` (no objects).
- **Transactions:** `env.DB.batch([stmt1, stmt2])` executes as atomic transaction.
- **Consistency:** Use **D1 Sessions** (`env.DB.withSession()`) for guaranteed read-after-write consistency.
- **Sessions API:** Use for long-running migrations (up to 15 min).

### R2 (Object Storage)
- **S3 Compatibility:** Use `region: "auto"` in S3 SDK.
- **Direct Put:** `env.R2.put(key, body, { httpMetadata: { contentType: "..." } })`.
- **Multipart:** For files >100MB. `createMultipartUpload` → `uploadPart` → `complete`.
- **Gotcha:** `etag` is unquoted; use `httpEtag` for response headers. R2 requires `Content-Length` for streams.

### Durable Objects (Stateful)
- **Model:** Single-threaded, persistent memory, SQLite storage.
- **Gates (Concurrency):** Input gates block new requests during storage reads. `fetch()` breaks gates! Use `blockConcurrencyWhile()` for critical sections.
- **RPC (2024+):** Call methods directly `await stub.increment()`. Faster and type-safe vs `stub.fetch()`.
- **Hibernation:** DO with WebSockets hibernates when idle. Use `ws.serializeAttachment()` for metadata that survives wake.
- **Alarms:** 1 per DO. Schedule background tasks `this.ctx.storage.setAlarm(Date.now() + 60000)`.

---

## 3. Messaging & Events

### Queues
- **Top Mistake:** Uncaught error in handler retries the **ENTIRE BATCH**. Always try/catch per message and call `msg.retry()` explicitly.
- **Idempotency:** Duplicates possible (at-least-once). Track `msg.id` in KV with TTL.
- **Consumer:** `msg.ack()` or `msg.retry({delaySeconds: 60})`. Unhandled messages auto-retry.

### Workflows (Orchestration)
- **Steps:** `step.do('name', async () => {})`, `step.sleep('wait', '1h')`, `step.waitForEvent('approval')`.
- **Reliability:** Automatic retries, state persistence, resume from failure.
- **Trigger:** `env.MY_WORKFLOW.create({id: 'unique', params: {}})`. Idempotent (skips existing IDs).

---

## 4. Specialized Workers

### Email Workers
- **Stream Gotcha:** `message.raw` is single-use. Buffer first: `await new Response(message.raw).arrayBuffer()`.
- **Handlers:** `message.forward('dest@ex.com')`, `message.setReject('spam')`, `message.reply(EmailMessage)`.
- **Security:** Use **envelope** `message.from` for security, not header From (spoofable).

### Tail Workers (Observability)
- **Role:** Processes `TraceItem[]` from producers. 100 events/batch.
- **Critical:** Tail handlers return `void`. Use `ctx.waitUntil()` for external logging calls.
- **Outcome:** `event.outcome` is script status ('ok', 'exception', 'exceededCpu'), NOT HTTP status.

---

## 5. Networking & Security

### TCP Sockets (`cloudflare:sockets`)
- **API:** `connect({ hostname, port }, { secureTransport: "on" | "starttls" })`.
- **Streams:** Use `socket.readable.getReader()` and `socket.writable.getWriter()`.
- **TLS:** Use `socket.startTls()` after handshake for `starttls` mode.

### TURN (WebRTC)
- **Credentials:** Generate server-side ONLY. `https://rtc.live.cloudflare.com/v1/turn/keys/{id}/credentials/generate`.
- **TTL Limit:** Maximum 48 hours (172800s). Browser clients: Filter port 53 (blocked).
- **ICE Restart:** Required if connection fails or credentials expire during long calls.

### Rulesets & WAF
- **Phases:** `http_request_firewall_custom` (Custom) → `http_request_firewall_managed` (Managed) → `http_ratelimit` (Rate Limit).
- **Expression:** `http.request.uri.path contains "/api" and ip.geoip.country eq "US"`.

---

## 6. AI & Media

### Workers AI
- **Method:** `env.AI.run(model, input)`.
- **Patterns:** Streaming SSE for chat, batch embeddings for RAG efficiency.
- **Gotchas:** `embedding.data[0]` extracts vector. Use `wrangler dev --remote` (no local inference).

### Vectorize
- **Batching:** Undocumented limit of **500 vectors per upsert**. Chunk larger sets.
- **Filtering:** Create metadata index BEFORE inserting. Existing vectors won't be indexed retroactively.
- **Latency:** Vectors queryable 5-10s after upsert.

### Analytics Engine
- **Writing:** `env.ANALYTICS.writeDataPoint({ blobs, doubles, indexes })`. Fire-and-forget (void).
- **Indexing:** Use `indexes` for high-cardinality (millions of IDs), `blobs` for dimensions (hundreds of statuses).

---

## 7. Performance & Optimization

### Top Performance Rules
1. **Parallelize:** Use `Promise.all([kv.get(), db.prepare().all()])` instead of sequential awaits.
2. **Waterfalls:** Fetch data in parallel with map/UI initialization.
3. **Clean Removal:** In SPAs, ALWAYS call `map.remove()` or `dispose()` on component unmount.
4. **Subrequests:** Max 1000 per request. Batch individual calls into single API requests.

### Limits
- **CPU Time:** 10ms (Free/Standard), 30ms (Unbound), 300s (Consumer max via config).
- **Memory:** 128MB per Worker/DO.
- **Environment:** 5MB total binding size.

### Global Scope Caution (The #1 Gotcha)
```typescript
// ❌ WRONG: env not available here, or stale
const key = env.API_KEY; 

export default {
  async fetch(req, env) {
    // ✅ CORRECT: access here
    const key = env.API_KEY; 
  }
}
```

## Resources
- [Limits Reference](https://developers.cloudflare.com/workers/platform/limits/)
- [Compatibility Dates](https://developers.cloudflare.com/workers/configuration/compatibility-dates/)
- [Wrangler Commands](https://developers.cloudflare.com/workers/wrangler/commands/)
