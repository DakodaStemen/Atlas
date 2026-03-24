---
name: cloudflare-ai-services
description: Cloudflare AI platform services including Vectorize (vector database), AI Search (AutoRAG), AI Gateway (proxy/caching/rate-limiting), email routing Workers, and Durable Object storage patterns. Use when building AI features on Cloudflare Workers platform.
domain: ai-engineering
tags: [cloudflare, vectorize, ai-search, autorag, ai-gateway, email-routing, durable-objects, workers-ai]
triggers: cloudflare vectorize, AI Search, AutoRAG, AI Gateway, email routing, durable objects, workers AI, cloudflare ai
---

# Cloudflare AI Services

## 1. Vectorize (Vector Database)

### Setup

```bash
npx wrangler vectorize create my-index --dimensions=768 --metric=cosine
```

Dimensions and metric are immutable after creation.

```jsonc
// wrangler.jsonc
{ "vectorize": [{ "binding": "VECTORIZE", "index_name": "my-index" }] }
```

### Metadata Indexes

Must create BEFORE inserting vectors (existing vectors not retroactively indexed):

```bash
wrangler vectorize create-metadata-index my-index --property-name=category --type=string
wrangler vectorize create-metadata-index my-index --property-name=price --type=number
```

### Workers AI Integration

```typescript
const result = await env.AI.run("@cf/baai/bge-base-en-v1.5", { text: [query] });
const matches = await env.VECTORIZE.query(result.data[0], { topK: 5 });
```

| Model | Dimensions |
|-------|-----------|
| `@cf/baai/bge-small-en-v1.5` | 384 |
| `@cf/baai/bge-base-en-v1.5` | 768 (recommended) |
| `@cf/baai/bge-large-en-v1.5` | 1024 |

### RAG Pattern

```typescript
const emb = await env.AI.run("@cf/baai/bge-base-en-v1.5", { text: [query] });
const matches = await env.VECTORIZE.query(emb.data[0], { topK: 5, returnMetadata: "indexed" });
const docs = await Promise.all(matches.matches.map(m => env.R2.get(m.metadata.key).then(o => o?.text())));
const answer = await env.AI.run("@cf/meta/llama-3-8b-instruct", {
  prompt: `Context:\n${docs.filter(Boolean).join("\n\n")}\n\nQuestion: ${query}`
});
```

### Multi-Tenant

- **Namespaces** (<50K tenants): `{ namespace: "tenant-123" }` on upsert/query.
- **Metadata filter** (>50K tenants): Create metadata index on tenantId, filter on query.

### Best Practices

1. Pass `data[0]` not `data` or full response. 2. Batch 500 vectors per upsert. 3. Create metadata indexes before inserting. 4. Use `returnMetadata: "indexed"` for best speed. 5. Handle 5-10s mutation delay in async operations.

## 2. AI Search (AutoRAG)

### Configuration

```jsonc
{ "ai": { "binding": "AI" } }
```

```typescript
const answer = await env.AI.autorag("my-instance").aiSearch({
  query: "How do I configure caching?",
  model: "@cf/meta/llama-3.3-70b-instruct-fp8-fast"
});
```

### search() vs aiSearch()

| Method | Returns | Latency | Use Case |
|--------|---------|---------|----------|
| `search()` | Raw chunks only | ~100-300ms | Custom UI, analytics |
| `aiSearch()` | AI response + chunks | ~500-2000ms | Chatbots, Q&A |

### Score Thresholds

0.3 (broad recall), 0.5 (balanced, production default), 0.7 (high precision, critical accuracy).

### Multitenancy (Folder-Based)

```typescript
filters: { column: "folder", operator: "gte", value: `tenants/${tenantId}/` }
```

### Data Sources

- **R2 Bucket**: Supports .md, .txt, .html, .pdf, .doc, .docx, .csv, .json. Auto-indexed metadata: filename, folder, timestamp.
- **Website Crawler**: Requires domain on Cloudflare, sitemap.xml, allow CloudflareAISearch user agent.
- **Indexing**: Automatic every 6 hours. Force sync via dashboard (30s rate limit).

## 3. AI Gateway

### SDK Integration

```typescript
import { createAiGateway } from 'ai-gateway-provider';
import { createOpenAI } from '@ai-sdk/openai';
const gateway = createAiGateway({ accountId: "...", gatewayId: "my-gateway" });
const openai = createOpenAI({ ...gateway.openai() });
```

### Features

- **Caching**: Cache LLM responses by input hash. Reduces cost and latency for repeated queries.
- **Rate limiting**: Protect upstream providers from burst traffic. Configure per-gateway limits.
- **Logging**: Full request/response logging for debugging and evaluation.
- **Fallback**: Route to backup provider when primary fails.
- **Analytics**: Token usage, latency, cache hit rates per gateway.

### Troubleshooting

- 429 errors: Check rate limit configuration, implement client-side retry with backoff.
- Cache misses: Verify caching is enabled, check that requests are identical (same params).
- High latency: Check provider health, consider enabling caching for common queries.

## 4. Email Routing Workers

### Basic Setup

```jsonc
{ "name": "email-worker", "main": "src/index.ts", "send_email": [{ "name": "EMAIL" }] }
```

```typescript
export default {
  async email(message, env, ctx) {
    await message.forward("destination@example.com");
  }
} satisfies ExportedHandler;
```

### DNS (Auto-Created)

```dns
yourdomain.com. IN MX 1 isaac.mx.cloudflare.net.
yourdomain.com. IN TXT "v=spf1 include:_spf.mx.cloudflare.net ~all"
```

## 5. Durable Object Storage Patterns

### Schema Migration

```typescript
export class MyDurableObject extends DurableObject {
  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    const ver = this.sql.exec("PRAGMA user_version").one()?.user_version || 0;
    if (ver === 0) {
      this.sql.exec(`CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT)`);
      this.sql.exec("PRAGMA user_version = 1");
    }
    if (ver === 1) {
      this.sql.exec(`ALTER TABLE users ADD COLUMN email TEXT`);
      this.sql.exec("PRAGMA user_version = 2");
    }
  }
}
```

### In-Memory Caching

Use a `Map` as write-through cache in front of Durable Object storage. Read from map first, write to both map and storage.

### Rate Limiting

Use Durable Object with SQLite to track requests per key within a sliding window. Delete expired entries on each check.

### Alarms

Schedule future execution with `ctx.storage.setAlarm(Date.now() + delay)`. Handle in `alarm()` method. Self-rescheduling for recurring tasks.
