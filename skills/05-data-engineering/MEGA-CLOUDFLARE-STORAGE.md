---
name: D1 Configuration
description: # D1 Configuration
 
 ## wrangler.jsonc Setup
---

# D1 Configuration

## wrangler.jsonc Setup (D1 Configuration)

```jsonc
{
  "name": "your-worker-name",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01", // Use current date for new projects
  "d1_databases": [
    {
      "binding": "DB",                    // Env variable name
      "database_name": "your-db-name",    // Human-readable name
      "database_id": "your-database-id",  // UUID from dashboard/CLI
      "migrations_dir": "migrations"      // Optional: default is "migrations"
    },
    // Read replica (paid plans only)
    {
      "binding": "DB_REPLICA",
      "database_name": "your-db-name",
      "database_id": "your-database-id"   // Same ID, different binding
    },
    // Multiple databases
    {
      "binding": "ANALYTICS_DB",
      "database_name": "analytics-db",
      "database_id": "yyy-yyy-yyy"
    }
  ]
}
```

## TypeScript Types

```typescript
interface Env { DB: D1Database; ANALYTICS_DB?: D1Database; }

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const result = await env.DB.prepare('SELECT * FROM users').all();
    return Response.json(result.results);
  }
}
```

## Migrations

File structure: `migrations/0001_initial_schema.sql`, `0002_add_posts.sql`, etc.

### Example Migration

```sql
-- migrations/0001_initial_schema.sql
CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email TEXT UNIQUE NOT NULL,
  name TEXT NOT NULL,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);

CREATE TABLE IF NOT EXISTS posts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id INTEGER NOT NULL,
  title TEXT NOT NULL,
  content TEXT,
  published BOOLEAN DEFAULT 0,
  created_at TEXT DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_posts_user_id ON posts(user_id);
CREATE INDEX idx_posts_published ON posts(published);
```

### Running Migrations

```bash
# Create new migration file
wrangler d1 migrations create <db-name> add_users_table
# Creates: migrations/0001_add_users_table.sql

# Apply migrations
wrangler d1 migrations apply <db-name> --local     # Apply to local DB
wrangler d1 migrations apply <db-name> --remote    # Apply to production DB

# List applied migrations
wrangler d1 migrations list <db-name> --remote

# Direct SQL execution (bypasses migration tracking)
wrangler d1 execute <db-name> --remote --command="SELECT * FROM users"
wrangler d1 execute <db-name> --local --file=./schema.sql
```

**Migration tracking**: Wrangler creates `d1_migrations` table automatically to track applied migrations

## Indexing Strategy

```sql
-- Index frequently queried columns
CREATE INDEX idx_users_email ON users(email);

-- Composite indexes for multi-column queries
CREATE INDEX idx_posts_user_published ON posts(user_id, published);

-- Covering indexes (include queried columns)
CREATE INDEX idx_users_email_name ON users(email, name);

-- Partial indexes for filtered queries
CREATE INDEX idx_active_users ON users(email) WHERE active = 1;

-- Check if query uses index
EXPLAIN QUERY PLAN SELECT * FROM users WHERE email = ?;
```

## Drizzle ORM

```typescript
// drizzle.config.ts
export default {
  schema: './src/schema.ts', out: './migrations', dialect: 'sqlite', driver: 'd1-http',
  dbCredentials: { accountId: process.env.CLOUDFLARE_ACCOUNT_ID!, databaseId: process.env.D1_DATABASE_ID!, token: process.env.CLOUDFLARE_API_TOKEN! }
} satisfies Config;

// schema.ts
import { sqliteTable, text, integer } from 'drizzle-orm/sqlite-core';
export const users = sqliteTable('users', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  email: text('email').notNull().unique(),
  name: text('name').notNull()
});

// worker.ts
import { drizzle } from 'drizzle-orm/d1';
import { users } from './schema';
export default {
  async fetch(request: Request, env: Env) {
    const db = drizzle(env.DB);
    return Response.json(await db.select().from(users));
  }
}
```

## Import & Export

```bash
# Export full database (schema + data)
wrangler d1 export <db-name> --remote --output=./backup.sql

# Export data only (no schema)
wrangler d1 export <db-name> --remote --no-schema --output=./data-only.sql

# Export with foreign key constraints preserved
# (Default: foreign keys are disabled during export for import compatibility)

# Import SQL file
wrangler d1 execute <db-name> --remote --file=./backup.sql

# Limitations
# - BLOB data may not export correctly (use R2 for binary files)
# - Very large exports (>1GB) may timeout (split into chunks)
# - Import is NOT atomic (use batch() for transactional imports in Workers)
```

## Plan Tiers

| Feature | Free | Paid |
| --------- | ------ | ------ |
| Database size | 500 MB | 10 GB |
| Batch size | 1,000 statements | 10,000 statements |
| Time Travel | 7 days | 30 days |
| Read replicas | ❌ | ✅ |
| Sessions API | ❌ | ✅ (up to 15 min) |
| Pricing | Free | $5/mo + usage |

**Usage pricing** (paid plans): $0.001 per 1K reads + $1 per 1M writes + $0.75/GB storage/month

## Local Development

```bash
wrangler dev --persist-to=./.wrangler/state  # Persist across restarts
# Local DB: .wrangler/state/v3/d1/<database-id>.sqlite
sqlite3 .wrangler/state/v3/d1/<database-id>.sqlite  # Inspect

# Local dev uses free tier limits by default
```

## When to use

Use when the user asks about or needs: D1 Configuration.
﻿---
name: D1 Patterns & Best Practices
description: # D1 Patterns & Best Practices
 
 ## Pagination
---

# D1 Patterns & Best Practices

## Pagination (D1 Patterns & Best Practices)

```typescript
async function getUsers({ page, pageSize }: { page: number; pageSize: number }, env: Env) {
  const offset = (page - 1) * pageSize;
  const [countResult, dataResult] = await env.DB.batch([
    env.DB.prepare('SELECT COUNT(*) as total FROM users'),
    env.DB.prepare('SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?').bind(pageSize, offset)
  ]);
  return { data: dataResult.results, total: countResult.results[0].total, page, pageSize, totalPages: Math.ceil(countResult.results[0].total / pageSize) };
}
```

## Conditional Queries

```typescript
async function searchUsers(filters: { name?: string; email?: string; active?: boolean }, env: Env) {
  const conditions: string[] = [], params: (string | number | boolean | null)[] = [];
  if (filters.name) { conditions.push('name LIKE ?'); params.push(`%${filters.name}%`); }
  if (filters.email) { conditions.push('email = ?'); params.push(filters.email); }
  if (filters.active !== undefined) { conditions.push('active = ?'); params.push(filters.active ? 1 : 0); }
  const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
  return await env.DB.prepare(`SELECT * FROM users ${whereClause}`).bind(...params).all();
}
```

## Bulk Insert

```typescript
async function bulkInsertUsers(users: Array<{ name: string; email: string }>, env: Env) {
  const stmt = env.DB.prepare('INSERT INTO users (name, email) VALUES (?, ?)');
  const batch = users.map(user => stmt.bind(user.name, user.email));
  return await env.DB.batch(batch);
}
```

## Caching with KV

```typescript
async function getCachedUser(userId: number, env: { DB: D1Database; CACHE: KVNamespace }) {
  const cacheKey = `user:${userId}`;
  const cached = await env.CACHE?.get(cacheKey, 'json');
  if (cached) return cached;
  const user = await env.DB.prepare('SELECT * FROM users WHERE id = ?').bind(userId).first();
  if (user) await env.CACHE?.put(cacheKey, JSON.stringify(user), { expirationTtl: 300 });
  return user;
}
```

## Query Optimization

```typescript
// ✅ Use indexes in WHERE clauses
const users = await env.DB.prepare('SELECT * FROM users WHERE email = ?').bind(email).all();

// ✅ Limit result sets
const recentPosts = await env.DB.prepare('SELECT * FROM posts ORDER BY created_at DESC LIMIT 100').all();

// ✅ Use batch() for multiple independent queries
const [user, posts, comments] = await env.DB.batch([
  env.DB.prepare('SELECT * FROM users WHERE id = ?').bind(userId),
  env.DB.prepare('SELECT * FROM posts WHERE user_id = ?').bind(userId),
  env.DB.prepare('SELECT * FROM comments WHERE user_id = ?').bind(userId)
]);

// ❌ Avoid N+1 queries
for (const post of posts) {
  const author = await env.DB.prepare('SELECT * FROM users WHERE id = ?').bind(post.user_id).first(); // Bad: multiple round trips
}

// ✅ Use JOINs instead
const postsWithAuthors = await env.DB.prepare(`
  SELECT posts.*, users.name as author_name
  FROM posts
  JOIN users ON posts.user_id = users.id
`).all();
```

## Multi-Tenant SaaS

```typescript
// Each tenant gets own database
export default {
  async fetch(request: Request, env: { [key: `TENANT_${string}`]: D1Database }) {
    const tenantId = request.headers.get('X-Tenant-ID');
    const data = await env[`TENANT_${tenantId}`].prepare('SELECT * FROM records').all();
    return Response.json(data.results);
  }
}
```

## Session Storage

```typescript
async function createSession(userId: number, token: string, env: Env) {
  const expiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString();
  return await env.DB.prepare('INSERT INTO sessions (user_id, token, expires_at) VALUES (?, ?, ?)').bind(userId, token, expiresAt).run();
}

async function validateSession(token: string, env: Env) {
  return await env.DB.prepare('SELECT s.*, u.email FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.token = ? AND s.expires_at > CURRENT_TIMESTAMP').bind(token).first();
}
```

## Analytics/Events

```typescript
async function logEvent(event: { type: string; userId?: number; metadata: object }, env: Env) {
  return await env.DB.prepare('INSERT INTO events (type, user_id, metadata) VALUES (?, ?, ?)').bind(event.type, event.userId || null, JSON.stringify(event.metadata)).run();
}

async function getEventStats(startDate: string, endDate: string, env: Env) {
  return await env.DB.prepare('SELECT type, COUNT(*) as count FROM events WHERE timestamp BETWEEN ? AND ? GROUP BY type ORDER BY count DESC').bind(startDate, endDate).all();
}
```

## Read Replication Pattern (Paid Plans)

```typescript
interface Env { DB: D1Database; DB_REPLICA: D1Database; }

export default {
  async fetch(request: Request, env: Env) {
    if (request.method === 'GET') {
      // Reads: use replica for lower latency
      const users = await env.DB_REPLICA.prepare('SELECT * FROM users WHERE active = 1').all();
      return Response.json(users.results);
    }
    
    if (request.method === 'POST') {
      const { name, email } = await request.json();
      const result = await env.DB.prepare('INSERT INTO users (name, email) VALUES (?, ?)').bind(name, email).run();
      
      // Read-after-write: use primary for consistency (replication lag <100ms-2s)
      const user = await env.DB.prepare('SELECT * FROM users WHERE id = ?').bind(result.meta.last_row_id).first();
      return Response.json(user, { status: 201 });
    }
  }
}
```

**Use replicas for**: Analytics dashboards, search results, public queries (eventual consistency OK)  
**Use primary for**: Read-after-write, financial transactions, authentication (consistency required)

## Sessions API Pattern (Paid Plans)

```typescript
// Migration with long-running session (up to 15 min)
async function runMigration(env: Env) {
  const session = env.DB.withSession({ timeout: 600 }); // 10 min
  try {
    await session.prepare('CREATE INDEX idx_users_email ON users(email)').run();
    await session.prepare('CREATE INDEX idx_posts_user ON posts(user_id)').run();
    await session.prepare('ANALYZE').run();
  } finally {
    session.close(); // Always close to prevent leaks
  }
}

// Bulk transformation with batching
async function transformLargeDataset(env: Env) {
  const session = env.DB.withSession({ timeout: 900 }); // 15 min max
  try {
    const BATCH_SIZE = 1000;
    let offset = 0;
    while (true) {
      const rows = await session.prepare('SELECT id, data FROM legacy LIMIT ? OFFSET ?').bind(BATCH_SIZE, offset).all();
      if (rows.results.length === 0) break;
      const updates = rows.results.map(row => 
        session.prepare('UPDATE legacy SET new_data = ? WHERE id = ?').bind(transform(row.data), row.id)
      );
      await session.batch(updates);
      offset += BATCH_SIZE;
    }
  } finally { session.close(); }
}
```

## Time Travel & Backups

```bash
wrangler d1 time-travel restore <db-name> --timestamp="2024-01-15T14:30:00Z"  # Point-in-time
wrangler d1 time-travel info <db-name>  # List restore points (7 days free, 30 days paid)
wrangler d1 export <db-name> --remote --output=./backup.sql  # Full export
wrangler d1 export <db-name> --remote --no-schema --output=./data.sql  # Data only
wrangler d1 execute <db-name> --remote --file=./backup.sql  # Import
```

## When to use

Use when the user asks about or needs: D1 Patterns & Best Practices.
﻿---
name: R2 Configuration
description: # R2 Configuration
 
 ## Workers Binding
---

# R2 Configuration

## Workers Binding (R2 Configuration)

### wrangler.jsonc

```jsonc
{
  "r2_buckets": [
    {
      "binding": "MY_BUCKET",
      "bucket_name": "my-bucket-name"
    }
  ]
}
```

## TypeScript Types

```typescript
interface Env { MY_BUCKET: R2Bucket; }

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const object = await env.MY_BUCKET.get('file.txt');
    return new Response(object?.body);
  }
}
```

## S3 SDK Setup

```typescript
import { S3Client, PutObjectCommand } from '@aws-sdk/client-s3';

const s3 = new S3Client({
  region: 'auto',
  endpoint: `https://${accountId}.r2.cloudflarestorage.com`,
  credentials: {
    accessKeyId: env.R2_ACCESS_KEY_ID,
    secretAccessKey: env.R2_SECRET_ACCESS_KEY
  }
});

await s3.send(new PutObjectCommand({
  Bucket: 'my-bucket',
  Key: 'file.txt',
  Body: data,
  StorageClass: 'STANDARD' // or 'STANDARD_IA'
}));
```

## Location Hints

```bash
wrangler r2 bucket create my-bucket --location=enam

# Hints: wnam, enam, weur, eeur, apac, oc
# Jurisdictions (override hint): --jurisdiction=eu (or fedramp)
```

## CORS Configuration

CORS must be configured via S3 SDK or dashboard (not available in Workers API):

```typescript
import { S3Client, PutBucketCorsCommand } from '@aws-sdk/client-s3';

const s3 = new S3Client({
  region: 'auto',
  endpoint: `https://${accountId}.r2.cloudflarestorage.com`,
  credentials: {
    accessKeyId: env.R2_ACCESS_KEY_ID,
    secretAccessKey: env.R2_SECRET_ACCESS_KEY
  }
});

await s3.send(new PutBucketCorsCommand({
  Bucket: 'my-bucket',
  CORSConfiguration: {
    CORSRules: [{
      AllowedOrigins: ['https://example.com'],
      AllowedMethods: ['GET', 'PUT', 'HEAD'],
      AllowedHeaders: ['*'],
      ExposeHeaders: ['ETag'],
      MaxAgeSeconds: 3600
    }]
  }
}));
```

## Object Lifecycles

```typescript
import { PutBucketLifecycleConfigurationCommand } from '@aws-sdk/client-s3';

await s3.send(new PutBucketLifecycleConfigurationCommand({
  Bucket: 'my-bucket',
  LifecycleConfiguration: {
    Rules: [
      {
        ID: 'expire-old-logs',
        Status: 'Enabled',
        Prefix: 'logs/',
        Expiration: { Days: 90 }
      },
      {
        ID: 'transition-to-ia',
        Status: 'Enabled',
        Prefix: 'archives/',
        Transitions: [{ Days: 30, StorageClass: 'STANDARD_IA' }]
      }
    ]
  }
}));
```

## API Token Scopes

When creating R2 tokens, set minimal permissions:

| Permission | Use Case |
| ------------ | ---------- |
| Object Read | Public serving, downloads |
| Object Write | Uploads only |
| Object Read & Write | Full object operations |
| Admin Read & Write | Bucket management, CORS, lifecycles |

**Best practice:** Separate tokens for Workers (read/write) vs admin tasks (CORS, lifecycles).

## Event Notifications

```jsonc
// wrangler.jsonc
{
  "r2_buckets": [
    {
      "binding": "MY_BUCKET",
      "bucket_name": "my-bucket",
      "event_notifications": [
        {
          "queue": "r2-events",
          "actions": ["PutObject", "DeleteObject", "CompleteMultipartUpload"]
        }
      ]
    }
  ],
  "queues": {
    "producers": [{ "binding": "R2_EVENTS", "queue": "r2-events" }],
    "consumers": [{ "queue": "r2-events", "max_batch_size": 10 }]
  }
}
```

## Bucket Management

```bash
wrangler r2 bucket create my-bucket --location=enam --storage-class=Standard
wrangler r2 bucket list
wrangler r2 bucket info my-bucket
wrangler r2 bucket delete my-bucket  # Must be empty
wrangler r2 bucket update-storage-class my-bucket --storage-class=InfrequentAccess

# Public bucket via dashboard
wrangler r2 bucket domain add my-bucket --domain=files.example.com
```

## When to use

Use when the user asks about or needs: R2 Configuration.
﻿---
name: R2 Patterns & Best Practices
description: # R2 Patterns & Best Practices
 
 ## Streaming Large Files
---

# R2 Patterns & Best Practices

## Streaming Large Files (R2 Patterns & Best Practices)

```typescript
const object = await env.MY_BUCKET.get(key);
if (!object) return new Response('Not found', { status: 404 });

const headers = new Headers();
object.writeHttpMetadata(headers);
headers.set('etag', object.httpEtag);

return new Response(object.body, { headers });
```

## Conditional GET (304 Not Modified)

```typescript
const ifNoneMatch = request.headers.get('if-none-match');
const object = await env.MY_BUCKET.get(key, {
  onlyIf: { etagDoesNotMatch: ifNoneMatch?.replace(/"/g, '') || '' }
});

if (!object) return new Response('Not found', { status: 404 });
if (!object.body) return new Response(null, { status: 304, headers: { 'etag': object.httpEtag } });

return new Response(object.body, { headers: { 'etag': object.httpEtag } });
```

## Upload with Validation

```typescript
const key = url.pathname.slice(1);
if (!key || key.includes('..')) return new Response('Invalid key', { status: 400 });

const object = await env.MY_BUCKET.put(key, request.body, {
  httpMetadata: { contentType: request.headers.get('content-type') || 'application/octet-stream' },
  customMetadata: { uploadedAt: new Date().toISOString(), ip: request.headers.get('cf-connecting-ip') || 'unknown' }
});

return Response.json({ key: object.key, size: object.size, etag: object.httpEtag });
```

## Multipart with Progress

```typescript
const PART_SIZE = 5 * 1024 * 1024; // 5MB
const partCount = Math.ceil(file.size / PART_SIZE);
const multipart = await env.MY_BUCKET.createMultipartUpload(key, { httpMetadata: { contentType: file.type } });

const uploadedParts: R2UploadedPart[] = [];
try {
  for (let i = 0; i < partCount; i++) {
    const start = i * PART_SIZE;
    const part = await multipart.uploadPart(i + 1, file.slice(start, start + PART_SIZE));
    uploadedParts.push(part);
    onProgress?.(Math.round(((i + 1) / partCount) * 100));
  }
  return await multipart.complete(uploadedParts);
} catch (error) {
  await multipart.abort();
  throw error;
}
```

## Batch Delete

```typescript
async function deletePrefix(prefix: string, env: Env) {
  let cursor: string | undefined;
  let truncated = true;

  while (truncated) {
    const listed = await env.MY_BUCKET.list({ prefix, limit: 1000, cursor });
    if (listed.objects.length > 0) {
      await env.MY_BUCKET.delete(listed.objects.map(o => o.key));
    }
    truncated = listed.truncated;
    cursor = listed.cursor;
  }
}
```

## Checksum Validation & Storage Transitions

```typescript
// Upload with checksum
const hash = await crypto.subtle.digest('SHA-256', data);
await env.MY_BUCKET.put(key, data, { sha256: hash });

// Transition storage class (requires S3 SDK)
import { S3Client, CopyObjectCommand } from '@aws-sdk/client-s3';
await s3.send(new CopyObjectCommand({
  Bucket: 'my-bucket', Key: key,
  CopySource: `/my-bucket/${key}`,
  StorageClass: 'STANDARD_IA'
}));
```

## Client-Side Uploads (Presigned URLs)

```typescript
import { S3Client } from '@aws-sdk/client-s3';
import { getSignedUrl } from '@aws-sdk/s3-request-presigner';
import { PutObjectCommand } from '@aws-sdk/client-s3';

// Worker: Generate presigned upload URL
const s3 = new S3Client({
  region: 'auto',
  endpoint: `https://${env.ACCOUNT_ID}.r2.cloudflarestorage.com`,
  credentials: { accessKeyId: env.R2_ACCESS_KEY_ID, secretAccessKey: env.R2_SECRET_ACCESS_KEY }
});

const url = await getSignedUrl(s3, new PutObjectCommand({ Bucket: 'my-bucket', Key: key }), { expiresIn: 3600 });
return Response.json({ uploadUrl: url });

// Client: Upload directly
const { uploadUrl } = await fetch('/api/upload-url').then(r => r.json());
await fetch(uploadUrl, { method: 'PUT', body: file });
```

## Caching with Cache API

```typescript
export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const cache = caches.default;
    const url = new URL(request.url);
    const cacheKey = new Request(url.toString(), request);

    // Check cache first
    let response = await cache.match(cacheKey);
    if (response) return response;

    // Fetch from R2
    const key = url.pathname.slice(1);
    const object = await env.MY_BUCKET.get(key);
    if (!object) return new Response('Not found', { status: 404 });

    const headers = new Headers();
    object.writeHttpMetadata(headers);
    headers.set('etag', object.httpEtag);
    headers.set('cache-control', 'public, max-age=31536000, immutable');

    response = new Response(object.body, { headers });

    // Cache for subsequent requests
    ctx.waitUntil(cache.put(cacheKey, response.clone()));

    return response;
  }
};
```

## Public Bucket with Custom Domain

```typescript
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // CORS preflight
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        headers: {
          'access-control-allow-origin': '*',
          'access-control-allow-methods': 'GET, HEAD',
          'access-control-max-age': '86400'
        }
      });
    }

    const key = new URL(request.url).pathname.slice(1);
    if (!key) return Response.redirect('/index.html', 302);

    const object = await env.MY_BUCKET.get(key);
    if (!object) return new Response('Not found', { status: 404 });

    const headers = new Headers();
    object.writeHttpMetadata(headers);
    headers.set('etag', object.httpEtag);
    headers.set('access-control-allow-origin', '*');
    headers.set('cache-control', 'public, max-age=31536000, immutable');

    return new Response(object.body, { headers });
  }
};
```

## r2.dev Public URLs

Enable r2.dev in dashboard for simple public access: `https://pub-${hashId}.r2.dev/${key}`  
Or add custom domain via dashboard: `https://files.example.com/${key}`

**Limitations:** No auth, bucket-level CORS, no cache override.

## When to use

Use when the user asks about or needs: R2 Patterns & Best Practices.
﻿---
name: R2 SQL API Reference
description: # R2 SQL API Reference
 
 SQL syntax, functions, operators, and data types for R2 SQL queries.
---

# R2 SQL API Reference

SQL syntax, functions, operators, and data types for R2 SQL queries.

## SQL Syntax

```sql
SELECT column_list | aggregation_function
FROM [namespace.]table_name
WHERE conditions
[GROUP BY column_list]
[HAVING conditions]
[ORDER BY column | aggregation_function [DESC | ASC]]
[LIMIT number]
```

## Schema Discovery

```sql
SHOW DATABASES;           -- List namespaces
SHOW NAMESPACES;          -- Alias for SHOW DATABASES
SHOW SCHEMAS;             -- Alias for SHOW DATABASES
SHOW TABLES IN namespace; -- List tables in namespace
DESCRIBE namespace.table; -- Show table schema, partition keys
```

## SELECT Clause

```sql
-- All columns
SELECT * FROM logs.http_requests;

-- Specific columns
SELECT user_id, timestamp, status FROM logs.http_requests;
```

**Limitations:** No column aliases, expressions, or nested column access

## WHERE Clause

### Operators

| Operator | Example |
| ---------- | --------- |
| `=`, `!=`, `<`, `<=`, `>`, `>=` | `status = 200` |
| `LIKE` | `user_agent LIKE '%Chrome%'` |
| `BETWEEN` | `timestamp BETWEEN '2025-01-01T00:00:00Z' AND '2025-01-31T23:59:59Z'` |
| `IS NULL`, `IS NOT NULL` | `email IS NOT NULL` |
| `AND`, `OR` | `status = 200 AND method = 'GET'` |

Use parentheses for precedence: `(status = 404 OR status = 500) AND method = 'POST'`

## Aggregation Functions

| Function | Description |
| ---------- | ------------- |
| `COUNT(*)` | Count all rows |
| `COUNT(column)` | Count non-null values |
| `COUNT(DISTINCT column)` | Count unique values |
| `SUM(column)`, `AVG(column)` | Numeric aggregations |
| `MIN(column)`, `MAX(column)` | Min/max values |

```sql
-- Multiple aggregations with GROUP BY
SELECT region, COUNT(*), SUM(amount), AVG(amount)
FROM sales.transactions
WHERE sale_date >= '2024-01-01'
GROUP BY region;
```

## HAVING Clause

Filter aggregated results (after GROUP BY):

```sql
SELECT category, SUM(amount)
FROM sales.transactions
GROUP BY category
HAVING SUM(amount) > 10000;
```

## ORDER BY Clause

Sort results by:

- **Partition key columns** - Always supported
- **Aggregation functions** - Supported via shuffle strategy

```sql
-- Order by partition key
SELECT * FROM logs.requests ORDER BY timestamp DESC LIMIT 100;

-- Order by aggregation (repeat function, aliases not supported)
SELECT region, SUM(amount)
FROM sales.transactions
GROUP BY region
ORDER BY SUM(amount) DESC;
```

**Limitations:** Cannot order by non-partition columns. See [gotchas.md](gotchas.md#order-by-limitations)

## LIMIT Clause

```sql
SELECT * FROM logs.requests LIMIT 100;
```

| Setting | Value |
| --------- | ------- |
| Min | 1 |
| Max | 10,000 |
| Default | 500 |

**Always use LIMIT** to enable early termination optimization.

## Data Types

| Type | SQL Literal | Example |
| ------ | ------------- | --------- |
| `integer` | Unquoted number | `42`, `-10` |
| `float` | Decimal number | `3.14`, `-0.5` |
| `string` | Single quotes | `'hello'`, `'GET'` |
| `boolean` | Keyword | `true`, `false` |
| `timestamp` | RFC3339 string | `'2025-01-01T00:00:00Z'` |
| `date` | ISO 8601 date | `'2025-01-01'` |

### Type Safety

- Quote strings with single quotes: `'value'`
- Timestamps must be RFC3339: `'2025-01-01T00:00:00Z'` (include timezone)
- Dates must be ISO 8601: `'2025-01-01'` (YYYY-MM-DD)
- No implicit conversions

```sql
-- ✅ Correct
WHERE status = 200 AND method = 'GET' AND timestamp > '2025-01-01T00:00:00Z'

-- ❌ Wrong
WHERE status = '200'              -- string instead of integer
WHERE timestamp > '2025-01-01'    -- missing time/timezone
WHERE method = GET                -- unquoted string
```

## Query Result Format

JSON array of objects:

```json
[
  {"user_id": "user_123", "timestamp": "2025-01-15T10:30:00Z", "status": 200},
  {"user_id": "user_456", "timestamp": "2025-01-15T10:31:00Z", "status": 404}
]
```

## See Also

- [patterns.md](patterns.md) - Query examples and use cases
- [gotchas.md](gotchas.md) - SQL limitations and error handling
- [configuration.md](configuration.md) - Setup and authentication

## When to use

Use when the user asks about or needs: R2 SQL API Reference.
﻿---
name: R2 SQL Configuration
description: # R2 SQL Configuration
 
 Setup and configuration for R2 SQL queries.
---

# R2 SQL Configuration

Setup and configuration for R2 SQL queries.

## Prerequisites

- R2 bucket with Data Catalog enabled
- API token with R2 permissions
- Wrangler CLI installed (for CLI queries)

## Enable R2 Data Catalog

R2 SQL queries Apache Iceberg tables in R2 Data Catalog. Must enable catalog on bucket first.

### Via Wrangler CLI

```bash
npx wrangler r2 bucket catalog enable <bucket-name>
```

Output includes:

- **Warehouse name** - Typically same as bucket name
- **Catalog URI** - REST endpoint for catalog operations

Example output:

```text
Catalog enabled successfully
Warehouse: my-bucket
Catalog URI: https://abc123.r2.cloudflarestorage.com/iceberg/my-bucket
```

### Via Dashboard

1. Navigate to **R2 Object Storage** → Select your bucket
2. Click **Settings** tab
3. Scroll to **R2 Data Catalog** section
4. Click **Enable**
5. Note the **Catalog URI** and **Warehouse** name

**Important:** Enabling catalog creates metadata directories in bucket but does not modify existing objects.

## Create API Token

R2 SQL requires API token with R2 permissions.

### Required Permission

**R2 Admin Read & Write** (includes R2 SQL Read permission)

### Via Dashboard (Create API Token)

1. Navigate to **R2 Object Storage**
2. Click **Manage API tokens** (top right)
3. Click **Create API token**
4. Select **Admin Read & Write** permission
5. Click **Create API Token**
6. **Copy token value** - shown only once

### Permission Scope

| Permission | Grants Access To |
| ------------ | ------------------ |
| R2 Admin Read & Write | R2 storage operations + R2 SQL queries + Data Catalog operations |
| R2 SQL Read | SQL queries only (no storage writes) |

**Note:** R2 SQL Read permission not yet available via Dashboard - use Admin Read & Write.

## Configure Environment

### Wrangler CLI

Set environment variable for Wrangler to use:

```bash
export WRANGLER_R2_SQL_AUTH_TOKEN=<your-token>
```

Or create `.env` file in project directory:

```text
WRANGLER_R2_SQL_AUTH_TOKEN=<your-token>
```

Wrangler automatically loads `.env` file when running commands.

### HTTP API

For programmatic access (non-Wrangler), pass token in Authorization header:

```bash
curl -X POST https://api.cloudflare.com/client/v4/accounts/{account_id}/r2/sql/query \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "warehouse": "my-bucket",
    "query": "SELECT * FROM default.my_table LIMIT 10"
  }'
```

**Note:** HTTP API endpoint URL may vary - see [patterns.md](patterns.md#http-api-query) for current endpoint.

## Verify Setup

Test configuration by querying system tables:

```bash
# List namespaces
npx wrangler r2 sql query "my-bucket" "SHOW DATABASES"

# List tables in namespace
npx wrangler r2 sql query "my-bucket" "SHOW TABLES IN default"
```

If successful, returns JSON array of results.

## Troubleshooting

### "Token authentication failed"

**Cause:** Invalid or missing token

#### Solution

- Verify `WRANGLER_R2_SQL_AUTH_TOKEN` environment variable set
- Check token has Admin Read & Write permission
- Create new token if expired

### "Catalog not enabled on bucket"

**Cause:** Data Catalog not enabled

#### Solution ("Catalog not enabled on bucket")

- Run `npx wrangler r2 bucket catalog enable <bucket-name>`
- Or enable via Dashboard (R2 → bucket → Settings → R2 Data Catalog)

### "Permission denied"

**Cause:** Token lacks required permissions

#### Solution ("Permission denied")

- Verify token has **Admin Read & Write** permission
- Create new token with correct permissions

## See Also

- [r2-data-catalog/configuration.md](../r2-data-catalog/configuration.md) - Detailed token setup and PyIceberg connection
- [patterns.md](patterns.md) - Query examples using configuration
- [gotchas.md](gotchas.md) - Common configuration errors

## When to use

Use when the user asks about or needs: R2 SQL Configuration.
﻿---
name: R2 SQL Gotchas
description: # R2 SQL Gotchas
 
 Limitations, troubleshooting, and common pitfalls for R2 SQL.
---

# R2 SQL Gotchas

Limitations, troubleshooting, and common pitfalls for R2 SQL.

## Critical Limitations

### No Workers Binding

**Cannot call R2 SQL from Workers/Pages code** - no binding exists.

```typescript
// ❌ This doesn't exist
export default {
  async fetch(request, env) {
    const result = await env.R2_SQL.query("SELECT * FROM table");  // Not possible
    return Response.json(result);
  }
};
```

#### Solutions

- HTTP API from external systems (not Workers)
- PyIceberg/Spark via r2-data-catalog REST API
- For Workers, use D1 or external databases

### ORDER BY Limitations

Can only order by:

1. **Partition key columns** - Always supported
2. **Aggregation functions** - Supported via shuffle strategy

**Cannot order by** regular non-partition columns.

```sql
-- ✅ Valid: ORDER BY partition key
SELECT * FROM logs.requests ORDER BY timestamp DESC LIMIT 100;

-- ✅ Valid: ORDER BY aggregation
SELECT region, SUM(amount) FROM sales.transactions
GROUP BY region ORDER BY SUM(amount) DESC;

-- ❌ Invalid: ORDER BY non-partition column
SELECT * FROM logs.requests ORDER BY user_id;

-- ❌ Invalid: ORDER BY alias (must repeat function)
SELECT region, SUM(amount) as total FROM sales.transactions
GROUP BY region ORDER BY total;  -- Use ORDER BY SUM(amount)
```

Check partition spec: `DESCRIBE namespace.table_name`

## SQL Feature Limitations

| Feature | Supported | Notes |
| --------- | ----------- | ------- |
| SELECT, WHERE, GROUP BY, HAVING | ✅ | Standard support |
| COUNT, SUM, AVG, MIN, MAX | ✅ | Standard aggregations |
| ORDER BY partition/aggregation | ✅ | See above |
| LIMIT | ✅ | Max 10,000 |
| Column aliases | ❌ | No AS alias |
| Expressions in SELECT | ❌ | No col1 + col2 |
| ORDER BY non-partition | ❌ | Fails at runtime |
| JOINs, subqueries, CTEs | ❌ | Denormalize at write time |
| Window functions, UNION | ❌ | Use external engines |
| INSERT/UPDATE/DELETE | ❌ | Use PyIceberg/Pipelines |
| Nested columns, arrays, JSON | ❌ | Flatten at write time |

### Workarounds

- No JOINs: Denormalize data or use Spark/PyIceberg
- No subqueries: Split into multiple queries
- No aliases: Accept generated names, transform in app

## Common Errors

### "Column not found"

**Cause:** Typo, column doesn't exist, or case mismatch  
**Solution:** `DESCRIBE namespace.table_name` to check schema

### "Type mismatch"

```sql
-- ❌ Wrong types
WHERE status = '200'              -- string instead of integer
WHERE timestamp > '2025-01-01'    -- missing time/timezone

-- ✅ Correct types
WHERE status = 200
WHERE timestamp > '2025-01-01T00:00:00Z'
```

### "ORDER BY column not in partition key"

**Cause:** Ordering by non-partition column  
**Solution:** Use partition key, aggregation, or remove ORDER BY. Check: `DESCRIBE table`

### "Token authentication failed"

```bash
# Check/set token
echo $WRANGLER_R2_SQL_AUTH_TOKEN
export WRANGLER_R2_SQL_AUTH_TOKEN=<your-token>

# Or .env file
echo "WRANGLER_R2_SQL_AUTH_TOKEN=<your-token>" > .env
```

### "Table not found"

```sql
-- Verify catalog and tables
SHOW DATABASES;
SHOW TABLES IN namespace_name;
```

Enable catalog: `npx wrangler r2 bucket catalog enable <bucket>`

### "LIMIT exceeds maximum"

Max LIMIT is 10,000. For pagination, use WHERE filters with partition keys.

### "No data returned" (unexpected)

#### Debug steps

1. `SELECT COUNT(*) FROM table` - verify data exists
2. Remove WHERE filters incrementally
3. `SELECT * FROM table LIMIT 10` - inspect actual data/types

## Performance Issues

### Slow Queries

**Causes:** Too many partitions, large LIMIT, no filters, small files

```sql
-- ❌ Slow: No filters
SELECT * FROM logs.requests LIMIT 10000;

-- ✅ Fast: Filter on partition key
SELECT * FROM logs.requests 
WHERE timestamp >= '2025-01-15T00:00:00Z' AND timestamp < '2025-01-16T00:00:00Z'
LIMIT 1000;

-- ✅ Faster: Multiple filters
SELECT * FROM logs.requests 
WHERE timestamp >= '2025-01-15T00:00:00Z' AND status = 404 AND method = 'GET'
LIMIT 1000;
```

#### File optimization

- Target Parquet size: 100-500MB compressed
- Pipelines roll interval: 300+ sec (prod), 10 sec (dev)
- Run compaction to merge small files

### Query Timeout

**Solution:** Add restrictive WHERE filters, reduce time range, query smaller intervals

```sql
-- ❌ Times out: Year-long aggregation
SELECT status, COUNT(*) FROM logs.requests 
WHERE timestamp >= '2024-01-01T00:00:00Z' GROUP BY status;

-- ✅ Faster: Month-long aggregation
SELECT status, COUNT(*) FROM logs.requests 
WHERE timestamp >= '2025-01-01T00:00:00Z' AND timestamp < '2025-02-01T00:00:00Z'
GROUP BY status;
```

## Best Practices

### Partitioning

- **Time-series:** Partition by day/hour on timestamp
- **Avoid:** High-cardinality keys (user_id), >10,000 partitions

```python
from pyiceberg.partitioning import PartitionSpec, PartitionField
from pyiceberg.transforms import DayTransform

PartitionSpec(PartitionField(source_id=1, field_id=1000, transform=DayTransform(), name="day"))
```

### Query Writing

- **Always use LIMIT** for early termination
- **Filter on partition keys first** for pruning
- **Combine filters with AND** for more pruning

```sql
-- Good
WHERE timestamp >= '2025-01-15T00:00:00Z' AND status = 404 AND method = 'GET' LIMIT 100
```

### Type Safety

- Quote strings: `'GET'` not `GET`
- RFC3339 timestamps: `'2025-01-01T00:00:00Z'` not `'2025-01-01'`
- ISO dates: `'2025-01-15'` not `'01/15/2025'`

### Data Organization

- **Pipelines:** Dev `roll_file_time: 10`, Prod `roll_file_time: 300+`
- **Compression:** Use `zstd`
- **Maintenance:** Compaction for small files, expire old snapshots

## Debugging Checklist

1. `npx wrangler r2 bucket catalog enable <bucket>` - Verify catalog
2. `echo $WRANGLER_R2_SQL_AUTH_TOKEN` - Check token
3. `SHOW DATABASES` - List namespaces
4. `SHOW TABLES IN namespace` - List tables
5. `DESCRIBE namespace.table` - Check schema
6. `SELECT COUNT(*) FROM namespace.table` - Verify data
7. `SELECT * FROM namespace.table LIMIT 10` - Test simple query
8. Add filters incrementally

## See Also

- [api.md](api.md) - SQL syntax
- [patterns.md](patterns.md) - Query optimization
- [configuration.md](configuration.md) - Setup
- [Cloudflare R2 SQL Docs](https://developers.cloudflare.com/r2-sql/)

## When to use

Use when the user asks about or needs: R2 SQL Gotchas.
﻿---
name: R2 SQL Patterns
description: # R2 SQL Patterns
 
 Common patterns, use cases, and integration examples for R2 SQL.
---

# R2 SQL Patterns

Common patterns, use cases, and integration examples for R2 SQL.

## Wrangler CLI Query

```bash
# Basic query
npx wrangler r2 sql query "my-bucket" "SELECT * FROM default.logs LIMIT 10"

# Multi-line query
npx wrangler r2 sql query "my-bucket" "
  SELECT status, COUNT(*), AVG(response_time)
  FROM logs.http_requests
  WHERE timestamp >= '2025-01-01T00:00:00Z'
  GROUP BY status
  ORDER BY COUNT(*) DESC
  LIMIT 100
"

# Use environment variable
export R2_SQL_WAREHOUSE="my-bucket"
npx wrangler r2 sql query "$R2_SQL_WAREHOUSE" "SELECT * FROM default.logs"
```

## HTTP API Query

For programmatic access from external systems (not Workers - see gotchas.md).

```bash
curl -X POST https://api.cloudflare.com/client/v4/accounts/{account_id}/r2/sql/query \
  -H "Authorization: Bearer <your-token>" \
  -H "Content-Type: application/json" \
  -d '{
    "warehouse": "my-bucket",
    "query": "SELECT * FROM default.my_table WHERE status = 200 LIMIT 100"
  }'
```

Response:

```json
{
  "success": true,
  "result": [{"user_id": "user_123", "timestamp": "2025-01-15T10:30:00Z", "status": 200}],
  "errors": []
}
```

## Pipelines Integration

Stream data to Iceberg tables via Pipelines, then query with R2 SQL.

```bash
# Setup pipeline (select Data Catalog Table destination)
npx wrangler pipelines setup

# Key settings:
# - Destination: Data Catalog Table
# - Compression: zstd (recommended)
# - Roll file time: 300+ sec (production), 10 sec (dev)

# Send data to pipeline
curl -X POST https://{stream-id}.ingest.cloudflare.com \
  -H "Content-Type: application/json" \
  -d '[{"user_id": "user_123", "event_type": "purchase", "timestamp": "2025-01-15T10:30:00Z", "amount": 29.99}]'

# Query ingested data (wait for roll interval)
npx wrangler r2 sql query "my-bucket" "
  SELECT event_type, COUNT(*), SUM(amount)
  FROM default.events
  WHERE timestamp >= '2025-01-15T00:00:00Z'
  GROUP BY event_type
"
```

See [pipelines/patterns.md](../pipelines/patterns.md) for detailed setup.

## PyIceberg Integration

Create and populate Iceberg tables with PyIceberg, then query with R2 SQL.

```python
from pyiceberg.catalog.rest import RestCatalog
import pyarrow as pa
import pandas as pd

# Setup catalog
catalog = RestCatalog(
    name="my_catalog",
    warehouse="my-bucket",
    uri="https://<account-id>.r2.cloudflarestorage.com/iceberg/my-bucket",
    token="<your-token>",
)
catalog.create_namespace_if_not_exists("analytics")

# Create table
schema = pa.schema([
    pa.field("user_id", pa.string(), nullable=False),
    pa.field("event_time", pa.timestamp("us", tz="UTC"), nullable=False),
    pa.field("page_views", pa.int64(), nullable=False),
])
table = catalog.create_table(("analytics", "user_metrics"), schema=schema)

# Append data
df = pd.DataFrame({
    "user_id": ["user_1", "user_2"],
    "event_time": pd.to_datetime(["2025-01-15 10:00:00", "2025-01-15 11:00:00"], utc=True),
    "page_views": [10, 25],
})
table.append(pa.Table.from_pandas(df, schema=schema))
```

Query with R2 SQL:

```bash
npx wrangler r2 sql query "my-bucket" "
  SELECT user_id, SUM(page_views)
  FROM analytics.user_metrics
  WHERE event_time >= '2025-01-15T00:00:00Z'
  GROUP BY user_id
"
```

See [r2-data-catalog/patterns.md](../r2-data-catalog/patterns.md) for advanced PyIceberg patterns.

## Use Cases

### Log Analytics

```sql
-- Error rate by endpoint
SELECT path, COUNT(*), SUM(CASE WHEN status >= 400 THEN 1 ELSE 0 END) as errors
FROM logs.http_requests
WHERE timestamp BETWEEN '2025-01-01T00:00:00Z' AND '2025-01-31T23:59:59Z'
GROUP BY path ORDER BY errors DESC LIMIT 20;

-- Response time stats
SELECT method, MIN(response_time_ms), AVG(response_time_ms), MAX(response_time_ms)
FROM logs.http_requests WHERE timestamp >= '2025-01-15T00:00:00Z' GROUP BY method;

-- Traffic by status
SELECT status, COUNT(*) FROM logs.http_requests
WHERE timestamp >= '2025-01-15T00:00:00Z' AND method = 'GET'
GROUP BY status ORDER BY COUNT(*) DESC;
```

### Fraud Detection

```sql
-- High-value transactions
SELECT location, COUNT(*), SUM(amount), AVG(amount)
FROM fraud.transactions WHERE transaction_timestamp >= '2025-01-01T00:00:00Z' AND amount > 1000.0
GROUP BY location ORDER BY SUM(amount) DESC LIMIT 20;

-- Flagged transactions
SELECT merchant_category, COUNT(*), AVG(amount) FROM fraud.transactions
WHERE is_fraud_flag = true AND transaction_timestamp >= '2025-01-01T00:00:00Z'
GROUP BY merchant_category HAVING COUNT(*) > 10 ORDER BY COUNT(*) DESC;
```

### Business Intelligence

```sql
-- Sales by department
SELECT department, SUM(revenue), AVG(revenue), COUNT(*) FROM sales.transactions
WHERE sale_date >= '2024-01-01' GROUP BY department ORDER BY SUM(revenue) DESC LIMIT 10;

-- Product performance
SELECT category, COUNT(DISTINCT product_id), SUM(units_sold), SUM(revenue)
FROM sales.product_sales WHERE sale_date BETWEEN '2024-10-01' AND '2024-12-31'
GROUP BY category ORDER BY SUM(revenue) DESC;
```

## Connecting External Engines

R2 Data Catalog exposes Iceberg REST API. Connect Spark, Snowflake, Trino, DuckDB, etc.

```scala
// Apache Spark example
val spark = SparkSession.builder()
  .config("spark.sql.catalog.my_catalog", "org.apache.iceberg.spark.SparkCatalog")
  .config("spark.sql.catalog.my_catalog.catalog-impl", "org.apache.iceberg.rest.RESTCatalog")
  .config("spark.sql.catalog.my_catalog.uri", "https://<account-id>.r2.cloudflarestorage.com/iceberg/my-bucket")
  .config("spark.sql.catalog.my_catalog.token", "<token>")
  .getOrCreate()

spark.sql("SELECT * FROM my_catalog.default.my_table LIMIT 10").show()
```

See [r2-data-catalog/patterns.md](../r2-data-catalog/patterns.md) for more engines.

## Performance Optimization

### Partitioning

- **Time-series:** day/hour on timestamp
- **Geographic:** region/country
- **Avoid:** High-cardinality keys (user_id)

```python
from pyiceberg.partitioning import PartitionSpec, PartitionField
from pyiceberg.transforms import DayTransform

PartitionSpec(PartitionField(source_id=1, field_id=1000, transform=DayTransform(), name="day"))
```

### Query Optimization

- **Always use LIMIT** for early termination
- **Filter on partition keys first**
- **Multiple filters** for better pruning

```sql
-- Better: Multiple filters on partition key
SELECT * FROM logs.requests 
WHERE timestamp >= '2025-01-15T00:00:00Z' AND status = 404 AND method = 'GET' LIMIT 100;
```

### File Organization

- **Pipelines roll:** Dev 10-30s, Prod 300+s
- **Target Parquet:** 100-500MB compressed

## See Also

- [api.md](api.md) - SQL syntax reference
- [gotchas.md](gotchas.md) - Limitations and troubleshooting
- [r2-data-catalog/patterns.md](../r2-data-catalog/patterns.md) - PyIceberg advanced patterns
- [pipelines/patterns.md](../pipelines/patterns.md) - Streaming ingestion patterns

## When to use

Use when the user asks about or needs: R2 SQL Patterns.
﻿---
name: KV Configuration
description: # KV Configuration
 
 ## Create Namespace
---

# KV Configuration

## Create Namespace (KV Configuration)

```bash
wrangler kv namespace create MY_NAMESPACE
# Output: { binding = "MY_NAMESPACE", id = "abc123..." }

wrangler kv namespace create MY_NAMESPACE --preview  # For local dev
```

## Workers Binding

### wrangler.jsonc

```jsonc
{
  "kv_namespaces": [
    {
      "binding": "MY_KV",
      "id": "abc123xyz789"
    },
    // Optional: Different namespace for preview/development
    {
      "binding": "MY_KV",
      "preview_id": "preview-abc123"
    }
  ]
}
```

## TypeScript Types

### env.d.ts

```typescript
interface Env {
  MY_KV: KVNamespace;
  SESSIONS: KVNamespace;
  CACHE: KVNamespace;
}
```

#### worker.ts

```typescript
export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    // env.MY_KV is now typed as KVNamespace
    const value = await env.MY_KV.get("key");
    return new Response(value || "Not found");
  }
} satisfies ExportedHandler<Env>;
```

#### Type-safe JSON operations

```typescript
interface UserProfile {
  name: string;
  email: string;
  role: "admin" | "user";
}

const profile = await env.USERS.get<UserProfile>("user:123", "json");
// profile: UserProfile | null (type-safe!)
if (profile) {
  console.log(profile.name); // TypeScript knows this is a string
}
```

## CLI Operations

```bash
# Put
wrangler kv key put --binding=MY_KV "key" "value"
wrangler kv key put --binding=MY_KV "key" --path=./file.json --ttl=3600

# Get
wrangler kv key get --binding=MY_KV "key"

# Delete
wrangler kv key delete --binding=MY_KV "key"

# List
wrangler kv key list --binding=MY_KV --prefix="user:"

# Bulk operations (max 10,000 keys per file)
wrangler kv bulk put data.json --binding=MY_KV
wrangler kv bulk get keys.json --binding=MY_KV
wrangler kv bulk delete keys.json --binding=MY_KV --force
```

## Local Development

```bash
wrangler dev                # Local KV (isolated)
wrangler dev --remote       # Remote KV (production)

# Or in wrangler.jsonc:
# "kv_namespaces": [{ "binding": "MY_KV", "id": "...", "remote": true }]
```

## REST API

### Single Operations

```typescript
import Cloudflare from 'cloudflare';

const client = new Cloudflare({
  apiEmail: process.env.CLOUDFLARE_EMAIL,
  apiKey: process.env.CLOUDFLARE_API_KEY
});

// Single key operations
await client.kv.namespaces.values.update(namespaceId, 'key', {
  account_id: accountId,
  value: 'value',
  expiration_ttl: 3600
});
```

### Bulk Operations

```typescript
// Bulk update (up to 10,000 keys, max 100MB total)
await client.kv.namespaces.bulkUpdate(namespaceId, {
  account_id: accountId,
  body: [
    { key: "key1", value: "value1", expiration_ttl: 3600 },
    { key: "key2", value: "value2", metadata: { version: 1 } },
    { key: "key3", value: "value3" }
  ]
});

// Bulk get (up to 100 keys)
const results = await client.kv.namespaces.bulkGet(namespaceId, {
  account_id: accountId,
  keys: ["key1", "key2", "key3"]
});

// Bulk delete (up to 10,000 keys)
await client.kv.namespaces.bulkDelete(namespaceId, {
  account_id: accountId,
  keys: ["key1", "key2", "key3"]
});
```

## When to use

Use when the user asks about or needs: KV Configuration.
﻿---
name: KV Patterns & Best Practices
description: # KV Patterns & Best Practices
 
 ## Multi-Tier Caching
---

# KV Patterns & Best Practices

## Multi-Tier Caching (KV Patterns & Best Practices)

```typescript
// Memory → KV → Origin (3-tier cache)
const memoryCache = new Map<string, { data: any; expires: number }>();

async function getCached(env: Env, key: string): Promise<any> {
  const now = Date.now();
  
  // L1: Memory cache (fastest)
  const cached = memoryCache.get(key);
  if (cached && cached.expires > now) {
    return cached.data;
  }
  
  // L2: KV cache (fast)
  const kvValue = await env.CACHE.get(key, "json");
  if (kvValue) {
    memoryCache.set(key, { data: kvValue, expires: now + 60000 }); // 1min in memory
    return kvValue;
  }
  
  // L3: Origin (slow)
  const origin = await fetch(`https://api.example.com/${key}`).then(r => r.json());
  
  // Backfill caches
  await env.CACHE.put(key, JSON.stringify(origin), { expirationTtl: 300 }); // 5min in KV
  memoryCache.set(key, { data: origin, expires: now + 60000 });
  
  return origin;
}
```

## API Response Caching

```typescript
async function getCachedData(env: Env, key: string, fetcher: () => Promise<any>): Promise<any> {
  const cached = await env.MY_KV.get(key, "json");
  if (cached) return cached;
  
  const data = await fetcher();
  await env.MY_KV.put(key, JSON.stringify(data), { expirationTtl: 300 });
  return data;
}

const apiData = await getCachedData(
  env,
  "cache:users",
  () => fetch("https://api.example.com/users").then(r => r.json())
);
```

## Session Management

```typescript
interface Session { userId: string; expiresAt: number; }

async function createSession(env: Env, userId: string): Promise<string> {
  const sessionId = crypto.randomUUID();
  const expiresAt = Date.now() + (24 * 60 * 60 * 1000);
  
  await env.SESSIONS.put(
    `session:${sessionId}`,
    JSON.stringify({ userId, expiresAt }),
    { expirationTtl: 86400, metadata: { createdAt: Date.now() } }
  );
  
  return sessionId;
}

async function getSession(env: Env, sessionId: string): Promise<Session | null> {
  const data = await env.SESSIONS.get<Session>(`session:${sessionId}`, "json");
  if (!data || data.expiresAt < Date.now()) return null;
  return data;
}
```

## Coalesce Cold Keys

```typescript
// ❌ BAD: Many individual keys
await env.KV.put("user:123:name", "John");
await env.KV.put("user:123:email", "john@example.com");

// ✅ GOOD: Single coalesced object
await env.USERS.put("user:123:profile", JSON.stringify({
  name: "John",
  email: "john@example.com",
  role: "admin"
}));

// Benefits: Hot key cache, single read, reduced operations
// Trade-off: Harder to update individual fields
```

## Prefix-Based Namespacing

```typescript
// Logical partitioning within single namespace
const PREFIXES = {
  users: "user:",
  sessions: "session:",
  cache: "cache:",
  features: "feature:"
} as const;

// Write with prefix
async function setUser(env: Env, id: string, data: any) {
  await env.KV.put(`${PREFIXES.users}${id}`, JSON.stringify(data));
}

// Read with prefix
async function getUser(env: Env, id: string) {
  return await env.KV.get(`${PREFIXES.users}${id}`, "json");
}

// List by prefix
async function listUserIds(env: Env): Promise<string[]> {
  const result = await env.KV.list({ prefix: PREFIXES.users });
  return result.keys.map(k => k.name.replace(PREFIXES.users, ""));
}

// Example hierarchy
"user:123:profile"
"user:123:settings"
"cache:api:users"
"session:abc-def"
"feature:flags:beta"
```

## Metadata Versioning

```typescript
interface VersionedData {
  version: number;
  data: any;
}

async function migrateIfNeeded(env: Env, key: string) {
  const result = await env.DATA.getWithMetadata(key, "json");
  
  if (!result.value) return null;
  
  const currentVersion = result.metadata?.version || 1;
  const targetVersion = 2;
  
  if (currentVersion < targetVersion) {
    // Migrate data format
    const migrated = migrate(result.value, currentVersion, targetVersion);
    
    // Store with new version
    await env.DATA.put(key, JSON.stringify(migrated), {
      metadata: { version: targetVersion, migratedAt: Date.now() }
    });
    
    return migrated;
  }
  
  return result.value;
}

function migrate(data: any, from: number, to: number): any {
  if (from === 1 && to === 2) {
    // V1 → V2: Rename field
    return { ...data, userName: data.name };
  }
  return data;
}
```

## Error Boundary Pattern

```typescript
// Resilient get with fallback
async function resilientGet<T>(
  env: Env,
  key: string,
  fallback: T
): Promise<T> {
  try {
    const value = await env.KV.get<T>(key, "json");
    return value ?? fallback;
  } catch (err) {
    console.error(`KV error for ${key}:`, err);
    return fallback;
  }
}

// Usage
const config = await resilientGet(env, "config:app", {
  theme: "light",
  maxItems: 10
});
```

## When to use

Use when the user asks about or needs: KV Patterns & Best Practices.
