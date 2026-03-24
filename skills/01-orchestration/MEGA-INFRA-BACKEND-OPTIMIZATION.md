---
name: Use Connection Pooling for All Applications
description: ## Use Connection Pooling for All Applications
 
 Postgres connections are expensive (1-3MB RAM each). Without pooling, applications exhaust connections under load.
tags: connection-pooling, pgbouncer, performance, scalability
---

## Use Connection Pooling for All Applications

Postgres connections are expensive (1-3MB RAM each). Without pooling, applications exhaust connections under load.

### Incorrect (new connection per request)

```sql
-- Each request creates a new connection
-- Application code: db.connect() per request
-- Result: 500 concurrent users = 500 connections = crashed database

-- Check current connections
select count(*) from pg_stat_activity;  -- 487 connections!
```

#### Correct (connection pooling)

```sql
-- Use a pooler like PgBouncer between app and database
-- Application connects to pooler, pooler reuses a small pool to Postgres

-- Configure pool_size based on: (CPU cores * 2) + spindle_count
-- Example for 4 cores: pool_size = 10

-- Result: 500 concurrent users share 10 actual connections
select count(*) from pg_stat_activity;  -- 10 connections
```

Pool modes:

- **Transaction mode**: connection returned after each transaction (best for most apps)
- **Session mode**: connection held for entire session (needed for prepared statements, temp tables)

Reference: [Connection Pooling](https://supabase.com/docs/guides/database/connecting-to-postgres#connection-pooler)

## When to use

Use when the user asks about or needs: Use Connection Pooling for All Applications.
﻿---
name: Use Single Dependency Versions Across Monorepo
description: ## Use Single Dependency Versions Across Monorepo
 
 Use a single version of each dependency across all packages in your monorepo.
tags: monorepo, dependencies, installation
---

## Use Single Dependency Versions Across Monorepo

Use a single version of each dependency across all packages in your monorepo.
Prefer exact versions over ranges. Multiple versions cause duplicate code in
bundles, runtime conflicts, and inconsistent behavior across packages.

Use a tool like syncpack to enforce this. As a last resort, use yarn resolutions
or npm overrides.

### Incorrect (version ranges, multiple versions)

```json
// packages/app/package.json
{
  "dependencies": {
    "react-native-reanimated": "^3.0.0"
  }
}

// packages/ui/package.json
{
  "dependencies": {
    "react-native-reanimated": "^3.5.0"
  }
}
```

#### Correct (exact versions, single source of truth)

```json
// package.json (root)
{
  "pnpm": {
    "overrides": {
      "react-native-reanimated": "3.16.1"
    }
  }
}

// packages/app/package.json
{
  "dependencies": {
    "react-native-reanimated": "3.16.1"
  }
}

// packages/ui/package.json
{
  "dependencies": {
    "react-native-reanimated": "3.16.1"
  }
}
```

Use your package manager's override/resolution feature to enforce versions at
the root. When adding dependencies, specify exact versions without `^` or `~`.

## When to use

Use when the user asks about or needs: Use Single Dependency Versions Across Monorepo.
﻿---
name: Use Versioning for Blue-Green Deployments
description: ## Use Versioning for Blue-Green Deployments
 
 Set `applicationVersion` in configuration to tag workflows with a version. DBOS only recovers workflows matching the current application version, preventing code mismatches during recovery.
tags: advanced, versioning, blue-green, deployment
---

## Use Versioning for Blue-Green Deployments

Set `applicationVersion` in configuration to tag workflows with a version. DBOS only recovers workflows matching the current application version, preventing code mismatches during recovery.

### Incorrect (deploying new code that breaks in-progress workflows)

```typescript
DBOS.setConfig({
  name: "my-app",
  systemDatabaseUrl: process.env.DBOS_SYSTEM_DATABASE_URL,
  // No version set - all workflows recovered regardless of code version
});
```

#### Correct (versioned deployment)

```typescript
DBOS.setConfig({
  name: "my-app",
  systemDatabaseUrl: process.env.DBOS_SYSTEM_DATABASE_URL,
  applicationVersion: "2.0.0",
});
```

By default, the application version is automatically computed from a hash of workflow source code. Set it explicitly for more control.

### Directing Enqueued Workflows to Latest Version

Use `DBOS.getLatestApplicationVersion` to route enqueued work to the latest version:

```typescript
const latestVersion = await DBOS.getLatestApplicationVersion();
const handle = await DBOS.startWorkflow(myWorkflow, {
  queueName: "my_queue",
  enqueueOptions: { applicationVersion: latestVersion.versionName },
})(arg1, arg2);
```

Scheduled workflows are automatically enqueued to the latest version.

### Checking and Retiring Old Versions

```typescript
const active = await DBOS.listWorkflows({
  applicationVersion: "1.0.0",
  status: ["ENQUEUED", "PENDING"],
});
if (active.length === 0) {
  console.log("Safe to retire version 1.0.0");
}
```

### Version Management APIs

```typescript
// List all registered versions (newest first)
const versions = await DBOS.listApplicationVersions();

// Get the latest version
const latest = await DBOS.getLatestApplicationVersion();

// Roll back: promote a previous version to latest
await DBOS.setLatestApplicationVersion("1.0.0");
```

### Forking Workflows to a New Version

```typescript
const handle = await DBOS.forkWorkflow<string>(
  workflowID,
  failedStepID,
  { applicationVersion: "2.0.0" }
);
```

Reference: [Versioning](https://docs.dbos.dev/typescript/tutorials/upgrading-workflows#versioning)

## When to use

Use when the user asks about or needs: Use Versioning for Blue-Green Deployments.
﻿---
name: Use Lowercase Identifiers for Compatibility
description: ## Use Lowercase Identifiers for Compatibility
 
 PostgreSQL folds unquoted identifiers to lowercase. Quoted mixed-case identifiers require quotes forever and cause issues with tools, ORMs, and AI assistants that may not recognize them.
tags: naming, identifiers, case-sensitivity, schema, conventions
---

## Use Lowercase Identifiers for Compatibility

PostgreSQL folds unquoted identifiers to lowercase. Quoted mixed-case identifiers require quotes forever and cause issues with tools, ORMs, and AI assistants that may not recognize them.

### Incorrect (mixed-case identifiers)

```sql
-- Quoted identifiers preserve case but require quotes everywhere
CREATE TABLE "Users" (
  "userId" bigint PRIMARY KEY,
  "firstName" text,
  "lastName" text
);

-- Must always quote or queries fail
SELECT "firstName" FROM "Users" WHERE "userId" = 1;

-- This fails - Users becomes users without quotes
SELECT firstName FROM Users;
-- ERROR: relation "users" does not exist
```

#### Correct (lowercase snake_case)

```sql
-- Unquoted lowercase identifiers are portable and tool-friendly
CREATE TABLE users (
  user_id bigint PRIMARY KEY,
  first_name text,
  last_name text
);

-- Works without quotes, recognized by all tools
SELECT first_name FROM users WHERE user_id = 1;
```

Common sources of mixed-case identifiers:

```sql
-- ORMs often generate quoted camelCase - configure them to use snake_case
-- Migrations from other databases may preserve original casing
-- Some GUI tools quote identifiers by default - disable this

-- If stuck with mixed-case, create views as a compatibility layer
CREATE VIEW users AS SELECT "userId" AS user_id, "firstName" AS first_name FROM "Users";
```

Reference: [Identifiers and Key Words](https://www.postgresql.org/docs/current/sql-syntax-lexical.html#SQL-SYNTAX-IDENTIFIERS)

## When to use

Use when the user asks about or needs: Use Lowercase Identifiers for Compatibility.
﻿---
name: Use Steps for External Operations
description: ## Use Steps for External Operations
 
 Any function that performs complex operations, accesses external APIs, or has side effects should be a step. Step results are checkpointed, enabling workflow recovery.
tags: step, external, api, checkpoint
---

## Use Steps for External Operations

Any function that performs complex operations, accesses external APIs, or has side effects should be a step. Step results are checkpointed, enabling workflow recovery.

### Incorrect (external call in workflow)

```typescript
async function myWorkflowFn() {
  // External API call directly in workflow - not checkpointed!
  const response = await fetch("https://api.example.com/data");
  return await response.json();
}
const myWorkflow = DBOS.registerWorkflow(myWorkflowFn);
```

#### Correct (external call in step using `DBOS.runStep`)

```typescript
async function fetchData() {
  return await fetch("https://api.example.com/data").then(r => r.json());
}

async function myWorkflowFn() {
  const data = await DBOS.runStep(fetchData, { name: "fetchData" });
  return data;
}
const myWorkflow = DBOS.registerWorkflow(myWorkflowFn);
```

`DBOS.runStep` can also accept an inline arrow function:

```typescript
async function myWorkflowFn() {
  const data = await DBOS.runStep(
    () => fetch("https://api.example.com/data").then(r => r.json()),
    { name: "fetchData" }
  );
  return data;
}
```

Alternatively, you can use `DBOS.registerStep` to pre-register a step or `@DBOS.step()` as a class decorator, but `DBOS.runStep` is preferred for most use cases.

Step requirements:

- Inputs and outputs must be serializable to JSON
- Cannot call, start, or enqueue workflows from within steps
- Calling a step from another step makes the called step part of the calling step's execution

When to use steps:

- API calls to external services
- File system operations
- Random number generation
- Getting current time
- Any non-deterministic operation

Reference: [DBOS Steps](https://docs.dbos.dev/typescript/tutorials/step-tutorial)

## When to use

Use when the user asks about or needs: Use Steps for External Operations.
﻿---
name: Use DBOS Decorators with Classes
description: ## Use DBOS Decorators with Classes
 
 DBOS decorators work with class methods. Workflow classes must inherit from `DBOSConfiguredInstance`.
tags: classes, dbos_class, instance, oop
---

## Use DBOS Decorators with Classes

DBOS decorators work with class methods. Workflow classes must inherit from `DBOSConfiguredInstance`.

### Incorrect (missing class setup)

```python
class MyService:
    def __init__(self, url):
        self.url = url

    @DBOS.workflow()  # Won't work without proper setup
    def fetch_data(self):
        return self.fetch()
```

#### Correct (proper class setup)

```python
from dbos import DBOS, DBOSConfiguredInstance

@DBOS.dbos_class()
class URLFetcher(DBOSConfiguredInstance):
    def __init__(self, url: str):
        self.url = url
        # instance_name must be unique and passed to super()
        super().__init__(instance_name=url)

    @DBOS.workflow()
    def fetch_workflow(self):
        return self.fetch_url()

    @DBOS.step()
    def fetch_url(self):
        return requests.get(self.url).text

# Instantiate BEFORE DBOS.launch()
example_fetcher = URLFetcher("https://example.com")
api_fetcher = URLFetcher("https://api.example.com")

if __name__ == "__main__":
    DBOS.launch()
    print(example_fetcher.fetch_workflow())
```

Requirements:

- Class must be decorated with `@DBOS.dbos_class()`
- Class must inherit from `DBOSConfiguredInstance`
- `instance_name` must be unique and passed to `super().__init__()`
- All instances must be created before `DBOS.launch()`

Steps can be added to any class without these requirements.

Reference: [Python Classes](https://docs.dbos.dev/python/tutorials/classes)

## When to use

Use when the user asks about or needs: Use DBOS Decorators with Classes.
﻿---
name: Use DBOS with Class Instances
description: ## Use DBOS with Class Instances
 
 Class instance methods can be workflows and steps. Classes with workflow methods must extend `ConfiguredInstance` to enable recovery.
tags: pattern, class, instance, ConfiguredInstance
---

## Use DBOS with Class Instances

Class instance methods can be workflows and steps. Classes with workflow methods must extend `ConfiguredInstance` to enable recovery.

### Incorrect (instance workflows without ConfiguredInstance)

```typescript
class MyWorker {
  constructor(private config: any) {}

  @DBOS.workflow()
  async processTask(task: string) {
    // Recovery won't work - DBOS can't find the instance after restart
  }
}
```

#### Correct (extending ConfiguredInstance)

```typescript
import { DBOS, ConfiguredInstance } from "@dbos-inc/dbos-sdk";

class MyWorker extends ConfiguredInstance {
  cfg: WorkerConfig;

  constructor(name: string, config: WorkerConfig) {
    super(name); // Unique name required for recovery
    this.cfg = config;
  }

  override async initialize(): Promise<void> {
    // Optional: validate config at DBOS.launch() time
  }

  @DBOS.workflow()
  async processTask(task: string): Promise<void> {
    // Can use this.cfg safely - instance is recoverable
    const result = await DBOS.runStep(
      () => fetch(this.cfg.apiUrl).then(r => r.text()),
      { name: "callApi" }
    );
  }
}

// Create instances BEFORE DBOS.launch()
const worker1 = new MyWorker("worker-us", { apiUrl: "https://us.api.com" });
const worker2 = new MyWorker("worker-eu", { apiUrl: "https://eu.api.com" });

// Then launch
await DBOS.launch();
```

Key requirements:

- `ConfiguredInstance` constructor requires a unique `name` per class
- All instances must be created **before** `DBOS.launch()`
- The `initialize()` method is called during launch for validation
- Use `DBOS.runStep` inside instance workflows for step operations
- Event registration decorators like `@DBOS.scheduled` cannot be applied to instance methods

Reference: [Using TypeScript Objects](https://docs.dbos.dev/typescript/tutorials/instantiated-objects)

## When to use

Use when the user asks about or needs: Use DBOS with Class Instances.
﻿---
name: Use Durable Sleep for Delayed Execution
description: ## Use Durable Sleep for Delayed Execution
 
 Use `DBOS.sleep()` for durable delays within workflows. The wakeup time is stored in the database, so the sleep survives restarts.
tags: pattern, sleep, delay, durable, schedule
---

## Use Durable Sleep for Delayed Execution

Use `DBOS.sleep()` for durable delays within workflows. The wakeup time is stored in the database, so the sleep survives restarts.

### Incorrect (non-durable sleep)

```typescript
async function delayedTaskFn() {
  // setTimeout is not durable - lost on restart!
  await new Promise(r => setTimeout(r, 60000));
  await DBOS.runStep(doWork, { name: "doWork" });
}
const delayedTask = DBOS.registerWorkflow(delayedTaskFn);
```

#### Correct (durable sleep)

```typescript
async function delayedTaskFn() {
  // Durable sleep - survives restarts
  await DBOS.sleep(60000); // 60 seconds in milliseconds
  await DBOS.runStep(doWork, { name: "doWork" });
}
const delayedTask = DBOS.registerWorkflow(delayedTaskFn);
```

`DBOS.sleep()` takes milliseconds (unlike Python which takes seconds).

Use cases:

- Scheduling tasks to run in the future
- Implementing retry delays
- Delays spanning hours, days, or weeks

```typescript
async function scheduledTaskFn(task: string) {
  // Sleep for one week
  await DBOS.sleep(7 * 24 * 60 * 60 * 1000);
  await processTask(task);
}
```

For getting the current time durably, use `DBOS.now()`:

```typescript
async function myWorkflowFn() {
  const now = await DBOS.now(); // Checkpointed as a step
  // For random UUIDs:
  const id = await DBOS.randomUUID(); // Checkpointed as a step
}
```

Reference: [Durable Sleep](https://docs.dbos.dev/typescript/tutorials/workflow-tutorial#durable-sleep)

## When to use

Use when the user asks about or needs: Use Durable Sleep for Delayed Execution.
﻿---
name: Use Portable Serialization for Cross-Language Interoperability
description: ## Use Portable Serialization for Cross-Language Interoperability
 
 By default, TypeScript DBOS uses SuperJSON serialization, which only TypeScript can read. Use `"portable"` serialization to write data as JSON that any DBOS SDK (Python, TypeScript, Java, Go) can read and write.
tags: serialization, portable, cross-language, interoperability, json
---

## Use Portable Serialization for Cross-Language Interoperability

By default, TypeScript DBOS uses SuperJSON serialization, which only TypeScript can read. Use `"portable"` serialization to write data as JSON that any DBOS SDK (Python, TypeScript, Java, Go) can read and write.

### Incorrect (default SuperJSON — blocks cross-language access)

```typescript
// A Python or Java client cannot read this workflow's
// inputs, outputs, events, or streams
async function processOrderFn(orderId: string) {
  await DBOS.setEvent("status", { progress: 50 });
  return { result: "done" };
}
const processOrder = DBOS.registerWorkflow(processOrderFn);
```

#### Correct (portable JSON — readable by any language)

```typescript
import { DBOS } from "@dbos-inc/dbos-sdk";

async function processOrderFn(orderId: string) {
  await DBOS.setEvent("status", { progress: 50 });
  return { result: "done" };
}
const processOrder = DBOS.registerWorkflow(processOrderFn, {
  name: "processOrder",
  serializationType: "portable",
});
```

### Supported Portable Types

Portable JSON supports JSON primitives, arrays, and objects. Some TypeScript types are automatically converted:

| TypeScript Type | Portable Representation |
| ----------------- | ------------------------ |
| `Date` | RFC 3339 UTC string |
| `BigInt` | Numeric string |

### Where to Set Serialization

**On workflow registration** — affects inputs, outputs, events, and streams for that workflow:

```typescript
const myWorkflow = DBOS.registerWorkflow(myWorkflowFn, {
  name: "myWorkflow",
  serializationType: "portable",
});
```

Or with a decorator:

```typescript
class Orders {
  @DBOS.workflow({ serializationType: "portable" })
  static async processOrder(orderId: string) {
    await DBOS.setEvent("progress", 50);  // Portable by default
    return { done: true };                // Portable by default
  }
}
```

**On individual operations** — override per-operation when mixing strategies:

```typescript
// Explicitly set portable on send (send is never affected by workflow default)
await DBOS.send(
  "workflow-123",
  { status: "complete" },
  "updates",
  undefined,  // idempotencyKey
  { serializationType: "portable" }
);

// Override on setEvent or writeStream
await DBOS.setEvent("key", value, { serializationType: "portable" });
await DBOS.writeStream("key", value, { serializationType: "portable" });
```

**On enqueue from DBOSClient** — for cross-language workflow submission:

```typescript
import { DBOSClient } from "@dbos-inc/dbos-sdk";

const client = await DBOSClient.create({ systemDatabaseUrl: dbUrl });
const handle = await client.enqueue(
  {
    workflowName: "processOrder",
    queueName: "orders",
    serializationType: "portable",
  },
  "order-123",
);
```

### Serialization Strategy Options

```typescript
// On workflow registration or decorator
serializationType: undefined   // Uses config serializer (SuperJSON by default)
serializationType: "portable"  // Portable JSON for cross-language use
serializationType: "native"    // Explicitly uses native TypeScript serializer
```

### Portable Errors

When a portable workflow fails, the error is serialized in a standard JSON structure all languages can read:

```typescript
import { PortableWorkflowError } from "@dbos-inc/dbos-sdk";

throw new PortableWorkflowError(
  "Order not found",
  "NotFoundError",
  404,
  { orderId: "order-123" },
);
```

Non-portable exceptions raised in a portable workflow are automatically converted to this format on a best-effort basis.

### Key Rules

- `send` is **never** affected by the workflow's serialization strategy — always set `serializationType` explicitly on `send` for cross-language messages
- Step outputs always use the native serializer regardless of workflow strategy (steps are internal)
- `DBOSClient.serializer` must match the app's serializer for **default**-format data, but portable data is always readable

Reference: [Cross-Language Interaction](https://docs.dbos.dev/explanations/portable-workflows)

## When to use

Use when the user asks about or needs: Use Portable Serialization for Cross-Language Interoperability.


---

<!-- merged from: cross-request-lru-caching.md -->

﻿---
name: Cross-Request LRU Caching
description: ## Cross-Request LRU Caching
 
 `React.cache()` only works within one request. For data shared across sequential requests (user clicks button A then button B), use an LRU cache.
tags: server, cache, lru, cross-request
---

## Cross-Request LRU Caching

`React.cache()` only works within one request. For data shared across sequential requests (user clicks button A then button B), use an LRU cache.

### Implementation

```typescript
import { LRUCache } from 'lru-cache'

const cache = new LRUCache<string, any>({
  max: 1000,
  ttl: 5 * 60 * 1000  // 5 minutes
})

export async function getUser(id: string) {
  const cached = cache.get(id)
  if (cached) return cached

  const user = await db.user.findUnique({ where: { id } })
  cache.set(id, user)
  return user
}

// Request 1: DB query, result cached
// Request 2: cache hit, no DB query
```

Use when sequential user actions hit multiple endpoints needing the same data within seconds.

**With Vercel's [Fluid Compute](https://vercel.com/docs/fluid-compute):** LRU caching is especially effective because multiple concurrent requests can share the same function instance and cache. This means the cache persists across requests without needing external storage like Redis.

**In traditional serverless:** Each invocation runs in isolation, so consider Redis for cross-process caching.

Reference: [https://github.com/isaacs/node-lru-cache](https://github.com/isaacs/node-lru-cache)


---

<!-- merged from: cache-property-access-in-loops.md -->

﻿---
name: Cache Property Access in Loops
description: ## Cache Property Access in Loops
 
 Cache object property lookups in hot paths.
tags: javascript, loops, optimization, caching
---

## Cache Property Access in Loops

Cache object property lookups in hot paths.

### Incorrect (3 lookups × N iterations)

```typescript
for (let i = 0; i < arr.length; i++) {
  process(obj.config.settings.value)
}
```

#### Correct (1 lookup total)

```typescript
const value = obj.config.settings.value
const len = arr.length
for (let i = 0; i < len; i++) {
  process(value)
}
```


---

<!-- merged from: cache-repeated-function-calls.md -->

﻿---
name: Cache Repeated Function Calls
description: ## Cache Repeated Function Calls
 
 Use a module-level Map to cache function results when the same function is called repeatedly with the same inputs during render.
tags: javascript, cache, memoization, performance
---

## Cache Repeated Function Calls

Use a module-level Map to cache function results when the same function is called repeatedly with the same inputs during render.

### Incorrect (redundant computation)

```typescript
function ProjectList({ projects }: { projects: Project[] }) {
  return (
    <div>
      {projects.map(project => {
        // slugify() called 100+ times for same project names
        const slug = slugify(project.name)
        
        return <ProjectCard key={project.id} slug={slug} />
      })}
    </div>
  )
}
```

#### Correct (cached results)

```typescript
// Module-level cache
const slugifyCache = new Map<string, string>()

function cachedSlugify(text: string): string {
  if (slugifyCache.has(text)) {
    return slugifyCache.get(text)!
  }
  const result = slugify(text)
  slugifyCache.set(text, result)
  return result
}

function ProjectList({ projects }: { projects: Project[] }) {
  return (
    <div>
      {projects.map(project => {
        // Computed only once per unique project name
        const slug = cachedSlugify(project.name)
        
        return <ProjectCard key={project.id} slug={slug} />
      })}
    </div>
  )
}
```

#### Simpler pattern for single-value functions

```typescript
let isLoggedInCache: boolean | null = null

function isLoggedIn(): boolean {
  if (isLoggedInCache !== null) {
    return isLoggedInCache
  }
  
  isLoggedInCache = document.cookie.includes('auth=')
  return isLoggedInCache
}

// Clear cache when auth changes
function onAuthChange() {
  isLoggedInCache = null
}
```

Use a Map (not a hook) so it works everywhere: utilities, event handlers, not just React components.

Reference: [How we made the Vercel Dashboard twice as fast](https://vercel.com/blog/how-we-made-the-vercel-dashboard-twice-as-fast)


---

<!-- merged from: early-length-check-for-array-comparisons.md -->

﻿---
name: Early Length Check for Array Comparisons
description: ## Early Length Check for Array Comparisons
 
 When comparing arrays with expensive operations (sorting, deep equality, serialization), check lengths first. If lengths differ, the arrays cannot be equal.
tags: javascript, arrays, performance, optimization, comparison
---

## Early Length Check for Array Comparisons

When comparing arrays with expensive operations (sorting, deep equality, serialization), check lengths first. If lengths differ, the arrays cannot be equal.

In real-world applications, this optimization is especially valuable when the comparison runs in hot paths (event handlers, render loops).

### Incorrect (always runs expensive comparison)

```typescript
function hasChanges(current: string[], original: string[]) {
  // Always sorts and joins, even when lengths differ
  return current.sort().join() !== original.sort().join()
}
```

Two O(n log n) sorts run even when `current.length` is 5 and `original.length` is 100. There is also overhead of joining the arrays and comparing the strings.

#### Correct (O(1) length check first)

```typescript
function hasChanges(current: string[], original: string[]) {
  // Early return if lengths differ
  if (current.length !== original.length) {
    return true
  }
  // Only sort when lengths match
  const currentSorted = current.toSorted()
  const originalSorted = original.toSorted()
  for (let i = 0; i < currentSorted.length; i++) {
    if (currentSorted[i] !== originalSorted[i]) {
      return true
    }
  }
  return false
}
```

This new approach is more efficient because:

- It avoids the overhead of sorting and joining the arrays when lengths differ
- It avoids consuming memory for the joined strings (especially important for large arrays)
- It avoids mutating the original arrays
- It returns early when a difference is found


---

<!-- merged from: early-return-from-functions.md -->

﻿---
name: Early Return from Functions
description: ## Early Return from Functions
 
 Return early when result is determined to skip unnecessary processing.
tags: javascript, functions, optimization, early-return
---

## Early Return from Functions

Return early when result is determined to skip unnecessary processing.

### Incorrect (processes all items even after finding answer)

```typescript
function validateUsers(users: User[]) {
  let hasError = false
  let errorMessage = ''
  
  for (const user of users) {
    if (!user.email) {
      hasError = true
      errorMessage = 'Email required'
    }
    if (!user.name) {
      hasError = true
      errorMessage = 'Name required'
    }
    // Continues checking all users even after error found
  }
  
  return hasError ? { valid: false, error: errorMessage } : { valid: true }
}
```

#### Correct (returns immediately on first error)

```typescript
function validateUsers(users: User[]) {
  for (const user of users) {
    if (!user.email) {
      return { valid: false, error: 'Email required' }
    }
    if (!user.name) {
      return { valid: false, error: 'Name required' }
    }
  }

  return { valid: true }
}
```

---

<!-- merged from: http-error-codes-reference.md -->

﻿---
name: HTTP Error Codes Reference
description: # HTTP Error Codes Reference
 
 This file documents HTTP error codes returned by the Claude API, their common causes, and how to handle them. For language-specific error handling examples, see the `python/` or `typescript/` folders.
---

# HTTP Error Codes Reference

This file documents HTTP error codes returned by the Claude API, their common causes, and how to handle them. For language-specific error handling examples, see the `python/` or `typescript/` folders.

## Error Code Summary

| Code | Error Type | Retryable | Common Cause |
| ---- | ----------------------- | --------- | ------------------------------------ |
| 400 | `invalid_request_error` | No | Invalid request format or parameters |
| 401 | `authentication_error` | No | Invalid or missing API key |
| 403 | `permission_error` | No | API key lacks permission |
| 404 | `not_found_error` | No | Invalid endpoint or model ID |
| 413 | `request_too_large` | No | Request exceeds size limits |
| 429 | `rate_limit_error` | Yes | Too many requests |
| 500 | `api_error` | Yes | Anthropic service issue |
| 529 | `overloaded_error` | Yes | API is temporarily overloaded |

## Detailed Error Information

### 400 Bad Request

#### Causes

- Malformed JSON in request body
- Missing required parameters (`model`, `max_tokens`, `messages`)
- Invalid parameter types (e.g., string where integer expected)
- Empty messages array
- Messages not alternating user/assistant

#### Example error

```json
{
  "type": "error",
  "error": {
    "type": "invalid_request_error",
    "message": "messages: roles must alternate between \"user\" and \"assistant\""
  }
}
```

**Fix:** Validate request structure before sending. Check that:

- `model` is a valid model ID
- `max_tokens` is a positive integer
- `messages` array is non-empty and alternates correctly

---

### 401 Unauthorized

#### Causes (401 Unauthorized)

- Missing `x-api-key` header or `Authorization` header
- Invalid API key format
- Revoked or deleted API key

**Fix:** Ensure `ANTHROPIC_API_KEY` environment variable is set correctly.

---

### 403 Forbidden

#### Causes (403 Forbidden)

- API key doesn't have access to the requested model
- Organization-level restrictions
- Attempting to access beta features without beta access

**Fix:** Check your API key permissions in the Console. You may need a different API key or to request access to specific features.

---

### 404 Not Found

#### Causes (404 Not Found)

- Typo in model ID (e.g., `claude-sonnet-4.6` instead of `claude-sonnet-4-6`)
- Using deprecated model ID
- Invalid API endpoint

**Fix:** Use exact model IDs from the models documentation. You can use aliases (e.g., `claude-opus-4-6`).

---

### 413 Request Too Large

#### Causes (413 Request Too Large)

- Request body exceeds maximum size
- Too many tokens in input
- Image data too large

**Fix:** Reduce input size — truncate conversation history, compress/resize images, or split large documents into chunks.

---

### 400 Validation Errors

Some 400 errors are specifically related to parameter validation:

- `max_tokens` exceeds model's limit
- Invalid `temperature` value (must be 0.0-1.0)
- `budget_tokens` >= `max_tokens` in extended thinking
- Invalid tool definition schema

#### Common mistake with extended thinking

```yaml
# Wrong: budget_tokens must be < max_tokens
thinking: budget_tokens=10000, max_tokens=1000  → Error!

# Correct
thinking: budget_tokens=10000, max_tokens=16000
```

---

### 429 Rate Limited

#### Causes (429 Rate Limited)

- Exceeded requests per minute (RPM)
- Exceeded tokens per minute (TPM)
- Exceeded tokens per day (TPD)

#### Headers to check

- `retry-after`: Seconds to wait before retrying
- `x-ratelimit-limit-*`: Your limits
- `x-ratelimit-remaining-*`: Remaining quota

**Fix:** The Anthropic SDKs automatically retry 429 and 5xx errors with exponential backoff (default: `max_retries=2`). For custom retry behavior, see the language-specific error handling examples.

---

### 500 Internal Server Error

#### Causes (500 Internal Server Error)

- Temporary Anthropic service issue
- Bug in API processing

**Fix:** Retry with exponential backoff. If persistent, check [status.anthropic.com](https://status.anthropic.com).

---

### 529 Overloaded

#### Causes (529 Overloaded)

- High API demand
- Service capacity reached

**Fix:** Retry with exponential backoff. Consider using a different model (Haiku is often less loaded), spreading requests over time, or implementing request queuing.

---

## Common Mistakes and Fixes

| Mistake | Error | Fix |
| ------------------------------- | ---------------- | ------------------------------------------------------- |
| `budget_tokens` >= `max_tokens` | 400 | Ensure `budget_tokens` < `max_tokens` |
| Typo in model ID | 404 | Use valid model ID like `claude-opus-4-6` |
| First message is `assistant` | 400 | First message must be `user` |
| Consecutive same-role messages | 400 | Alternate `user` and `assistant` |
| API key in code | 401 (leaked key) | Use environment variable |
| Custom retry needs | 429/5xx | SDK retries automatically; customize with `max_retries` |

## Typed Exceptions in SDKs

**Always use the SDK's typed exception classes** instead of checking error messages with string matching. Each HTTP error code maps to a specific exception class:

| HTTP Code | TypeScript Class | Python Class |
| --------- | --------------------------------- | --------------------------------- |
| 400 | `Anthropic.BadRequestError` | `anthropic.BadRequestError` |
| 401 | `Anthropic.AuthenticationError` | `anthropic.AuthenticationError` |
| 403 | `Anthropic.PermissionDeniedError` | `anthropic.PermissionDeniedError` |
| 404 | `Anthropic.NotFoundError` | `anthropic.NotFoundError` |
| 429 | `Anthropic.RateLimitError` | `anthropic.RateLimitError` |
| 500+ | `Anthropic.InternalServerError` | `anthropic.InternalServerError` |
| Any | `Anthropic.APIError` | `anthropic.APIError` |

```typescript
// ✅ Correct: use typed exceptions
try {
  const response = await client.messages.create({...});
} catch (error) {
  if (error instanceof Anthropic.RateLimitError) {
    // Handle rate limiting
  } else if (error instanceof Anthropic.APIError) {
    console.error(`API error ${error.status}:`, error.message);
  }
}

// ❌ Wrong: don't check error messages with string matching
try {
  const response = await client.messages.create({...});
} catch (error) {
  const msg = error instanceof Error ? error.message : String(error);
  if (msg.includes("429") || msg.includes("rate_limit")) { ... }
}
```

All exception classes extend `Anthropic.APIError`, which has a `status` property. Use `instanceof` checks from most specific to least specific (e.g., check `RateLimitError` before `APIError`).