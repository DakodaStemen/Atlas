---
name: Cancel, Resume, and Fork Workflows
description: ## Cancel, Resume, and Fork Workflows
 
 DBOS provides methods to cancel, resume, and fork workflows for operational control.
tags: workflow, cancel, resume, fork, management
---

## Cancel, Resume, and Fork Workflows

DBOS provides methods to cancel, resume, and fork workflows for operational control.

### Incorrect (no way to handle stuck or failed workflows)

```typescript
// Workflow is stuck or failed - no recovery mechanism
const handle = await DBOS.startWorkflow(processTask)("data");
// If the workflow fails, there's no way to retry or recover
```

#### Correct (using cancel, resume, and fork)

```typescript
// Cancel a workflow - stops at its next step
await DBOS.cancelWorkflow(workflowID);

// Resume from the last completed step
const handle = await DBOS.resumeWorkflow<string>(workflowID);
const result = await handle.getResult();
```

Cancellation sets the workflow status to `CANCELLED` and preempts execution at the beginning of the next step. Cancelling also cancels all child workflows.

Resume restarts a workflow from its last completed step. Use this for workflows that are cancelled or have exceeded their maximum recovery attempts. You can also use this to start an enqueued workflow immediately, bypassing its queue.

Fork a workflow from a specific step:

```typescript
// List steps to find the right step ID
const steps = await DBOS.listWorkflowSteps(workflowID);
// steps[i].functionID is the step's ID

// Fork from a specific step
const forkHandle = await DBOS.forkWorkflow<string>(
  workflowID,
  startStep,
  {
    newWorkflowID: "new-wf-id",
    applicationVersion: "2.0.0",
    timeoutMS: 60000,
  }
);
const forkResult = await forkHandle.getResult();
```

Forking creates a new workflow with a new ID, copying the original workflow's inputs and step outputs up to the selected step. Useful for recovering from downstream service outages or patching workflows that failed due to a bug.

Reference: [Workflow Management](https://docs.dbos.dev/typescript/tutorials/workflow-management)

## When to use

Use when the user asks about or needs: Cancel, Resume, and Fork Workflows.
﻿---
name: Create Scheduled Workflows
description: ## Create Scheduled Workflows
 
 Use `dbos.WithSchedule` when registering a workflow to run it on a cron schedule. Each scheduled invocation runs exactly once per interval.
tags: pattern, scheduled, cron, recurring
---

## Create Scheduled Workflows

Use `dbos.WithSchedule` when registering a workflow to run it on a cron schedule. Each scheduled invocation runs exactly once per interval.

### Incorrect (manual scheduling with goroutine)

```go
// Manual scheduling is not durable and misses intervals during downtime
go func() {
 for {
  generateReport()
  time.Sleep(60 * time.Second)
 }
}()
```

#### Correct (using WithSchedule)

```go
// Scheduled workflow must accept time.Time as input
func everyThirtySeconds(ctx dbos.DBOSContext, scheduledTime time.Time) (string, error) {
 fmt.Println("Running scheduled task at:", scheduledTime)
 return "done", nil
}

func dailyReport(ctx dbos.DBOSContext, scheduledTime time.Time) (string, error) {
 _, err := dbos.RunAsStep(ctx, func(ctx context.Context) (string, error) {
  return generateReport()
 }, dbos.WithStepName("generateReport"))
 return "report generated", err
}

func main() {
 ctx, _ := dbos.NewDBOSContext(context.Background(), config)
 defer dbos.Shutdown(ctx, 30*time.Second)

 dbos.RegisterWorkflow(ctx, everyThirtySeconds,
  dbos.WithSchedule("*/30 * * * * *"),
 )
 dbos.RegisterWorkflow(ctx, dailyReport,
  dbos.WithSchedule("0 0 9 * * *"), // 9 AM daily
 )

 dbos.Launch(ctx)
 select {} // Block forever
}
```

Scheduled workflows must accept exactly one parameter of type `time.Time` representing the scheduled execution time.

DBOS crontab uses 6 fields with second precision:

```text
┌────────────── second
│ ┌──────────── minute
│ │ ┌────────── hour
│ │ │ ┌──────── day of month
│ │ │ │ ┌────── month
│ │ │ │ │ ┌──── day of week
* * * * * *
```

Reference: [Scheduled Workflows](https://docs.dbos.dev/golang/tutorials/workflow-tutorial#scheduled-workflows)

## When to use

Use when the user asks about or needs: Create Scheduled Workflows.
﻿---
name: Debounce Workflows to Prevent Wasted Work
description: ## Debounce Workflows to Prevent Wasted Work
 
 Use `Debouncer` to delay workflow execution until some time has passed since the last trigger. This prevents wasted work when a workflow is triggered multiple times in quick succession.
tags: pattern, debounce, delay, efficiency
---

## Debounce Workflows to Prevent Wasted Work

Use `Debouncer` to delay workflow execution until some time has passed since the last trigger. This prevents wasted work when a workflow is triggered multiple times in quick succession.

### Incorrect (executing on every trigger)

```typescript
async function processInputFn(userInput: string) {
  // Expensive processing
}
const processInput = DBOS.registerWorkflow(processInputFn);

// Every keystroke triggers a new workflow - wasteful!
async function onInputChange(userInput: string) {
  await processInput(userInput);
}
```

#### Correct (using Debouncer)

```typescript
import { DBOS, Debouncer } from "@dbos-inc/dbos-sdk";

async function processInputFn(userInput: string) {
  // Expensive processing
}
const processInput = DBOS.registerWorkflow(processInputFn);

const debouncer = new Debouncer({
  workflow: processInput,
  debounceTimeoutMs: 120000, // Max wait: 2 minutes
});

async function onInputChange(userId: string, userInput: string) {
  // Delays execution by 60 seconds from the last call
  // Uses the LAST set of inputs when finally executing
  await debouncer.debounce(userId, 60000, userInput);
}
```

Key behaviors:

- `debounceKey` groups executions that are debounced together (e.g., per user)
- `debouncePeriodMs` delays execution by this amount from the last call
- `debounceTimeoutMs` sets a max wait time since the first trigger
- When the workflow finally executes, it uses the **last** set of inputs
- After execution begins, the next `debounce` call starts a new cycle
- Workflows from `ConfiguredInstance` classes cannot be debounced

Reference: [Debouncing Workflows](https://docs.dbos.dev/typescript/tutorials/workflow-tutorial#debouncing-workflows)

## When to use

Use when the user asks about or needs: Debounce Workflows to Prevent Wasted Work.
﻿---
name: Deduplicate Queued Workflows
description: ## Deduplicate Queued Workflows
 
 Set a deduplication ID when enqueuing to prevent duplicate workflow executions. If a workflow with the same deduplication ID is already enqueued or executing, a `DBOSQueueDuplicatedError` is thrown.
tags: queue, deduplication, idempotent, duplicate
---

## Deduplicate Queued Workflows

Set a deduplication ID when enqueuing to prevent duplicate workflow executions. If a workflow with the same deduplication ID is already enqueued or executing, a `DBOSQueueDuplicatedError` is thrown.

### Incorrect (no deduplication)

```typescript
// Multiple clicks could enqueue duplicates
async function handleClick(userId: string) {
  await DBOS.startWorkflow(processTask, { queueName: queue.name })("task");
}
```

#### Correct (with deduplication)

```typescript
const queue = new WorkflowQueue("task_queue");

async function processTaskFn(task: string) {
  // ...
}
const processTask = DBOS.registerWorkflow(processTaskFn);

async function handleClick(userId: string) {
  try {
    await DBOS.startWorkflow(processTask, {
      queueName: queue.name,
      enqueueOptions: { deduplicationID: userId },
    })("task");
  } catch (e) {
    // DBOSQueueDuplicatedError - workflow already active for this user
    console.log("Task already in progress for user:", userId);
  }
}
```

Deduplication is per-queue. The deduplication ID is active while the workflow has status `ENQUEUED` or `PENDING`. Once the workflow completes, a new workflow with the same deduplication ID can be enqueued.

This is useful for:

- Ensuring one active task per user
- Preventing duplicate form submissions
- Idempotent event processing

Reference: [Deduplication](https://docs.dbos.dev/typescript/tutorials/queue-tutorial#deduplication)

## When to use

Use when the user asks about or needs: Deduplicate Queued Workflows.
---
name: durable-workflows-queues
description: This skill should be used when implementing durable, asynchronous backend workflows or job scheduling (e.g. Postgres-backed queues, workflow engines). It covers when and how to use durable execution, idempotency, and dynamic job scheduling. Use when the user says "durable workflow", "job queue", "async workflow", "DBOS", or "transactional outbox".
domain: backend
category: workflows
tags: [workflow, queue, durable, async, postgres, job-scheduling]
triggers: durable workflow, job queue, async backend, DBOS, queue listening, job scheduling, transactional outbox
---

# Durable Workflows and Queues

When and how to implement durable, asynchronous backend workflows and Postgres-backed job scheduling. Aligned with dbos-inc/agent-skills and patterns for reliable, recoverable execution.

## When to Use This Skill

- Designing or implementing background jobs, workflows, or task queues.
- Needing guarantees that work completes or can be retried after failure.
- Evaluating Postgres as a queue or workflow store vs. dedicated message brokers.
- Implementing idempotency, sagas, or transactional outbox patterns.

## Core Concepts

### Durable Execution

- Workflow state is persisted; if the process crashes, execution can resume from the last persisted step.
- Use when operations are long-running, multi-step, or must not be lost on restart.
- Prefer idempotent step handlers so retries are safe.

### Postgres as Queue

- Tables used as queues: append jobs, claim with `SELECT ... FOR UPDATE SKIP LOCKED`, process, then delete or mark complete.
- Good for moderate throughput, transactional consistency with application data, and simplicity.
- For very high throughput or fan-out, consider a dedicated broker (e.g. Redis, RabbitMQ, SQS) and use Postgres for workflow state or outbox.

### Transactional Outbox

- Append "outbox" rows in the same transaction as domain changes; a separate process reads the outbox and publishes events or triggers downstream work.
- Ensures at-least-once delivery and avoids dual-write inconsistencies.
- Process outbox in batches; mark as published and handle failures with retries and dead-letter handling.

## Design Rules

1. **Idempotency:** Design job handlers so that running the same job twice (e.g. after retry) produces the same outcome or is harmless.
2. **Visibility:** Store job state (pending, running, done, failed) and optionally timestamps/attempts for observability and debugging.
3. **Timeouts:** Set max execution time per job; mark as failed and requeue or dead-letter if exceeded.
4. **Backpressure:** Limit concurrency (e.g. workers, polling rate) so the queue does not overwhelm the database or downstream services.

## When to Use Which

- **In-process + DB state:** Simple workflows with a few steps; state in Postgres, orchestration in app code (or a small library).
- **Dedicated workflow engine:** Complex DAGs, human-in-the-loop, or many steps; consider engines that support durability (e.g. Temporal, Inngest, or DBOS when applicable).
- **Message broker:** High throughput, fan-out, or need for multiple consumers; use Postgres for outbox or workflow state, broker for delivery.

## Checklist

- [ ] Job handlers are idempotent or retry-safe.
- [ ] State and progress are persisted before long steps.
- [ ] Timeouts and failure handling are defined (requeue, dead-letter, alert).
- [ ] Concurrency and backpressure are limited and configurable.

## Reference

- DBOS: [What's New in DBOS March 2026](https://www.dbos.dev/blog/dbos-new-features-march-2026), durable-workflows and queue-listening skills.
- Transactional outbox: ensure domain write and outbox write in same transaction.
﻿---
name: Enqueue Workflows from External Applications
description: ## Enqueue Workflows from External Applications
 
 Use `client.enqueue()` to submit workflows from outside your DBOS application. Since `DBOSClient` runs externally, workflow and queue metadata must be specified explicitly.
tags: client, enqueue, external, queue
---

## Enqueue Workflows from External Applications

Use `client.enqueue()` to submit workflows from outside your DBOS application. Since `DBOSClient` runs externally, workflow and queue metadata must be specified explicitly.

### Incorrect (trying to use DBOS.startWorkflow from external code)

```typescript
// DBOS.startWorkflow requires a full DBOS setup
await DBOS.startWorkflow(processTask, { queueName: "myQueue" })("data");
```

#### Correct (using DBOSClient.enqueue)

```typescript
import { DBOSClient } from "@dbos-inc/dbos-sdk";

const client = await DBOSClient.create({
  systemDatabaseUrl: process.env.DBOS_SYSTEM_DATABASE_URL,
});

// Basic enqueue
const handle = await client.enqueue(
  {
    workflowName: "processTask",
    queueName: "task_queue",
  },
  "task-data"
);

// Wait for the result
const result = await handle.getResult();
```

#### Type-safe enqueue

```typescript
// Import or declare the workflow type
declare class Tasks {
  static processTask(data: string): Promise<string>;
}

const handle = await client.enqueue<typeof Tasks.processTask>(
  {
    workflowName: "processTask",
    workflowClassName: "Tasks",
    queueName: "task_queue",
  },
  "task-data"
);

// TypeScript infers the result type
const result = await handle.getResult(); // type: string
```

#### Enqueue options

- `workflowName` (required): Name of the workflow function
- `queueName` (required): Name of the queue
- `workflowClassName`: Class name if the workflow is a class method
- `workflowConfigName`: Instance name if using `ConfiguredInstance`
- `workflowID`: Custom workflow ID
- `workflowTimeoutMS`: Timeout in milliseconds
- `deduplicationID`: Prevent duplicate enqueues
- `priority`: Queue priority (lower = higher priority)
- `queuePartitionKey`: Partition key for partitioned queues

Always call `client.destroy()` when done.

Reference: [DBOS Client Enqueue](https://docs.dbos.dev/typescript/reference/client#enqueue)

## When to use

Use when the user asks about or needs: Enqueue Workflows from External Applications.
﻿---
name: Keep Workflows Deterministic
description: ## Keep Workflows Deterministic
 
 Workflow functions must be deterministic: given the same inputs and step return values, they must invoke the same steps in the same order. Non-deterministic operations must be moved to steps.
tags: workflow, determinism, recovery, reliability
---

## Keep Workflows Deterministic

Workflow functions must be deterministic: given the same inputs and step return values, they must invoke the same steps in the same order. Non-deterministic operations must be moved to steps.

### Incorrect (non-deterministic workflow)

```typescript
async function exampleWorkflowFn() {
  // Random value in workflow breaks recovery!
  // On replay, Math.random() returns a different value,
  // so the workflow may take a different branch.
  const choice = Math.random() > 0.5 ? 1 : 0;
  if (choice === 0) {
    await stepOne();
  } else {
    await stepTwo();
  }
}
const exampleWorkflow = DBOS.registerWorkflow(exampleWorkflowFn);
```

#### Correct (non-determinism in step)

```typescript
async function exampleWorkflowFn() {
  // Step result is checkpointed - replay uses the saved value
  const choice = await DBOS.runStep(
    () => Promise.resolve(Math.random() > 0.5 ? 1 : 0),
    { name: "generateChoice" }
  );
  if (choice === 0) {
    await stepOne();
  } else {
    await stepTwo();
  }
}
const exampleWorkflow = DBOS.registerWorkflow(exampleWorkflowFn);
```

Non-deterministic operations that must be in steps:

- Random number generation (use `DBOS.randomUUID()` for UUIDs)
- Getting current time (use `DBOS.now()` for timestamps)
- Accessing external APIs
- Reading files
- Database queries (use transactions or steps)

Reference: [Workflow Determinism](https://docs.dbos.dev/typescript/tutorials/workflow-tutorial#determinism)

## When to use

Use when the user asks about or needs: Keep Workflows Deterministic.
﻿---
name: List and Inspect Workflows
description: ## List and Inspect Workflows
 
 Use `DBOS.listWorkflows` to query workflow executions by status, name, time range, and other criteria.
tags: workflow, list, inspect, status, monitoring
---

## List and Inspect Workflows

Use `DBOS.listWorkflows` to query workflow executions by status, name, time range, and other criteria.

### Incorrect (no monitoring of workflow state)

```typescript
// Start workflow with no way to check on it later
await DBOS.startWorkflow(processTask)("data");
// If something goes wrong, no way to find or debug it
```

#### Correct (listing and inspecting workflows)

```typescript
// List workflows by status
const erroredWorkflows = await DBOS.listWorkflows({
  status: "ERROR",
});

for (const wf of erroredWorkflows) {
  console.log(`Workflow ${wf.workflowID}: ${wf.workflowName} - ${wf.error}`);
}
```

List workflows with multiple filters:

```typescript
const workflows = await DBOS.listWorkflows({
  workflowName: "processOrder",
  status: "SUCCESS",
  limit: 100,
  sortDesc: true,
  loadOutput: true,
});
```

List enqueued workflows:

```typescript
const queued = await DBOS.listQueuedWorkflows({
  queueName: "task_queue",
});
```

List workflow steps:

```typescript
const steps = await DBOS.listWorkflowSteps(workflowID);
if (steps) {
  for (const step of steps) {
    console.log(`Step ${step.functionID}: ${step.name}`);
    if (step.error) console.log(`  Error: ${step.error}`);
    if (step.childWorkflowID) console.log(`  Child: ${step.childWorkflowID}`);
  }
}
```

Workflow status values: `ENQUEUED`, `PENDING`, `SUCCESS`, `ERROR`, `CANCELLED`, `RETRIES_EXCEEDED`

To optimize performance, set `loadInput: false` and `loadOutput: false` when you don't need workflow inputs or outputs.

Reference: [Workflow Management](https://docs.dbos.dev/typescript/tutorials/workflow-management)

## When to use

Use when the user asks about or needs: List and Inspect Workflows.
﻿---
name: Partition Queues for Per-Entity Limits
description: ## Partition Queues for Per-Entity Limits
 
 Partitioned queues apply flow control limits per partition key instead of the entire queue. Each partition acts as a dynamic "subqueue".
tags: queue, partition, per-user, dynamic
---

## Partition Queues for Per-Entity Limits

Partitioned queues apply flow control limits per partition key instead of the entire queue. Each partition acts as a dynamic "subqueue".

### Incorrect (global concurrency for per-user limits)

```typescript
// Global concurrency=1 blocks ALL users, not per-user
const queue = new WorkflowQueue("tasks", { concurrency: 1 });
```

#### Correct (partitioned queue)

```typescript
const queue = new WorkflowQueue("tasks", {
  partitionQueue: true,
  concurrency: 1,
});

async function onUserTask(userID: string, task: string) {
  // Each user gets their own partition - at most 1 task per user
  // but tasks from different users can run concurrently
  await DBOS.startWorkflow(processTask, {
    queueName: queue.name,
    enqueueOptions: { queuePartitionKey: userID },
  })(task);
}
```

#### Two-level queueing (per-user + global limits)

```typescript
const concurrencyQueue = new WorkflowQueue("concurrency-queue", { concurrency: 5 });
const partitionedQueue = new WorkflowQueue("partitioned-queue", {
  partitionQueue: true,
  concurrency: 1,
});

// At most 1 task per user AND at most 5 tasks globally
async function onUserTask(userID: string, task: string) {
  await DBOS.startWorkflow(concurrencyManager, {
    queueName: partitionedQueue.name,
    enqueueOptions: { queuePartitionKey: userID },
  })(task);
}

async function concurrencyManagerFn(task: string) {
  const handle = await DBOS.startWorkflow(processTask, {
    queueName: concurrencyQueue.name,
  })(task);
  return await handle.getResult();
}
const concurrencyManager = DBOS.registerWorkflow(concurrencyManagerFn);
```

Reference: [Partitioning Queues](https://docs.dbos.dev/typescript/tutorials/queue-tutorial#partitioning-queues)

## When to use

Use when the user asks about or needs: Partition Queues for Per-Entity Limits.
﻿---
name: Queues Configuration
description: # Queues Configuration
 
 ## Create Queue
---

# Queues Configuration

## Create Queue (Queues Configuration)

```bash
wrangler queues create my-queue
wrangler queues create my-queue --retention-period-hours=336  # 14 days
wrangler queues create my-queue --delivery-delay-secs=300
```

## Producer Binding

### wrangler.jsonc

```jsonc
{
  "queues": {
    "producers": [
      {
        "queue": "my-queue-name",
        "binding": "MY_QUEUE",
        "delivery_delay": 60  // Optional: default delay in seconds
      }
    ]
  }
}
```

## Consumer Configuration (Push-based)

### wrangler.jsonc (Consumer Configuration (Push-based))

```jsonc
{
  "queues": {
    "consumers": [
      {
        "queue": "my-queue-name",
        "max_batch_size": 10,           // 1-100, default 10
        "max_batch_timeout": 5,         // 0-60s, default 5
        "max_retries": 3,               // default 3, max 100
        "dead_letter_queue": "my-dlq",  // optional
        "retry_delay": 300              // optional: delay retries in seconds
      }
    ]
  }
}
```

## Consumer Configuration (Pull-based)

### wrangler.jsonc (Consumer Configuration (Pull-based))

```jsonc
{
  "queues": {
    "consumers": [
      {
        "queue": "my-queue-name",
        "type": "http_pull",
        "visibility_timeout_ms": 5000,  // default 30000, max 12h
        "max_retries": 5,
        "dead_letter_queue": "my-dlq"
      }
    ]
  }
}
```

## TypeScript Types

```typescript
interface Env {
  MY_QUEUE: Queue<MessageBody>;
  ANALYTICS_QUEUE: Queue<AnalyticsEvent>;
}

interface MessageBody {
  id: string;
  action: 'create' | 'update' | 'delete';
  data: Record<string, any>;
}

export default {
  async queue(batch: MessageBatch<MessageBody>, env: Env): Promise<void> {
    for (const msg of batch.messages) {
      console.log(msg.body.action);
      msg.ack();
    }
  }
} satisfies ExportedHandler<Env>;
```

## Content Type Selection

Choose content type based on consumer type and data requirements:

| Content Type | Use When | Readable By | Supports | Size |
| -------------- | ---------- | ------------- | ---------- | ------ |
| `json` | Pull consumers, dashboard visibility, simple objects | All (push/pull/dashboard) | JSON-serializable types only | Medium |
| `v8` | Push consumers only, complex JS objects | Push consumers only | Date, Map, Set, BigInt, typed arrays | Small |
| `text` | String-only payloads | All | Strings only | Smallest |
| `bytes` | Binary data (images, files) | All | ArrayBuffer, Uint8Array | Variable |

### Decision tree

1. Need to view in dashboard or use pull consumer? → Use `json`
2. Need Date, Map, Set, or other V8 types? → Use `v8` (push consumers only)
3. Just strings? → Use `text`
4. Binary data? → Use `bytes`

```typescript
// JSON: Good for simple objects, pull consumers, dashboard visibility
await env.QUEUE.send({ id: 123, name: 'test' }, { contentType: 'json' });

// V8: Good for Date, Map, Set (push consumers only)
await env.QUEUE.send({ 
  created: new Date(), 
  tags: new Set(['a', 'b']) 
}, { contentType: 'v8' });

// Text: Simple strings
await env.QUEUE.send('process-user-123', { contentType: 'text' });

// Bytes: Binary data
await env.QUEUE.send(imageBuffer, { contentType: 'bytes' });
```

**Default behavior:** If not specified, Cloudflare auto-selects `json` for JSON-serializable objects and `v8` for complex types.

**IMPORTANT:** `v8` messages cannot be read by pull consumers or viewed in the dashboard. Use `json` if you need visibility or pull-based consumption.

## CLI Commands

```bash
# Consumer management
wrangler queues consumer add my-queue my-worker --batch-size=50 --max-retries=5
wrangler queues consumer http add my-queue
wrangler queues consumer worker remove my-queue my-worker
wrangler queues consumer http remove my-queue

# Queue operations
wrangler queues list
wrangler queues pause my-queue
wrangler queues resume my-queue
wrangler queues purge my-queue
wrangler queues delete my-queue
```

## When to use

Use when the user asks about or needs: Queues Configuration.
﻿---
name: Queues Patterns & Best Practices
description: # Queues Patterns & Best Practices
 
 ## Async Task Processing
---

# Queues Patterns & Best Practices

## Async Task Processing (Queues Patterns & Best Practices)

```typescript
// Producer: Accept request, queue work
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { userId, reportType } = await request.json();
    await env.REPORT_QUEUE.send({ userId, reportType, requestedAt: Date.now() });
    return Response.json({ message: 'Report queued', status: 'pending' });
  }
};

// Consumer: Process reports
export default {
  async queue(batch: MessageBatch, env: Env): Promise<void> {
    for (const msg of batch.messages) {
      const { userId, reportType } = msg.body;
      const report = await generateReport(userId, reportType, env);
      await env.REPORTS_BUCKET.put(`${userId}/${reportType}.pdf`, report);
      msg.ack();
    }
  }
};
```

## Buffering API Calls

```typescript
// Producer: Queue log entries
ctx.waitUntil(env.LOGS_QUEUE.send({
  method: request.method,
  url: request.url,
  timestamp: Date.now()
}));

// Consumer: Batch write to external API
async queue(batch: MessageBatch, env: Env): Promise<void> {
  const logs = batch.messages.map(m => m.body);
  await fetch(env.LOG_ENDPOINT, { method: 'POST', body: JSON.stringify({ logs }) });
  batch.ackAll();
}
```

## Rate Limiting Upstream

```typescript
async queue(batch: MessageBatch, env: Env): Promise<void> {
  for (const msg of batch.messages) {
    try {
      await callRateLimitedAPI(msg.body);
      msg.ack();
    } catch (error) {
      if (error.status === 429) {
        const retryAfter = parseInt(error.headers.get('Retry-After') || '60');
        msg.retry({ delaySeconds: retryAfter });
      } else throw error;
    }
  }
}
```

## Event-Driven Workflows

```typescript
// R2 event → Queue → Worker
export default {
  async queue(batch: MessageBatch, env: Env): Promise<void> {
    for (const msg of batch.messages) {
      const event = msg.body;
      if (event.action === 'PutObject') {
        await processNewFile(event.object.key, env);
      } else if (event.action === 'DeleteObject') {
        await cleanupReferences(event.object.key, env);
      }
      msg.ack();
    }
  }
};
```

## Dead Letter Queue Pattern

```typescript
// Main queue: After max_retries, goes to DLQ automatically
export default {
  async queue(batch: MessageBatch, env: Env): Promise<void> {
    for (const msg of batch.messages) {
      try {
        await riskyOperation(msg.body);
        msg.ack();
      } catch (error) {
        console.error(`Failed after ${msg.attempts} attempts:`, error);
      }
    }
  }
};

// DLQ consumer: Log and store failed messages
export default {
  async queue(batch: MessageBatch, env: Env): Promise<void> {
    for (const msg of batch.messages) {
      await env.FAILED_KV.put(msg.id, JSON.stringify(msg.body));
      msg.ack();
    }
  }
};
```

## Priority Queues

High priority: `max_batch_size: 5, max_batch_timeout: 1`. Low priority: `max_batch_size: 100, max_batch_timeout: 30`.

## Delayed Job Processing

```typescript
await env.EMAIL_QUEUE.send({ to, template, userId }, { delaySeconds: 3600 });
```

## Fan-out Pattern

```typescript
async fetch(request: Request, env: Env): Promise<Response> {
  const event = await request.json();
  
  // Send to multiple queues for parallel processing
  await Promise.all([
    env.ANALYTICS_QUEUE.send(event),
    env.NOTIFICATIONS_QUEUE.send(event),
    env.AUDIT_LOG_QUEUE.send(event)
  ]);
  
  return Response.json({ status: 'processed' });
}
```

## Idempotency Pattern

```typescript
async queue(batch: MessageBatch, env: Env): Promise<void> {
  for (const msg of batch.messages) {
    // Check if already processed
    const processed = await env.PROCESSED_KV.get(msg.id);
    if (processed) {
      msg.ack();
      continue;
    }
    
    await processMessage(msg.body);
    await env.PROCESSED_KV.put(msg.id, '1', { expirationTtl: 86400 });
    msg.ack();
  }
}
```

## Integration: D1 Batch Writes

```typescript
async queue(batch: MessageBatch, env: Env): Promise<void> {
  // Collect all inserts for single D1 batch
  const statements = batch.messages.map(msg => 
    env.DB.prepare('INSERT INTO events (id, data, created) VALUES (?, ?, ?)')
      .bind(msg.id, JSON.stringify(msg.body), Date.now())
  );
  
  try {
    await env.DB.batch(statements);
    batch.ackAll();
  } catch (error) {
    console.error('D1 batch failed:', error);
    batch.retryAll({ delaySeconds: 60 });
  }
}
```

## Integration: Workflows

```typescript
// Queue triggers Workflow for long-running tasks
async queue(batch: MessageBatch, env: Env): Promise<void> {
  for (const msg of batch.messages) {
    try {
      const instance = await env.MY_WORKFLOW.create({
        id: msg.id,
        params: msg.body
      });
      console.log('Workflow started:', instance.id);
      msg.ack();
    } catch (error) {
      msg.retry({ delaySeconds: 30 });
    }
  }
}
```

## Integration: Durable Objects

```typescript
// Queue distributes work to Durable Objects by ID
async queue(batch: MessageBatch, env: Env): Promise<void> {
  for (const msg of batch.messages) {
    const { userId, action } = msg.body;
    
    // Route to user-specific DO
    const id = env.USER_DO.idFromName(userId);
    const stub = env.USER_DO.get(id);
    
    try {
      await stub.fetch(new Request('https://do/process', {
        method: 'POST',
        body: JSON.stringify({ action, messageId: msg.id })
      }));
      msg.ack();
    } catch (error) {
      msg.retry({ delaySeconds: 60 });
    }
  }
}
```

## When to use

Use when the user asks about or needs: Queues Patterns & Best Practices.
﻿---
name: Set Queue Priority for Workflows
description: ## Set Queue Priority for Workflows
 
 Enable priority on a queue to process higher-priority workflows first. Lower numbers indicate higher priority.
tags: queue, priority, ordering, importance
---

## Set Queue Priority for Workflows

Enable priority on a queue to process higher-priority workflows first. Lower numbers indicate higher priority.

### Incorrect (no priority - FIFO only)

```typescript
const queue = new WorkflowQueue("tasks");
// All tasks processed in FIFO order regardless of importance
```

#### Correct (priority-enabled queue)

```typescript
const queue = new WorkflowQueue("tasks", { priorityEnabled: true });

async function processTaskFn(task: string) {
  // ...
}
const processTask = DBOS.registerWorkflow(processTaskFn);

// High priority task (lower number = higher priority)
await DBOS.startWorkflow(processTask, {
  queueName: queue.name,
  enqueueOptions: { priority: 1 },
})("urgent-task");

// Low priority task
await DBOS.startWorkflow(processTask, {
  queueName: queue.name,
  enqueueOptions: { priority: 100 },
})("background-task");
```

Priority rules:

- Range: `1` to `2,147,483,647`
- Lower number = higher priority
- Workflows **without** assigned priorities have the highest priority (run first)
- Workflows with the same priority are dequeued in FIFO order

Reference: [Priority](https://docs.dbos.dev/typescript/tutorials/queue-tutorial#priority)

## When to use

Use when the user asks about or needs: Set Queue Priority for Workflows.
﻿---
name: Set Workflow Timeouts
description: ## Set Workflow Timeouts
 
 Set a timeout for a workflow by passing `timeoutMS` to `DBOS.startWorkflow`. When the timeout expires, the workflow and all its children are cancelled.
tags: workflow, timeout, cancellation, duration
---

## Set Workflow Timeouts

Set a timeout for a workflow by passing `timeoutMS` to `DBOS.startWorkflow`. When the timeout expires, the workflow and all its children are cancelled.

### Incorrect (no timeout for potentially long workflow)

```typescript
// No timeout - could run indefinitely
const handle = await DBOS.startWorkflow(processTask)("data");
```

#### Correct (with timeout)

```typescript
async function processTaskFn(data: string) {
  // ...
}
const processTask = DBOS.registerWorkflow(processTaskFn);

// Timeout after 5 minutes (in milliseconds)
const handle = await DBOS.startWorkflow(processTask, {
  timeoutMS: 5 * 60 * 1000,
})("data");
```

Key timeout behaviors:

- Timeouts are **start-to-completion**: the timeout begins when the workflow starts execution, not when it's enqueued
- Timeouts are **durable**: they persist across restarts, so workflows can have very long timeouts (hours, days, weeks)
- Cancellation happens at the **beginning of the next step** - the current step completes first
- Cancelling a workflow also cancels all **child workflows**

Reference: [Workflow Timeouts](https://docs.dbos.dev/typescript/tutorials/workflow-tutorial#workflow-timeouts)

## When to use

Use when the user asks about or needs: Set Workflow Timeouts.
﻿---
name: Start Workflows in Background
description: ## Start Workflows in Background
 
 Use `DBOS.startWorkflow` to start a workflow in the background and get a handle to track it. The workflow is guaranteed to run to completion even if the app is interrupted.
tags: workflow, background, handle, async, waitFirst
---

## Start Workflows in Background

Use `DBOS.startWorkflow` to start a workflow in the background and get a handle to track it. The workflow is guaranteed to run to completion even if the app is interrupted.

### Incorrect (no way to track background work)

```typescript
async function processDataFn(data: string) {
  // ...
}
const processData = DBOS.registerWorkflow(processDataFn);

// Fire and forget - no way to track or get result
processData(data);
```

#### Correct (using startWorkflow)

```typescript
async function processDataFn(data: string) {
  return "processed: " + data;
}
const processData = DBOS.registerWorkflow(processDataFn);

async function main() {
  // Start workflow in background, get handle
  const handle = await DBOS.startWorkflow(processData)("input");

  // Get the workflow ID
  console.log(handle.workflowID);

  // Wait for result
  const result = await handle.getResult();

  // Check status
  const status = await handle.getStatus();
}
```

Retrieve a handle later by workflow ID:

```typescript
const handle = DBOS.retrieveWorkflow<string>(workflowID);
const result = await handle.getResult();
```

### Waiting for the First of Multiple Workflows

Use `DBOS.waitFirst` to race multiple concurrent workflows and process results as they complete:

```typescript
const handles = await Promise.all(
  items.map((item) => DBOS.startWorkflow(processItem)(item))
);

// Wait for whichever finishes first
const firstDone = await DBOS.waitFirst(handles);
const result = await firstDone.getResult();
```

`waitFirst` takes a non-empty array of `WorkflowHandle` and throws if the array is empty.

Reference: [Starting Workflows in Background](https://docs.dbos.dev/typescript/tutorials/workflow-tutorial#starting-workflows-in-the-background)

## When to use

Use when the user asks about or needs: Start Workflows in Background.
﻿---
name: Use Async Workflows Correctly
description: ## Use Async Workflows Correctly
 
 Coroutine (async) functions can be DBOS workflows. Use async-specific methods and patterns.
tags: async, coroutine, await, asyncio
---

## Use Async Workflows Correctly

Coroutine (async) functions can be DBOS workflows. Use async-specific methods and patterns.

### Incorrect (mixing sync and async)

```python
@DBOS.workflow()
async def async_workflow():
    # Don't use sync sleep in async workflow!
    DBOS.sleep(10)

    # Don't use sync start_workflow for async workflows
    handle = DBOS.start_workflow(other_async_workflow)
```

#### Correct (async patterns)

```python
import asyncio
import aiohttp

@DBOS.step()
async def fetch_async():
    async with aiohttp.ClientSession() as session:
        async with session.get("https://example.com") as response:
            return await response.text()

@DBOS.workflow()
async def async_workflow():
    # Use async sleep
    await DBOS.sleep_async(10)

    # Await async steps
    result = await fetch_async()

    # Use async start_workflow
    handle = await DBOS.start_workflow_async(other_async_workflow)

    return result
```

### Running Async Steps In Parallel

You can run async steps in parallel if they are started in **deterministic order**:

#### Correct (deterministic start order)

```python
@DBOS.workflow()
async def parallel_workflow():
    # Start steps in deterministic order, then await together
    tasks = [
        asyncio.create_task(step1("arg1")),
        asyncio.create_task(step2("arg2")),
        asyncio.create_task(step3("arg3")),
    ]
    # Use return_exceptions=True for proper error handling
    results = await asyncio.gather(*tasks, return_exceptions=True)
    return results
```

#### Incorrect (non-deterministic order)

```python
@DBOS.workflow()
async def bad_parallel_workflow():
    async def seq_a():
        await step1("arg1")
        await step2("arg2")  # Order depends on step1 timing

    async def seq_b():
        await step3("arg3")
        await step4("arg4")  # Order depends on step3 timing

    # step2 and step4 may run in either order - non-deterministic!
    await asyncio.gather(seq_a(), seq_b())
```

If you need concurrent sequences, use child workflows instead of interleaving steps.

For transactions in async workflows, use `asyncio.to_thread`:

```python
@DBOS.transaction()
def sync_transaction(data):
    DBOS.sql_session.execute(...)

@DBOS.workflow()
async def async_workflow():
    result = await asyncio.to_thread(sync_transaction, data)
```

Reference: [Async Workflows](https://docs.dbos.dev/python/tutorials/workflow-tutorial#coroutine-async-workflows)

## When to use

Use when the user asks about or needs: Use Async Workflows Correctly.
﻿---
name: Use Events for Workflow Status Publishing
description: ## Use Events for Workflow Status Publishing
 
 Workflows can publish events (key-value pairs) with `DBOS.setEvent`. Other code can read events with `DBOS.getEvent`. Events are persisted and useful for real-time progress monitoring.
tags: communication, events, status, key-value
---

## Use Events for Workflow Status Publishing

Workflows can publish events (key-value pairs) with `DBOS.setEvent`. Other code can read events with `DBOS.getEvent`. Events are persisted and useful for real-time progress monitoring.

### Incorrect (using external state for progress)

```typescript
let progress = 0; // Global variable - not durable!

async function processDataFn() {
  progress = 50; // Not persisted, lost on restart
}
const processData = DBOS.registerWorkflow(processDataFn);
```

#### Correct (using events)

```typescript
async function processDataFn() {
  await DBOS.setEvent("status", "processing");
  await DBOS.runStep(stepOne, { name: "stepOne" });
  await DBOS.setEvent("progress", 50);
  await DBOS.runStep(stepTwo, { name: "stepTwo" });
  await DBOS.setEvent("progress", 100);
  await DBOS.setEvent("status", "complete");
}
const processData = DBOS.registerWorkflow(processDataFn);

// Read events from outside the workflow
const status = await DBOS.getEvent<string>(workflowID, "status", 0);
const progress = await DBOS.getEvent<number>(workflowID, "progress", 0);
// Returns null if the event doesn't exist within the timeout (default 60s)
```

Events are useful for interactive workflows. For example, a checkout workflow can publish a payment URL for the caller to redirect to:

```typescript
async function checkoutWorkflowFn() {
  const paymentURL = await DBOS.runStep(createPayment, { name: "createPayment" });
  await DBOS.setEvent("paymentURL", paymentURL);
  // Continue processing...
}
const checkoutWorkflow = DBOS.registerWorkflow(checkoutWorkflowFn);

// HTTP handler starts workflow and reads the payment URL
const handle = await DBOS.startWorkflow(checkoutWorkflow)();
const url = await DBOS.getEvent<string>(handle.workflowID, "paymentURL", 300);
```

Reference: [Workflow Events](https://docs.dbos.dev/typescript/tutorials/workflow-communication#workflow-events)

## When to use

Use when the user asks about or needs: Use Events for Workflow Status Publishing.
﻿---
name: Use Messages for Workflow Notifications
description: ## Use Messages for Workflow Notifications
 
 Use `DBOS.send` to send messages to a workflow and `DBOS.recv` to receive them. Messages are queued per topic and persisted for reliable delivery.
tags: communication, messages, send, recv, notification
---

## Use Messages for Workflow Notifications

Use `DBOS.send` to send messages to a workflow and `DBOS.recv` to receive them. Messages are queued per topic and persisted for reliable delivery.

### Incorrect (using external messaging for workflow communication)

```typescript
// External message queue is not integrated with workflow recovery
import { Queue } from "some-external-queue";
```

#### Correct (using DBOS messages)

```typescript
async function checkoutWorkflowFn() {
  // Wait for payment notification (timeout 120 seconds)
  const notification = await DBOS.recv<string>("payment_status", 120);

  if (notification && notification === "paid") {
    await DBOS.runStep(fulfillOrder, { name: "fulfillOrder" });
  } else {
    await DBOS.runStep(cancelOrder, { name: "cancelOrder" });
  }
}
const checkoutWorkflow = DBOS.registerWorkflow(checkoutWorkflowFn);

// Send a message from a webhook handler
async function paymentWebhook(workflowID: string, status: string) {
  await DBOS.send(workflowID, status, "payment_status");
}
```

Key behaviors:

- `recv` waits for and consumes the next message for the specified topic
- Returns `null` if the wait times out (default timeout: 60 seconds)
- Messages without a topic can only be received by `recv` without a topic
- Messages are queued per-topic (FIFO)

#### Reliability guarantees

- All messages are persisted to the database
- Messages sent from workflows are delivered exactly-once
- Messages sent from non-workflow code can use an idempotency key:

```typescript
await DBOS.send(workflowID, message, "topic", "idempotency-key-123");
```

Reference: [Workflow Messaging](https://docs.dbos.dev/typescript/tutorials/workflow-communication#workflow-messaging-and-notifications)

## When to use

Use when the user asks about or needs: Use Messages for Workflow Notifications.
﻿---
name: Use Patching for Safe Workflow Upgrades
description: ## Use Patching for Safe Workflow Upgrades
 
 Use `DBOS.patch()` to safely deploy breaking changes to workflow code. Breaking changes alter which steps run or their order, which can cause recovery failures.
tags: advanced, patching, upgrade, breaking-change
---

## Use Patching for Safe Workflow Upgrades

Use `DBOS.patch()` to safely deploy breaking changes to workflow code. Breaking changes alter which steps run or their order, which can cause recovery failures.

### Incorrect (breaking change without patching)

```typescript
// BEFORE: original workflow
async function workflowFn() {
  await foo();
  await bar();
}
const workflow = DBOS.registerWorkflow(workflowFn);

// AFTER: breaking change - recovery will fail for in-progress workflows!
async function workflowFn() {
  await baz(); // Changed step
  await bar();
}
const workflow = DBOS.registerWorkflow(workflowFn);
```

#### Correct (using patch)

```typescript
async function workflowFn() {
  if (await DBOS.patch("use-baz")) {
    await baz(); // New workflows run this
  } else {
    await foo(); // Old workflows continue with original code
  }
  await bar();
}
const workflow = DBOS.registerWorkflow(workflowFn);
```

`DBOS.patch()` returns `true` for new workflows and `false` for workflows that started before the patch.

#### Deprecating patches (after all old workflows complete)

```typescript
async function workflowFn() {
  if (await DBOS.deprecatePatch("use-baz")) { // Always returns true
    await baz();
  }
  await bar();
}
const workflow = DBOS.registerWorkflow(workflowFn);
```

#### Removing patches (after all workflows using deprecatePatch complete)

```typescript
async function workflowFn() {
  await baz();
  await bar();
}
const workflow = DBOS.registerWorkflow(workflowFn);
```

Lifecycle: `patch()` → deploy → wait for old workflows → `deprecatePatch()` → deploy → wait → remove patch entirely.

Use `DBOS.listWorkflows` to check for active old workflows before deprecating or removing patches.

Reference: [Patching](https://docs.dbos.dev/typescript/tutorials/upgrading-workflows#patching)

## When to use

Use when the user asks about or needs: Use Patching for Safe Workflow Upgrades.
﻿---
name: Use Queues for Concurrent Workflows
description: ## Use Queues for Concurrent Workflows
 
 Queues run many workflows concurrently with managed flow control. Use them when you need to control how many workflows run at once.
tags: queue, concurrency, enqueue, workflow
---

## Use Queues for Concurrent Workflows

Queues run many workflows concurrently with managed flow control. Use them when you need to control how many workflows run at once.

### Incorrect (uncontrolled concurrency)

```typescript
async function processTaskFn(task: string) {
  // ...
}
const processTask = DBOS.registerWorkflow(processTaskFn);

// Starting many workflows without control - could overwhelm resources
for (const task of tasks) {
  await DBOS.startWorkflow(processTask)(task);
}
```

#### Correct (using a queue)

```typescript
import { DBOS, WorkflowQueue } from "@dbos-inc/dbos-sdk";

const queue = new WorkflowQueue("task_queue");

async function processTaskFn(task: string) {
  // ...
}
const processTask = DBOS.registerWorkflow(processTaskFn);

async function processAllTasksFn(tasks: string[]) {
  const handles = [];
  for (const task of tasks) {
    // Enqueue by passing queueName to startWorkflow
    const handle = await DBOS.startWorkflow(processTask, {
      queueName: queue.name,
    })(task);
    handles.push(handle);
  }
  // Wait for all tasks
  const results = [];
  for (const h of handles) {
    results.push(await h.getResult());
  }
  return results;
}
const processAllTasks = DBOS.registerWorkflow(processAllTasksFn);
```

Queues process workflows in FIFO order. All queues should be created before `DBOS.launch()`.

Reference: [DBOS Queues](https://docs.dbos.dev/typescript/tutorials/queue-tutorial)

## When to use

Use when the user asks about or needs: Use Queues for Concurrent Workflows.
﻿---
name: Use Workflow IDs for Idempotency
description: ## Use Workflow IDs for Idempotency
 
 Assign a workflow ID to ensure a workflow executes only once, even if called multiple times. This prevents duplicate side effects like double payments.
tags: pattern, idempotency, workflow-id, deduplication
---

## Use Workflow IDs for Idempotency

Assign a workflow ID to ensure a workflow executes only once, even if called multiple times. This prevents duplicate side effects like double payments.

### Incorrect (no idempotency)

```typescript
async function processPaymentFn(orderId: string, amount: number) {
  await DBOS.runStep(() => chargeCard(amount), { name: "chargeCard" });
  await DBOS.runStep(() => updateOrder(orderId), { name: "updateOrder" });
}
const processPayment = DBOS.registerWorkflow(processPaymentFn);

// Multiple calls could charge the card multiple times!
await processPayment("order-123", 50);
await processPayment("order-123", 50); // Double charge!
```

#### Correct (with workflow ID)

```typescript
async function processPaymentFn(orderId: string, amount: number) {
  await DBOS.runStep(() => chargeCard(amount), { name: "chargeCard" });
  await DBOS.runStep(() => updateOrder(orderId), { name: "updateOrder" });
}
const processPayment = DBOS.registerWorkflow(processPaymentFn);

// Same workflow ID = only one execution
const workflowID = `payment-${orderId}`;
await DBOS.startWorkflow(processPayment, { workflowID })("order-123", 50);
await DBOS.startWorkflow(processPayment, { workflowID })("order-123", 50);
// Second call returns the result of the first execution
```

Access the current workflow ID inside a workflow:

```typescript
async function myWorkflowFn() {
  const currentID = DBOS.workflowID;
  console.log(`Running workflow: ${currentID}`);
}
```

Workflow IDs must be **globally unique** for your application. If not set, a random UUID is generated.

Reference: [Workflow IDs and Idempotency](https://docs.dbos.dev/typescript/tutorials/workflow-tutorial#workflow-ids-and-idempotency)

## When to use

Use when the user asks about or needs: Use Workflow IDs for Idempotency.
﻿---
name: Workflow Configuration
description: # Workflow Configuration
 
 ## wrangler.jsonc Setup
---

# Workflow Configuration

## wrangler.jsonc Setup (Workflow Configuration)

```jsonc
{
  "name": "my-worker",
  "main": "src/index.ts",
  "compatibility_date": "2025-01-01",  // Use current date for new projects
  "observability": {
    "enabled": true  // Enables Workflows dashboard + structured logs
  },
  "workflows": [
    {
      "name": "my-workflow",           // Workflow name
      "binding": "MY_WORKFLOW",        // Env binding
      "class_name": "MyWorkflow"      // TS class name
      // "script_name": "other-worker" // For cross-script calls
    }
  ],
  "limits": {
    "cpu_ms": 300000  // 5 min max (default 30s)
  }
}
```

## Step Configuration

```typescript
// Basic step
const data = await step.do('step name', async () => ({ result: 'value' }));

// With retry config
await step.do('api call', {
  retries: {
    limit: 10,              // Default: 5, or Infinity
    delay: '10 seconds',    // Default: 10000ms
    backoff: 'exponential'  // constant | linear | exponential
  },
  timeout: '30 minutes'     // Per-attempt timeout (default: 10min)
}, async () => {
  const res = await fetch('https://api.example.com/data');
  if (!res.ok) throw new Error('Failed');
  return res.json();
});
```

### Parallel Steps

```typescript
const [user, settings] = await Promise.all([
  step.do('fetch user', async () => this.env.KV.get(`user:${id}`)),
  step.do('fetch settings', async () => this.env.KV.get(`settings:${id}`))
]);
```

### Conditional Steps

```typescript
const config = await step.do('fetch config', async () => 
  this.env.KV.get('flags', { type: 'json' })
);

// ✅ Deterministic (based on step output)
if (config.enableEmail) {
  await step.do('send email', async () => sendEmail());
}

// ❌ Non-deterministic (Date.now outside step)
if (Date.now() > deadline) { /* BAD */ }
```

### Dynamic Steps (Loops)

```typescript
const files = await step.do('list files', async () => 
  this.env.BUCKET.list()
);

for (const file of files.objects) {
  await step.do(`process ${file.key}`, async () => {
    const obj = await this.env.BUCKET.get(file.key);
    return processData(await obj.arrayBuffer());
  });
}
```

## Multiple Workflows

```jsonc
{
  "workflows": [
    {"name": "user-onboarding", "binding": "USER_ONBOARDING", "class_name": "UserOnboarding"},
    {"name": "data-processing", "binding": "DATA_PROCESSING", "class_name": "DataProcessing"}
  ]
}
```

Each class extends `WorkflowEntrypoint` with its own `Params` type.

## Cross-Script Bindings

Worker A defines workflow. Worker B calls it by adding `script_name`:

```jsonc
// Worker B (caller)
{
  "workflows": [{
    "name": "billing-workflow",
    "binding": "BILLING",
    "script_name": "billing-worker"  // Points to Worker A
  }]
}
```

## Bindings

Workflows access Cloudflare bindings via `this.env`:

```typescript
type Env = {
  MY_WORKFLOW: Workflow;
  KV: KVNamespace;
  DB: D1Database;
  BUCKET: R2Bucket;
  AI: Ai;
  VECTORIZE: VectorizeIndex;
};

await step.do('use bindings', async () => {
  const kv = await this.env.KV.get('key');
  const db = await this.env.DB.prepare('SELECT * FROM users').first();
  const file = await this.env.BUCKET.get('file.txt');
  const ai = await this.env.AI.run('@cf/meta/llama-2-7b-chat-int8', { prompt: 'Hi' });
});
```

## Pages Functions Binding

Pages Functions can trigger Workflows via service bindings:

```typescript
// functions/_middleware.ts
export const onRequest: PagesFunction<Env> = async ({ env, request }) => {
  const instance = await env.MY_WORKFLOW.create({
    params: { url: request.url }
  });
  return new Response(`Started ${instance.id}`);
};
```

Configure in wrangler.jsonc under `service_bindings`.

See: [api.md](./api.md), [patterns.md](./patterns.md)

## When to use

Use when the user asks about or needs: Workflow Configuration.
﻿---
name: Workflow Patterns
description: # Workflow Patterns
 
 ## Image Processing Pipeline
---

# Workflow Patterns

## Image Processing Pipeline (Workflow Patterns)

```typescript
export class ImageProcessingWorkflow extends WorkflowEntrypoint<Env, Params> {
  async run(event, step) {
    const imageData = await step.do('fetch', async () => (await this.env.BUCKET.get(event.params.imageKey)).arrayBuffer());
    const description = await step.do('generate description', async () => 
      await this.env.AI.run('@cf/llava-hf/llava-1.5-7b-hf', {image: Array.from(new Uint8Array(imageData)), prompt: 'Describe this image', max_tokens: 50})
    );
    await step.waitForEvent('await approval', { event: 'approved', timeout: '24h' });
    await step.do('publish', async () => await this.env.BUCKET.put(`public/${event.params.imageKey}`, imageData));
  }
}
```

## User Lifecycle

```typescript
export class UserLifecycleWorkflow extends WorkflowEntrypoint<Env, Params> {
  async run(event, step) {
    await step.do('welcome email', async () => await sendEmail(event.params.email, 'Welcome!'));
    await step.sleep('trial period', '7 days');
    const hasConverted = await step.do('check conversion', async () => {
      const user = await this.env.DB.prepare('SELECT subscription_status FROM users WHERE id = ?').bind(event.params.userId).first();
      return user.subscription_status === 'active';
    });
    if (!hasConverted) await step.do('trial expiration email', async () => await sendEmail(event.params.email, 'Trial ending'));
  }
}
```

## Data Pipeline

```typescript
export class DataPipelineWorkflow extends WorkflowEntrypoint<Env, Params> {
  async run(event, step) {
    const rawData = await step.do('extract', {retries: { limit: 10, delay: '30s', backoff: 'exponential' }}, async () => {
      const res = await fetch(event.params.sourceUrl);
      if (!res.ok) throw new Error('Fetch failed');
      return res.json();
    });
    const transformed = await step.do('transform', async () => 
      rawData.map(item => ({ id: item.id, normalized: normalizeData(item) }))
    );
    const dataRef = await step.do('store', async () => {
      const key = `processed/${Date.now()}.json`;
      await this.env.BUCKET.put(key, JSON.stringify(transformed));
      return { key };
    });
    await step.do('load', async () => {
      const data = await (await this.env.BUCKET.get(dataRef.key)).json();
      for (let i = 0; i < data.length; i += 100) {
        await this.env.DB.batch(data.slice(i, i + 100).map(item => 
          this.env.DB.prepare('INSERT INTO records VALUES (?, ?)').bind(item.id, item.normalized)
        ));
      }
    });
  }
}
```

## Human-in-the-Loop Approval

```typescript
export class ApprovalWorkflow extends WorkflowEntrypoint<Env, Params> {
  async run(event, step) {
    await step.do('create approval', async () => await this.env.DB.prepare('INSERT INTO approvals (id, user_id, status) VALUES (?, ?, ?)').bind(event.instanceId, event.params.userId, 'pending').run());
    try {
      const approval = await step.waitForEvent<{ approved: boolean }>('wait for approval', { event: 'approval-response', timeout: '48h' });
      if (approval.approved) { await step.do('process approval', async () => {}); } 
      else { await step.do('handle rejection', async () => {}); }
    } catch (e) {
      await step.do('auto reject', async () => await this.env.DB.prepare('UPDATE approvals SET status = ? WHERE id = ?').bind('auto-rejected', event.instanceId).run());
    }
  }
}
```

## Testing Workflows

### Setup

```typescript
// vitest.config.ts
import { defineWorkersConfig } from '@cloudflare/vitest-pool-workers/config';

export default defineWorkersConfig({
  test: {
    poolOptions: {
      workers: {
        wrangler: { configPath: './wrangler.jsonc' }
      }
    }
  }
});
```

### Introspection API

```typescript
import { introspectWorkflowInstance } from 'cloudflare:test';

const instance = await env.MY_WORKFLOW.create({ params: { userId: '123' } });
const introspector = await introspectWorkflowInstance(env.MY_WORKFLOW, instance.id);

// Wait for step completion
const result = await introspector.waitForStepResult({ name: 'fetch user', index: 0 });

// Mock step behavior
await introspector.modify(async (m) => {
  await m.mockStepResult({ name: 'api call' }, { mocked: true });
});
```

## Best Practices

### ✅ DO

1. **Granular steps**: One API call per step (unless proving idempotency)
2. **Idempotency**: Check-then-execute; use idempotency keys
3. **Deterministic names**: Use static or step-output-based names
4. **Return state**: Persist via step returns, not variables
5. **Always await**: `await step.do()`, avoid dangling promises
6. **Deterministic conditionals**: Base on `event.payload` or step outputs
7. **Store large data externally**: R2/KV for >1 MiB, return refs
8. **Batch creation**: `createBatch()` for multiple instances

### ❌ DON'T

1. **One giant step**: Breaks durability & retry control
2. **State outside steps**: Lost on hibernation
3. **Mutate events**: Events immutable, return new state
4. **Non-deterministic logic outside steps**: `Math.random()`, `Date.now()` must be in steps
5. **Side effects outside steps**: May duplicate on restart
6. **Non-deterministic step names**: Prevents caching
7. **Ignore timeouts**: `waitForEvent` throws, use try-catch
8. **Reuse instance IDs**: Must be unique within retention

## Orchestration Patterns

### Fan-Out (Parallel Processing)

```typescript
const files = await step.do('list', async () => this.env.BUCKET.list());
await Promise.all(files.objects.map((file, i) => step.do(`process ${i}`, async () => processFile(await (await this.env.BUCKET.get(file.key)).arrayBuffer()))));
```

### Parent-Child Workflows

```typescript
const child = await step.do('start child', async () => await this.env.CHILD_WORKFLOW.create({id: `child-${event.instanceId}`, params: { data: result.data }}));
await step.do('other work', async () => console.log(`Child started: ${child.id}`));
```

### Race Pattern

```typescript
const winner = await Promise.race([
  step.do('option A', async () => slowOperation()),
  step.do('option B', async () => fastOperation())
]);
```

### Scheduled Workflow Chain

```typescript
export default { async scheduled(event, env) { await env.DAILY_WORKFLOW.create({id: `daily-${event.scheduledTime}`, params: { timestamp: event.scheduledTime }}); }};
export class DailyWorkflow extends WorkflowEntrypoint<Env, Params> {
  async run(event, step) {
    await step.do('daily task', async () => {});
    await step.sleep('wait 7 days', '7 days');
    await step.do('weekly followup', async () => {});
  }
}
```

See: [configuration.md](./configuration.md), [api.md](./api.md), [gotchas.md](./gotchas.md)

## When to use

Use when the user asks about or needs: Workflow Patterns.
