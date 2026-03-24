---
name: workflow-orchestration
description: Workflow orchestration covering Temporal (workflows, activities, signals, queries, child workflows, saga pattern, versioning, testing) and data pipeline orchestration with Airflow/Prefect/Dagster (DAGs, scheduling, retries, sensors, testing). Use when building durable workflow systems or orchestrated data pipelines.
domain: infrastructure
tags: [temporal, airflow, prefect, dagster, workflow, orchestration, saga, durable-execution, pipeline-orchestration]
triggers: temporal, airflow, prefect, dagster, workflow orchestration, durable execution, saga pattern, pipeline scheduling
---


# Temporal.io Durable Workflows

## When to Use

Temporal is the right tool when you need code that survives infrastructure failures mid-execution. It is not a message broker and not a cron scheduler (though it can replace both). Reach for it when:

- **Long-running processes** span minutes, hours, days, or years — order fulfillment, document review pipelines, multi-step onboarding flows.
- **Saga orchestration** requires coordinated rollbacks across multiple services. Temporal makes compensation logic explicit and auditable.
- **Reliable async** — you want guaranteed execution without manually managing queues, retries, dead-letter topics, or idempotency keys.
- **Human-in-the-loop** workflows require pausing for external approval or input via signals before continuing.

Do not use Temporal when a simple queue consumer or a fire-and-forget HTTP call is sufficient. It adds operational overhead (a Temporal cluster) and requires deterministic code, which demands discipline.

**vs. queues (SQS, Kafka):** Queues hand off individual messages; the caller has no visibility into multi-step progress. Temporal tracks full execution state and handles retries with backoff automatically.

**vs. cron:** Cron fires and forgets. Temporal Schedules record every execution, handle overlap policies, support backfill, and feed into observable workflow histories.

**vs. Airflow:** Airflow is a DAG scheduler optimized for data pipelines with a Python-centric DSL. Temporal is a general-purpose durable execution engine that works across Go, TypeScript, Python, Java, and .NET, with no DAG constraint.

## Core Concepts

| Concept | Definition |
| --- | --- |
| **Workflow Definition** | A function containing your business logic. Must be deterministic. |
| **Workflow Execution** | A running instance of a Workflow Definition, identified by a Workflow ID. |
| **Activity** | A function that performs a single side-effectful operation (HTTP call, DB write, file I/O). Has no determinism constraint. |
| **Worker** | A process you run. It polls a Task Queue and executes Workflow and Activity code. |
| **Task Queue** | A named queue on the Temporal server. Workers subscribe to it; clients dispatch work to it. Multiple workers on the same queue provide load balancing. |
| **Namespace** | Isolation boundary (like a database schema). Separate namespaces for prod, staging, team boundaries. |
| **Event History** | Append-only log of every event in a Workflow Execution. This is the source of truth for replay. |

**Determinism constraint:** Workflow code is replayed from Event History every time a Worker picks up a task. Any non-deterministic code — `time.Now()`, `rand`, direct network calls, reading environment variables mid-workflow — will produce different output on replay and cause a non-deterministic error. Replace all non-deterministic operations:

- Time: use `workflow.Now(ctx)` / `workflow.Sleep(ctx, d)`, not `time.Now()` / `time.Sleep()`
- Random: seed from workflow parameters, not `rand.New(rand.NewSource(time.Now().UnixNano()))`
- Side effects: always inside Activities, never directly in Workflow code
- Goroutines: use `workflow.Go(ctx, fn)`, not `go fn()`

## Workflow Definition

### Go

```go
// Input/output structs — always use structs, never bare primitives.
// This allows safe schema evolution without breaking running executions.
type ProcessOrderInput struct {
    OrderID    string
    CustomerID string
    Items      []OrderItem
}

type ProcessOrderOutput struct {
    Status         string
    TrackingNumber string
}

func ProcessOrderWorkflow(ctx workflow.Context, input ProcessOrderInput) (ProcessOrderOutput, error) {
    logger := workflow.GetLogger(ctx)

    // ActivityOptions scoped to this call; override per-activity as needed.
    activityOpts := workflow.ActivityOptions{
        StartToCloseTimeout: 30 * time.Second,
        RetryPolicy: &temporal.RetryPolicy{
            MaximumAttempts: 5,
        },
    }
    ctx = workflow.WithActivityOptions(ctx, activityOpts)

    var chargeResult ChargeResult
    if err := workflow.ExecuteActivity(ctx, ChargePayment, ChargeInput{
        OrderID:    input.OrderID,
        CustomerID: input.CustomerID,
    }).Get(ctx, &chargeResult); err != nil {
        return ProcessOrderOutput{}, err
    }

    // workflow.Sleep is deterministic; time.Sleep is not.
    _ = workflow.Sleep(ctx, 2*time.Second)

    var shipResult ShipResult
    if err := workflow.ExecuteActivity(ctx, ShipOrder, ShipInput{
        OrderID: input.OrderID,
        Items:   input.Items,
    }).Get(ctx, &shipResult); err != nil {
        return ProcessOrderOutput{}, err
    }

    logger.Info("Order processed", "orderID", input.OrderID)
    return ProcessOrderOutput{
        Status:         "shipped",
        TrackingNumber: shipResult.TrackingNumber,
    }, nil
}
```

### TypeScript

```typescript
import * as wf from '@temporalio/workflow';
import type { Activities } from './activities';

const { chargePayment, shipOrder } = wf.proxyActivities<Activities>({
  startToCloseTimeout: '30s',
  retry: { maximumAttempts: 5 },
});

export interface ProcessOrderInput {
  orderId: string;
  customerId: string;
  items: OrderItem[];
}

export async function processOrderWorkflow(input: ProcessOrderInput): Promise<{ trackingNumber: string }> {
  await chargePayment({ orderId: input.orderId, customerId: input.customerId });

  // deterministic sleep
  await wf.sleep('2s');

  const { trackingNumber } = await shipOrder({ orderId: input.orderId, items: input.items });
  return { trackingNumber };
}
```

#### Determinism rules summary

- Never call `Date.now()`, `Math.random()`, `fetch()`, or Node I/O directly in a workflow function.
- Never use `Promise.race` on timers you control outside `wf.sleep` or `wf.condition`.
- Import activities via `proxyActivities`; never import and call them directly.

## Activity Definition

Activities are plain functions with no Temporal-specific constraints. They can do anything: HTTP calls, DB writes, file I/O. The key disciplines:

**Idempotency** — Activities retry on failure. Design them so running twice has the same observable result as running once. Before creating a resource, check if it already exists. Treat "not found" as success for deletes. Use idempotency keys for payment APIs.

**Heartbeating** — For activities longer than ~10 seconds, emit heartbeats so the Temporal server knows the worker is still alive. Cancellations are only delivered at heartbeat points.

### Go (Activity Definition)

```go
func ShipOrder(ctx context.Context, input ShipInput) (ShipResult, error) {
    logger := activity.GetLogger(ctx)

    for i, item := range input.Items {
        // Heartbeat on each iteration; passes progress state that survives retries.
        activity.RecordHeartbeat(ctx, i)

        // Check for cancellation delivered at heartbeat.
        if ctx.Err() != nil {
            return ShipResult{}, ctx.Err()
        }

        if err := dispatchToWarehouse(item); err != nil {
            // Return the error; Temporal will retry per the RetryPolicy.
            return ShipResult{}, fmt.Errorf("dispatch item %s: %w", item.SKU, err)
        }

        logger.Info("Dispatched item", "sku", item.SKU, "index", i)
    }

    tracking, err := generateTrackingNumber(input.OrderID)
    if err != nil {
        // Non-retryable: business logic failure, not a transient fault.
        return ShipResult{}, temporal.NewNonRetryableApplicationError(
            "tracking generation failed", "TrackingError", err, nil,
        )
    }

    return ShipResult{TrackingNumber: tracking}, nil
}
```

**HeartbeatTimeout** — set it to roughly 2× your expected heartbeat interval. If a worker dies, the server detects the missing heartbeat within this window and reschedules the activity.

```go
activityOpts := workflow.ActivityOptions{
    StartToCloseTimeout: 10 * time.Minute,
    HeartbeatTimeout:    30 * time.Second,
}
```

### TypeScript (Activity Definition)

```typescript
import { Context, heartbeat } from '@temporalio/activity';

export async function shipOrder(input: ShipInput): Promise<ShipResult> {
  for (let i = 0; i < input.items.length; i++) {
    // Deliver progress to heartbeat; retrievable on retry via Context.current().heartbeatDetails
    heartbeat(i);

    await dispatchToWarehouse(input.items[i]);
  }
  const trackingNumber = await generateTrackingNumber(input.orderId);
  return { trackingNumber };
}
```

On retry, recover heartbeat progress:

```typescript
const lastHeartbeat = Context.current().heartbeatDetails as number | undefined;
const startIndex = lastHeartbeat ?? 0;
```

## Retry Policies

Temporal retries Activities automatically by default. The default policy:

```text
InitialInterval    = 1s
BackoffCoefficient = 2.0        (exponential backoff)
MaximumInterval    = 100s       (caps growth at 100 × InitialInterval)
MaximumAttempts    = unlimited
NonRetryableErrors = []
```

Workflows do **not** retry by default.

### Go (Retry Policies)

```go
retryPolicy := &temporal.RetryPolicy{
    InitialInterval:        time.Second,
    BackoffCoefficient:     2.0,
    MaximumInterval:        100 * time.Second,
    MaximumAttempts:        0,    // 0 = unlimited
    NonRetryableErrorTypes: []string{"CreditCardError", "InsufficientFundsError"},
}

activityOpts := workflow.ActivityOptions{
    ScheduleToCloseTimeout: 24 * time.Hour, // outer deadline for all attempts
    StartToCloseTimeout:    30 * time.Second, // per-attempt deadline
    RetryPolicy:            retryPolicy,
}
```

Mark an error non-retryable from within an Activity:

```go
return temporal.NewNonRetryableApplicationError(
    "Credit Card Charge Error",
    "CreditCardError",
    nil,
    nil,
)
```

### TypeScript (Retry Policies)

```typescript
throw ApplicationFailure.create({
  message: `Invalid charge amount: ${chargeAmount}`,
  details: [chargeAmount],
  nonRetryable: true,
});
```

#### Timeout hierarchy

- `ScheduleToStartTimeout` — how long a task can wait in the queue before a worker picks it up. Only set this if the task becomes obsolete after a delay.
- `StartToCloseTimeout` — maximum duration of a single execution attempt. **Always set this.**
- `ScheduleToCloseTimeout` — end-to-end deadline including all retry attempts. Use this as your absolute outer bound.

Prefer `ScheduleToCloseTimeout` set to weeks/months over limiting `MaximumAttempts`, so transient outages don't exhaust retries unnecessarily.

## Signals and Queries

**Signals** send asynchronous external input to a running workflow. The workflow decides when to consume it.

**Queries** read workflow state synchronously from outside. The workflow registers a handler; clients call it at any time, even after completion.

**Updates** (newer API) combine signal + query: send input and get a synchronous result back. Use when the caller needs confirmation the workflow acted on the input.

### Go — Signal

```go
// In the workflow:
var approveInput ApproveInput
workflow.GetSignalChannel(ctx, "approve-signal").Receive(ctx, &approveInput)
// Execution blocks here until the signal arrives.

// From a client or another workflow:
err = temporalClient.SignalWorkflow(ctx, workflowID, runID, "approve-signal", ApproveInput{
    ApproverName: "alice",
})

// SignalWithStart — signal an existing workflow, or start a new one if none exists:
err = temporalClient.SignalWithStartWorkflow(ctx, workflowID, "approve-signal", ApproveInput{
    ApproverName: "alice",
}, client.StartWorkflowOptions{TaskQueue: "orders"}, OrderWorkflow, orderInput)
```

Listening to multiple signals concurrently with a Selector:

```go
signalCh := workflow.GetSignalChannel(ctx, "your-signal")
workflow.Go(ctx, func(ctx workflow.Context) {
    for {
        selector := workflow.NewSelector(ctx)
        selector.AddReceive(signalCh, func(c workflow.ReceiveChannel, more bool) {
            var sig MySignal
            c.Receive(ctx, &sig)
            // handle sig
        })
        selector.Select(ctx)
    }
})
```

### Go — Query

```go
// Register handler at workflow start (before any await/sleep).
err := workflow.SetQueryHandler(ctx, "get-status", func() (string, error) {
    return currentStatus, nil
})
if err != nil {
    return err
}

// From a client:
resp, err := temporalClient.QueryWorkflow(ctx, workflowID, runID, "get-status")
var status string
resp.Get(&status)
```

### TypeScript — Signal and Query

```typescript
// Define at module level (shared between workflow and client).
export const approveSignal = wf.defineSignal<[ApproveInput]>('approve');
export const getStatusQuery = wf.defineQuery<string>('getStatus');

// In the workflow function:
let approved = false;

wf.setHandler(approveSignal, (input: ApproveInput) => {
  approved = true;
  approverName = input.name;
});

wf.setHandler(getStatusQuery, () => currentStatus);

// Block until approved:
await wf.condition(() => approved);
```

## Child Workflows

Spawn a child workflow when you need to:

- Partition work to avoid Event History size limits (a workflow with 1M activities will bloat; spawn 1K children with 1K activities each instead).
- Route work to a different Task Queue / Worker pool (separate service).
- Create a one-to-one mapping with a resource and serialize operations against it.
- Run periodic logic via Continue-As-New inside the child without bloating the parent's history.

Default to Activities unless one of the above applies.

### Go (Child Workflows)

```go
childCtx := workflow.WithChildOptions(ctx, workflow.ChildWorkflowOptions{
    WorkflowID:        "child-" + parentID,
    TaskQueue:         "child-task-queue",
    ParentClosePolicy: enums.PARENT_CLOSE_POLICY_TERMINATE, // default: terminates child when parent closes
})

var childResult ChildResult
err := workflow.ExecuteChildWorkflow(childCtx, ChildWorkflow, childInput).Get(ctx, &childResult)
```

#### ParentClosePolicy options

- `PARENT_CLOSE_POLICY_TERMINATE` — child is terminated when parent completes/fails/cancels.
- `PARENT_CLOSE_POLICY_ABANDON` — child keeps running independently.
- `PARENT_CLOSE_POLICY_REQUEST_CANCEL` — child receives a cancellation request.

**Fan-out pattern** — spawn N children and collect results:

```go
futures := make([]workflow.Future, len(items))
for i, item := range items {
    childCtx := workflow.WithChildOptions(ctx, workflow.ChildWorkflowOptions{
        WorkflowID: fmt.Sprintf("process-item-%s", item.ID),
    })
    futures[i] = workflow.ExecuteChildWorkflow(childCtx, ProcessItemWorkflow, item)
}

results := make([]ItemResult, len(items))
for i, f := range futures {
    if err := f.Get(ctx, &results[i]); err != nil {
        return nil, err
    }
}
```

Do not spawn more than ~1,000 children from a single parent; each spawn adds events to the parent's history.

## Schedules

Schedules replace Temporal's older Cron Workflow feature and are more flexible. They live on the server; you do not need a workflow running to maintain them.

### Go — Create

```go
scheduleClient := temporalClient.ScheduleClient()

handle, err := scheduleClient.Create(ctx, client.ScheduleOptions{
    ID: "nightly-report",
    Spec: client.ScheduleSpec{
        CronExpressions: []string{"0 2 * * *"}, // 02:00 UTC daily
    },
    Action: &client.ScheduleWorkflowAction{
        ID:        "nightly-report-workflow",
        Workflow:  GenerateReportWorkflow,
        TaskQueue: "reports",
        Args:      []interface{}{ReportInput{Type: "daily"}},
    },
    Policy: &client.SchedulePolicies{
        Overlap: enums.SCHEDULE_OVERLAP_POLICY_SKIP, // skip if previous run still active
    },
})
```

### Go — Pause / Resume / Backfill

```go
// Pause
err = handle.Pause(ctx, client.SchedulePauseOptions{Note: "Paused for maintenance."})

// Resume
err = handle.Unpause(ctx, client.ScheduleUnpauseOptions{Note: "Maintenance complete."})

// Backfill — run all missed executions in a time range immediately
err = handle.Backfill(ctx, client.ScheduleBackfillOptions{
    Backfill: []client.ScheduleBackfill{
        {
            Start:   time.Now().Add(-4 * time.Hour),
            End:     time.Now(),
            Overlap: enums.SCHEDULE_OVERLAP_POLICY_ALLOW_ALL,
        },
    },
})
```

**Overlap policies:** `SKIP` (default), `BUFFER_ONE`, `BUFFER_ALL`, `CANCEL_OTHER`, `TERMINATE_OTHER`, `ALLOW_ALL`.

Scheduled workflow executions receive search attributes `TemporalScheduledStartTime` and `TemporalScheduledById` automatically, enabling filtering in the Temporal UI.

## Saga Pattern

Sagas coordinate multi-service transactions without distributed locks. Each step has a compensating activity that undoes it. On failure at step N, run compensations for steps N-1 down to 1 in reverse order.

```go
func BookTripWorkflow(ctx workflow.Context, input BookTripInput) (BookTripResult, error) {
    // Collect compensations to run on failure.
    var compensations []func(workflow.Context) error

    activityOpts := workflow.ActivityOptions{StartToCloseTimeout: 30 * time.Second}
    ctx = workflow.WithActivityOptions(ctx, activityOpts)

    // Step 1: reserve flight
    var flightRes FlightReservation
    if err := workflow.ExecuteActivity(ctx, ReserveFlight, input.Flight).Get(ctx, &flightRes); err != nil {
        return BookTripResult{}, err
    }
    compensations = append(compensations, func(ctx workflow.Context) error {
        return workflow.ExecuteActivity(ctx, CancelFlightReservation, flightRes.ReservationID).Get(ctx, nil)
    })

    // Step 2: reserve hotel
    var hotelRes HotelReservation
    if err := workflow.ExecuteActivity(ctx, ReserveHotel, input.Hotel).Get(ctx, &hotelRes); err != nil {
        return BookTripResult{}, runCompensations(ctx, compensations)
    }
    compensations = append(compensations, func(ctx workflow.Context) error {
        return workflow.ExecuteActivity(ctx, CancelHotelReservation, hotelRes.ReservationID).Get(ctx, nil)
    })

    // Step 3: charge payment
    if err := workflow.ExecuteActivity(ctx, ChargePayment, input.Payment).Get(ctx, nil); err != nil {
        return BookTripResult{}, runCompensations(ctx, compensations)
    }

    return BookTripResult{
        FlightID: flightRes.ReservationID,
        HotelID:  hotelRes.ReservationID,
    }, nil
}

func runCompensations(ctx workflow.Context, compensations []func(workflow.Context) error) error {
    // Run in reverse order; log but continue on compensation errors.
    var errs []error
    for i := len(compensations) - 1; i >= 0; i-- {
        if err := compensations[i](ctx); err != nil {
            errs = append(errs, err)
        }
    }
    if len(errs) > 0 {
        return fmt.Errorf("compensation errors: %v", errs)
    }
    return errors.New("saga rolled back")
}
```

Compensation activities must also be idempotent. Canceling a reservation that was never made (or already canceled) should be a no-op, not an error.

## Testing

Temporal's test framework runs workflows in-memory without a real server, with time-skipping so year-long workflows run in milliseconds.

### Go (Testing)

```go
type OrderWorkflowTestSuite struct {
    suite.Suite
    testsuite.WorkflowTestSuite
    env *testsuite.TestWorkflowEnvironment
}

func (s *OrderWorkflowTestSuite) SetupTest() {
    s.env = s.NewTestWorkflowEnvironment()
}

func (s *OrderWorkflowTestSuite) TearDownTest() {
    s.env.AssertExpectations(s.T())
}

func (s *OrderWorkflowTestSuite) Test_ProcessOrder_Success() {
    // Mock the activity — no real network calls.
    s.env.OnActivity(ChargePayment, mock.Anything, mock.Anything).Return(
        ChargeResult{TransactionID: "txn-123"}, nil,
    )
    s.env.OnActivity(ShipOrder, mock.Anything, mock.Anything).Return(
        ShipResult{TrackingNumber: "track-abc"}, nil,
    )

    s.env.ExecuteWorkflow(ProcessOrderWorkflow, ProcessOrderInput{
        OrderID: "order-1", CustomerID: "cust-1",
    })

    s.True(s.env.IsWorkflowCompleted())
    s.NoError(s.env.GetWorkflowError())

    var result ProcessOrderOutput
    s.NoError(s.env.GetWorkflowResult(&result))
    s.Equal("shipped", result.Status)
}

func (s *OrderWorkflowTestSuite) Test_ProcessOrder_ChargeFailure() {
    s.env.OnActivity(ChargePayment, mock.Anything, mock.Anything).Return(
        ChargeResult{}, errors.New("payment declined"),
    )

    s.env.ExecuteWorkflow(ProcessOrderWorkflow, ProcessOrderInput{OrderID: "order-1"})

    s.True(s.env.IsWorkflowCompleted())
    s.Error(s.env.GetWorkflowError())
}

func TestOrderWorkflowTestSuite(t *testing.T) {
    suite.Run(t, new(OrderWorkflowTestSuite))
}
```

**Time skipping:** call `s.env.SetTestTimeout(time.Hour * 24)` or use `RegisterDelayedCallback` to inject signals/queries at specific simulated times.

**Replay testing:** validate that new code is backward-compatible with existing histories:

```go
replayer := worker.NewWorkflowReplayer()
replayer.RegisterWorkflow(ProcessOrderWorkflow)
err := replayer.ReplayWorkflowHistoryFromJSONFile(zaptest.NewLogger(t), "testdata/history.json")
assert.NoError(t, err)
```

Export history JSON from the Temporal CLI: `temporal workflow show --workflow-id=<id> --output json > testdata/history.json`

## Worker Configuration

Workers poll Task Queues. Tune concurrency to match your host resources; defaults are conservative.

### Go (Worker Configuration)

```go
w := worker.New(temporalClient, "order-processing", worker.Options{
    // Maximum number of Activity tasks executing concurrently on this worker.
    MaxConcurrentActivityExecutionSize: 100,

    // Maximum number of Workflow tasks executing concurrently.
    // Workflow tasks are lightweight (replay only); can be higher than activities.
    MaxConcurrentWorkflowTaskExecutionSize: 10,

    // Rate limit on activity executions per second across all workers on this task queue.
    // Useful for protecting downstream services.
    TaskQueueActivitiesPerSecond: 50,

    // Rate limit scoped to this worker process only.
    WorkerActivitiesPerSecond: 10,

    // Sticky task queues cache a workflow's state on the same worker,
    // reducing replay overhead. Set to 0 to disable (useful for debugging).
    StickyScheduleToStartTimeout: 5 * time.Second,
})

w.RegisterWorkflow(ProcessOrderWorkflow)
w.RegisterActivity(&OrderActivities{db: db, httpClient: httpClient})

if err := w.Run(worker.InterruptCh()); err != nil {
    log.Fatalln("Worker failed", err)
}
```

**Separate task queues for separate concerns.** Activities that make slow external calls should not share a queue (and thus thread pool) with Activities that should complete in milliseconds. Use different task queues and workers.

**Worker pools:** Run multiple instances of the same worker binary against the same task queue for horizontal scale. Temporal distributes tasks via long-polling; no additional coordination needed.

## Critical Rules / Gotchas

**Non-determinism failures** crash a workflow with `workflow.ErrNonDeterministicCode`. Common causes:

- Using `time.Now()` instead of `workflow.Now(ctx)`.
- Using `go` keyword instead of `workflow.Go`.
- Iterating a map (Go map iteration order is random).
- Calling `rand` with a time-based seed.
- Making network/DB calls directly in workflow code.
- Importing packages with `init()` side effects inside workflow code.

**Versioning workflows** — when you must change workflow logic for running executions, use the versioning API to branch:

```go
v := workflow.GetVersion(ctx, "add-verification-step", workflow.DefaultVersion, 1)
if v == 1 {
    // New code path for new executions.
    workflow.ExecuteActivity(ctx, VerifyOrder, ...).Get(ctx, nil)
}
// Old executions skip this block entirely (v == workflow.DefaultVersion).
```

Without versioning, deploying changed workflow code will cause in-flight executions to fail on the next replay.

### Activity timeouts vs. retries

- `StartToCloseTimeout` does not reset between retry attempts; each attempt has its own deadline.
- `ScheduleToCloseTimeout` is the hard outer limit across all attempts. If an activity retries for 3 days but `ScheduleToCloseTimeout` is 1 hour, it will fail at 1 hour.
- Set `MaximumAttempts = 0` (unlimited) and rely on `ScheduleToCloseTimeout` as the outer bound rather than counting attempts. Counting runs out when services have multi-day outages.

**Event History size limit:** 51,200 events is the default hard limit per execution. Long-running workflows that loop forever will eventually hit this. Use `workflow.NewContinueAsNewError` to start a fresh execution with the current state:

```go
if iterationCount >= 1000 {
    return workflow.NewContinueAsNewError(ctx, MyWorkflow, updatedState)
}
```

**Do not retry inside activities.** Internal retry loops lengthen the needed `StartToCloseTimeout`, suppress failure metrics in the Temporal UI, and make it impossible for Temporal's retry backoff to apply. Throw the error; let Temporal retry.

**Workflow Execution IDs must be unique per logical entity.** Use a stable business key (order ID, user ID) as the Workflow ID so you can deduplicate and signal by ID without storing Temporal metadata externally.

**Always use struct inputs/outputs, never bare primitives.** Adding a field to a struct is a backward-compatible schema change. Changing a bare `string` parameter to two parameters breaks all existing callers.

## References

- [Temporal Documentation](https://docs.temporal.io)
- [Retry Policies](https://docs.temporal.io/encyclopedia/retry-policies)
- [Activity Definition](https://docs.temporal.io/activity-definition)
- [Activity Execution](https://docs.temporal.io/activity-execution)
- [Sending Messages (Signals, Queries, Updates)](https://docs.temporal.io/sending-messages)
- [Go SDK Message Passing](https://docs.temporal.io/develop/go/message-passing)
- [TypeScript SDK Message Passing](https://docs.temporal.io/develop/typescript/message-passing)
- [Child Workflows](https://docs.temporal.io/child-workflows)
- [Schedules](https://docs.temporal.io/schedule)
- [Go SDK Testing](https://docs.temporal.io/develop/go/testing-suite)
- [Go SDK Schedules](https://docs.temporal.io/develop/go/schedules)
- [Temporal Community Forum](https://community.temporal.io)
- [Best Practices — Raphaël Beamonte](https://raphaelbeamonte.com/posts/good-practices-for-writing-temporal-workflows-and-activities/)
- [Failure Handling in Practice — Temporal Blog](https://temporal.io/blog/failure-handling-in-practice)

---


# Data Pipeline Orchestration: Airflow, Prefect, and Dagster

## When to Use Each Tool

### Apache Airflow

Best fit for enterprise teams running complex, static batch workflows with deep integration requirements. Airflow dominates the ecosystem (320M+ downloads in 2024) and has the most extensive operator library. Choose it when:

- Your workflows are mostly predetermined and don't need to change shape at runtime
- You need integrations with many external systems (AWS, GCP, Azure, databases, queues)
- You have an operations team already familiar with it
- Long-term stability and a large community matter more than developer ergonomics

Weaknesses: heavyweight scheduler, poor local dev experience, DAG-as-code can become messy, dynamic workflows require workarounds.

### Prefect

Best fit for Python-native teams that need to ship pipelines fast, especially in cloud-native or ML environments. Prefect 3.x removed the DAG constraint entirely—flows are just Python with `if/else`, loops, and dynamic branching. Choose it when:

- Workflows need to change shape at runtime based on data
- You want minimal boilerplate and fast iteration
- Your team does ML or data science and the code already lives in Python
- You need elastic scaling and hybrid execution (local + cloud)

Weaknesses: data lineage tracking is manual, smaller ecosystem than Airflow, less mature for very large enterprise deployments.

### Dagster

Best fit for teams that care deeply about data quality, lineage, and observability. Dagster frames pipelines around *assets* (tables, files, models) rather than tasks, so the UI shows what data you produce and how it relates. Choose it when:

- Data lineage and catalog-level visibility matter
- You run dbt, Spark, or ML training and want to track artifacts
- Local development, unit testing, and CI/CD integration are priorities
- You need fine-grained control over what gets recomputed and when

Weaknesses: steeper learning curve, asset-first model requires a mental shift, smaller community than Airflow.


## Prefect (Data Pipeline Orchestration: Airflow, Prefect, and Dagster)

### Flows and Tasks

Everything in Prefect is a Python function decorated with `@flow` or `@task`. Flows are the unit of deployment; tasks are the unit of retry and caching within a flow.

```python
from prefect import flow, task
from prefect.tasks import exponential_backoff
import httpx

@task(
    retries=3,
    retry_delay_seconds=exponential_backoff(backoff_factor=2),
    cache_policy=...,        # optional: cache results
    log_prints=True,
)
def fetch_data(url: str) -> dict:
    response = httpx.get(url)
    response.raise_for_status()
    return response.json()

@task
def transform(raw: dict) -> list[dict]:
    return [{"id": item["id"], "value": item["v"]} for item in raw["items"]]

@flow(name="etl-pipeline", log_prints=True)
def etl(url: str = "https://api.example.com/data") -> None:
    raw = fetch_data(url)
    records = transform(raw)
    print(f"Loaded {len(records)} records")
```

Flows can use native Python control flow — no DAG constraints:

```python
@flow
def conditional_pipeline(full_refresh: bool = False) -> None:
    if full_refresh:
        truncate_table()
    records = fetch_data()
    if len(records) > 0:
        load(records)
    else:
        notify_empty()
```

### Workflow Design Patterns

**Monoflow** — single flow with sequential tasks. Simple to build and own. Good for straightforward pipelines with one owner.

**Subflows** — one flow calls another flow directly. Provides logical separation and code reuse, runs in the same process.

```python
@flow
def validate(records: list[dict]) -> bool:
    ...

@flow
def ingest() -> None:
    records = fetch()
    if validate(records):
        load(records)
```

**Orchestrator / Flow of Deployments** — a parent flow triggers a deployed child flow by name. The child runs on separate infrastructure (e.g., GPU worker, high-memory machine). Best when you need both logical and execution separation.

```python
from prefect.deployments import run_deployment

@flow
def orchestrator():
    # runs the 'train-model/gpu-deployment' on its own infrastructure
    run_deployment(
        name="train-model/gpu-deployment",
        parameters={"epochs": 50},
        timeout=0,   # 0 = fire-and-forget; omit to wait for completion
    )
```

**Event-driven** — flows triggered automatically when another flow run reaches a state. Maximum decoupling; the downstream flow knows nothing about the upstream one. Configured in Prefect Cloud automations UI or via the API.

### Retries and Error Handling (Prefect)

```python
from prefect.tasks import exponential_backoff

@task(
    retries=4,
    retry_delay_seconds=exponential_backoff(backoff_factor=2),
    # results in: 2s, 4s, 8s, 16s between attempts
)
def call_flaky_api(endpoint: str) -> dict:
    ...

# Flow-level retries (retry the entire flow run)
@flow(retries=2, retry_delay_seconds=30)
def daily_sync():
    ...
```

### Caching

Cache task results to avoid recomputation when inputs haven't changed.

```python
from prefect.tasks import task_input_hash
from datetime import timedelta

@task(
    cache_key_fn=task_input_hash,       # cache key based on all task inputs
    cache_expiration=timedelta(hours=1),
)
def expensive_query(table: str, date: str) -> list[dict]:
    ...
```

Use `cache_policy=INPUTS` for the same effect via the newer API. A task with matching cache key and unexpired result returns the cached value without executing.

### Parameterization (Prefect)

Flow parameters are typed Python function arguments. Pass them at run time via the UI, CLI, API, or code.

```python
@flow
def process_batch(
    source_table: str,
    batch_date: str,
    batch_size: int = 1000,
    dry_run: bool = False,
) -> None:
    records = extract(source_table, batch_date, batch_size)
    if not dry_run:
        load(records)
```

Run via CLI:

```bash
prefect deployment run 'process-batch/prod' \
  --param source_table=orders \
  --param batch_date=2024-11-01 \
  --param dry_run=true
```

### Secrets

Use Prefect Secret blocks to store and retrieve credentials. Secrets are encrypted at rest in Prefect Cloud.

```python
from prefect.blocks.system import Secret

@task
def connect_to_db():
    db_password = Secret.load("db-password").get()
    # use db_password to build connection string
```

Create a secret block via CLI:

```bash
prefect block create secret --name db-password
```

Or use environment variable blocks for simpler config that doesn't need encryption.

### Scheduling and Deployments

Deployments attach a flow to infrastructure (work pool) and optionally a schedule. Prefect 3.x encourages code-based deployments managed through CI/CD.

```python
# deploy.py
from prefect import flow

if __name__ == "__main__":
    flow.from_source(
        source="https://github.com/myorg/pipelines.git",
        entrypoint="flows/etl.py:etl",
    ).deploy(
        name="prod",
        work_pool_name="default-agent-pool",
        cron="0 6 * * *",          # 6 AM daily
        parameters={"url": "https://api.example.com/data"},
    )
```

Supported schedule types: `cron`, `interval` (seconds), `rrule` (RFC 5545 recurrence rules).

### Testing Prefect Flows

Flows and tasks are plain Python functions. Test them directly.

```python
def test_transform():
    from flows.etl import transform
    result = transform({"items": [{"id": 1, "v": 42}]})
    assert result == [{"id": 1, "value": 42}]

def test_flow_end_to_end(respx_mock):
    # mock HTTP calls, then call the flow directly
    respx_mock.get("https://api.example.com/data").mock(
        return_value=httpx.Response(200, json={"items": [{"id": 1, "v": 10}]})
    )
    etl(url="https://api.example.com/data")   # just call it
```

No test harness needed. Flows run synchronously in tests. Use `prefect.testing.utilities.prefect_test_harness` context manager if you need state tracking in tests.


## Cross-Tool Patterns

### Idempotency

Every pipeline must be safe to re-run. Design each unit of work so that running it twice produces the same result as running it once:

- Use UPSERT (INSERT ... ON CONFLICT DO UPDATE) instead of INSERT
- Write to partitioned paths (`s3://bucket/date=2024-11-01/output.parquet`) and overwrite the partition
- Track a `run_id` or partition key to detect duplicate work

### Secrets Management Summary

| Tool | Mechanism |
| --- | --- |
| Airflow | Connections + Variables; delegate to external backend (AWS SSM, Vault) via `[secrets] backend` config |
| Prefect | Secret blocks (encrypted in Prefect Cloud); env var blocks for non-sensitive config |
| Dagster | `dg.EnvVar()` in resource configs; inject secrets from external stores as env vars into the worker |

All three: never hardcode credentials in DAG/flow/asset code. Never log secrets. Rotate using the secret store, not by editing pipeline code.

### Retry Strategy

- **Transient network/API failures**: 3–5 retries with exponential backoff (start at 5–30s, cap at ~1 hour)
- **Data validation failures**: don't retry automatically — alert and require human review
- **Resource contention** (DB locks, rate limits): retry with jitter to avoid thundering herd
- Set a `max_retry_delay` so retries don't pile up across hours

### Testing Pipelines

1. **Unit test transformation logic** — pure functions with no framework overhead
2. **Integration test with real dependencies** — use a staging environment; seed known data, assert known outputs
3. **DAG/flow structural tests** — validate graph loads, task count, no import errors (Airflow DagBag; Dagster `Definitions` object; Prefect flow import)
4. **Data quality checks** — post-materialization assertions (Airflow sensors after write, Dagster asset checks, Prefect tasks that query and assert)
5. **Mock external services** in unit tests; use real connections in integration tests

### Parameterization Best Practices

- Parameters should have defaults that work in development without any external config
- Validate parameter types early in the flow/DAG, before any expensive work starts
- Keep environment promotion (dev → staging → prod) to environment variables, not parameter values embedded in schedules
- Document which parameters are safe to override at runtime vs which should only change via code review
