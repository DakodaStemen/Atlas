---
name: microservices-patterns
description: Microservices architectural patterns — Saga, Outbox, CQRS integration, service decomposition, bulkhead, circuit breaker, and API composition.
domain: backend
category: architecture
tags: [microservices, saga, outbox, circuit-breaker, bulkhead, service-mesh, distributed-systems]
triggers: saga pattern, outbox pattern, microservices decomposition, circuit breaker pattern, bulkhead pattern, choreography vs orchestration, transactional outbox
---

# Microservices Architectural Patterns

## When to Use

Microservices make sense when:

- **Deployment independence is mandatory.** Teams need to ship different parts of the system at different cadences without coordinating a monolith release.
- **Scale is heterogeneous.** Your payment service needs 20 replicas; your reporting service needs one. A monolith forces you to scale everything together.
- **Team topology demands it.** Conway's Law is real. If you have 6 autonomous squads each owning a distinct business capability, a single codebase creates merge contention and cross-team coordination overhead. The right service boundary often follows the team boundary.
- **Failure isolation is a hard requirement.** A crash in the recommendation engine must not take down checkout.

Avoid microservices when:

- You have fewer than 3–4 engineers. The operational overhead (service discovery, distributed tracing, independent CI/CD pipelines) kills velocity at small team sizes.
- The domain is still being discovered. Decomposing too early locks you into wrong boundaries; the cost of cross-service refactoring is much higher than within a monolith.
- You cannot accept eventual consistency. If every user interaction requires reading data that was just written across multiple services with strong consistency, microservices will cause you pain.

**Team topology signal:** If two squads are regularly editing the same files and blocking each other's releases, that's a decomposition signal. If a single squad owns a service end-to-end (data model, API, deployment), that's a healthy boundary.

---

## Saga Pattern

A saga implements a distributed business transaction as a sequence of local transactions, each of which updates one service's database and emits an event or command that drives the next step. There is no distributed transaction coordinator — each service commits locally. When a step fails, the saga runs **compensating transactions** in reverse order to undo previously committed changes.

### Choreography vs. Orchestration

**Choreography** — services react to events. No central coordinator.

```text
OrderService       → publishes: OrderCreated
InventoryService   → listens: OrderCreated  → reserves stock → publishes: StockReserved
PaymentService     → listens: StockReserved → charges card   → publishes: PaymentProcessed
OrderService       → listens: PaymentProcessed → sets status=CONFIRMED
```

Compensation on payment failure:

```text
PaymentService → publishes: PaymentFailed
InventoryService → listens: PaymentFailed → releases reservation → publishes: StockReleased
OrderService → listens: StockReleased → sets status=CANCELLED
```

Pros: loose coupling, no single point of failure in the coordinator.
Cons: the flow is implicit in event subscriptions — hard to audit, hard to visualize, distributed ownership of the saga state.

**Orchestration** — a saga orchestrator (an object or service) explicitly commands participants and tracks state.

```bash
CreateOrderSaga orchestrator:
  1. POST /inventory/reserve        → on success, continue; on fail, CANCEL
  2. POST /payment/charge           → on success, continue; on fail, release inventory
  3. PUT  /orders/{id}/confirm      → done
```

The orchestrator persists its own state machine. If it crashes mid-flight, it can resume from the last confirmed step on restart.

Pros: flow is explicit and auditable in one place, easier compensations.
Cons: the orchestrator becomes a coupling point; avoid putting business logic into it beyond sequencing.

### Compensating Transactions

Compensating transactions must be **idempotent** and **retryable**. They are not rollbacks — they are forward-moving corrective actions.

```text
// Not a rollback: inventory has been reserved, we issue a release command
POST /inventory/release
{
  "reservationId": "res_abc123",
  "reason": "payment_failed"
}
```

Design compensations at schema time. If a step has no clean inverse (e.g., "send confirmation email"), mark it as a **pivot transaction** — the point past which the saga cannot be compensated and must proceed to completion regardless.

### Failure Modes

| Failure | Choreography handling | Orchestration handling |
| --- | --- | --- |
| Service crashes mid-step | Broker retries message delivery; service must be idempotent | Orchestrator retries the command; participant must be idempotent |
| Event lost | Broker durability + at-least-once delivery | Orchestrator resends command until acknowledged |
| Compensation fails | Saga is stuck; need dead-letter queue + human intervention | Orchestrator retries compensation; escalates to DLQ after N attempts |
| Concurrent sagas on same aggregate | Semantic lock pattern: flag the aggregate as "in-flight" | Same; orchestrator checks lock before starting |

**Semantic lock countermeasure:** when a saga starts, set a `pending` flag on the target aggregate. Reject concurrent operations against it until the saga completes or compensates.

---

## Transactional Outbox

### Why It's Needed

Without the outbox, you have a dual-write problem:

```text
// BROKEN: two writes, one can fail
db.save(order)         // succeeds
broker.publish(event)  // crashes — event never sent, order committed
```

A distributed transaction (2PC) between your DB and message broker is theoretically possible but almost never available in practice (Kafka and Postgres have no shared transaction coordinator), and even where available it introduces heavy coordination overhead.

### Solution

Write the event to an **OUTBOX table in the same local DB transaction** as the business entity change. A separate relay process reads the outbox and forwards to the broker.

```sql
-- Schema
CREATE TABLE outbox (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_id  TEXT        NOT NULL,
    aggregate_type TEXT       NOT NULL,
    event_type    TEXT        NOT NULL,
    payload       JSONB       NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at  TIMESTAMPTZ          -- NULL = not yet relayed
);

-- Business transaction
BEGIN;
  INSERT INTO orders (id, status) VALUES ($1, 'PENDING');
  INSERT INTO outbox (aggregate_id, aggregate_type, event_type, payload)
    VALUES ($1, 'Order', 'OrderCreated', $2);
COMMIT;
```

### Inbox/Outbox Relay

Two relay strategies:

1. **Polling publisher** — a background job polls `WHERE published_at IS NULL ORDER BY created_at`, publishes to broker, marks `published_at = now()`. Simple but adds DB load and has latency proportional to polling interval.

2. **Transaction log tailing (CDC)** — Debezium reads Postgres WAL (or MySQL binlog) and streams committed outbox inserts directly to Kafka with sub-second latency and zero additional DB queries.

```yaml
# Debezium connector config (Kafka Connect)
connector.class: io.debezium.connector.postgresql.PostgresConnector
database.hostname: postgres
database.dbname: orders
table.include.list: public.outbox
transforms: outbox
transforms.outbox.type: io.debezium.transforms.outbox.EventRouter
transforms.outbox.table.field.event.id: id
transforms.outbox.table.field.event.key: aggregate_id
transforms.outbox.table.field.event.payload: payload
transforms.outbox.route.by.field: aggregate_type
```

### At-Least-Once vs. Exactly-Once

The outbox gives you **at-least-once delivery**. The relay can crash after publishing to the broker but before marking the row as published, causing a duplicate on restart. Consumers must handle duplicates by tracking processed event IDs:

```sql
CREATE TABLE processed_events (
    event_id   UUID PRIMARY KEY,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Consumer logic
BEGIN;
  INSERT INTO processed_events (event_id) VALUES ($1)
    ON CONFLICT DO NOTHING;
  -- if 0 rows inserted, this is a duplicate — skip processing
  -- if 1 row inserted, apply business logic
COMMIT;
```

Exactly-once delivery requires Kafka transactions (`read_committed` isolation + transactional producers), which adds significant complexity. At-least-once + idempotent consumers is the pragmatic standard.

**Cleanup:** archive or delete processed outbox rows on a schedule. An unbounded outbox table is a silent performance bomb.

---

## Circuit Breaker

Prevents cascading failures by wrapping calls to a dependency and short-circuiting them when the dependency is unhealthy.

### States

```text
CLOSED  ──(failures exceed threshold)──► OPEN
OPEN    ──(timeout expires)────────────► HALF-OPEN
HALF-OPEN ──(probe succeeds)───────────► CLOSED
HALF-OPEN ──(probe fails)──────────────► OPEN
```

- **Closed:** requests pass through. Failure count tracked in a sliding window.
- **Open:** requests fail immediately without touching the downstream service. Returns fallback or error.
- **Half-Open:** a single probe request is allowed through. Success → close; failure → reopen.

### Resilience4j (Java)

```java
CircuitBreakerConfig config = CircuitBreakerConfig.custom()
    .slidingWindowType(SlidingWindowType.COUNT_BASED)
    .slidingWindowSize(10)
    .failureRateThreshold(50)           // trip at 50% failures
    .waitDurationInOpenState(Duration.ofSeconds(30))
    .permittedNumberOfCallsInHalfOpenState(3)
    .slowCallDurationThreshold(Duration.ofMillis(800))
    .slowCallRateThreshold(80)          // also trip if 80% calls are slow
    .build();

CircuitBreaker cb = CircuitBreaker.of("paymentService", config);

Supplier<PaymentResult> decoratedCall = CircuitBreaker
    .decorateSupplier(cb, () -> paymentClient.charge(request));

Try.ofSupplier(decoratedCall)
    .recover(CallNotPermittedException.class, ex -> PaymentResult.fallback());
```

### go-resilience

```go
breaker := circuit.NewBreaker(circuit.WithFailureRatio(0.5),
    circuit.WithOpenTimeout(30*time.Second))

result, err := breaker.Call(func() error {
    return paymentClient.Charge(ctx, req)
}, 800*time.Millisecond)
```

### Polly (.NET)

```csharp
var policy = Policy
    .Handle<HttpRequestException>()
    .Or<TimeoutException>()
    .CircuitBreakerAsync(
        exceptionsAllowedBeforeBreaking: 5,
        durationOfBreak: TimeSpan.FromSeconds(30),
        onBreak: (ex, breakDelay) => logger.LogWarning("Circuit open for {delay}", breakDelay),
        onReset: () => logger.LogInformation("Circuit closed"),
        onHalfOpen: () => logger.LogInformation("Circuit half-open")
    );
```

**Key configuration gotcha:** the timeout on a half-open probe must be shorter than the normal request timeout. If the downstream service is slow-failing (not refusing connections), a full-timeout probe keeps the circuit half-open longer than intended.

---

## Bulkhead

Isolates resource pools so that overload or failure in one part of the system does not exhaust resources for the rest. Named after the watertight compartments in ship hulls.

### Thread Pool Isolation

Assign a dedicated thread pool per downstream dependency. If the payment service hangs, its thread pool fills up; the inventory service thread pool is unaffected.

```java
// Resilience4j ThreadPoolBulkhead
ThreadPoolBulkheadConfig config = ThreadPoolBulkheadConfig.custom()
    .maxThreadPoolSize(10)
    .coreThreadPoolSize(5)
    .queueCapacity(20)
    .keepAliveDuration(Duration.ofMillis(20))
    .build();

ThreadPoolBulkhead bulkhead = ThreadPoolBulkhead.of("paymentService", config);

CompletableFuture<String> future = bulkhead
    .executeSupplier(() -> paymentClient.charge(request));
```

### Connection Pool Isolation (HikariCP)

Separate `DataSource` beans per logical concern. Prevent a runaway analytics query from starving OLTP connections:

```yaml
# application.yml
datasources:
  transactional:
    maximum-pool-size: 20
    connection-timeout: 3000
  reporting:
    maximum-pool-size: 5
    connection-timeout: 10000
```

### Semaphore Bulkhead (lightweight, same thread)

For async/reactive stacks where thread pool isolation is too coarse:

```java
BulkheadConfig config = BulkheadConfig.custom()
    .maxConcurrentCalls(25)
    .maxWaitDuration(Duration.ofMillis(50))
    .build();
```

Combine bulkhead + circuit breaker in that order (bulkhead wraps circuit breaker): the bulkhead limits concurrency, the circuit breaker tracks failure rate within that limited concurrency.

---

## Service Decomposition

### Domain-Driven Decomposition

Two primary strategies from DDD:

1. **Decompose by business capability** — identify what the business does (Orders, Payments, Inventory, Notifications) and make each capability a service. Capabilities are stable; they change slower than implementation details.

2. **Decompose by subdomain** — map bounded contexts from the domain model. Each bounded context has its own ubiquitous language; a `Customer` in the billing context has different attributes than a `Customer` in the shipping context. These differences should not be merged.

#### Bounded context signals that decomposition is needed

- The same entity name means different things to different teams.
- A database table has 40+ columns serving unrelated purposes.
- You need to join across 5+ tables to answer a single business question that belongs to one concept.
- Two teams cannot change their data models without coordinating schema migrations.

### Strangler Fig Pattern

Incrementally migrate a monolith to microservices without a big-bang rewrite:

1. Put a routing facade (API gateway or reverse proxy) in front of the monolith.
2. Implement one new capability as a microservice.
3. Route traffic for that capability to the new service; monolith handles everything else.
4. Repeat until the monolith handles nothing and can be deleted.

The facade is the strangler fig — it wraps the monolith and slowly takes over. Never do this without feature flags so you can route traffic back instantly.

```nginx
# Nginx routing — new payment service strangling monolith
location /api/payments {
    proxy_pass http://payment-service:8080;
}
location / {
    proxy_pass http://monolith:8080;
}
```

---

## API Composition vs. CQRS

### The Join Problem in Microservices

Services own their data. There is no cross-service JOIN. When a client needs data from three services (order details, customer profile, product info), the options are:

**API Composition** — the API gateway or a dedicated aggregator service calls each downstream service and merges the results in memory.

```text
GET /orders/{id}/summary
  → GET /order-service/orders/{id}
  → GET /customer-service/customers/{customerId}
  → GET /product-service/products/{productId}
  ← merge and return
```

Works well for simple aggregations. Fails when: the data comes from dozens of services, you need filtering/sorting across combined data, or latency of fan-out is unacceptable.

**CQRS (Command Query Responsibility Segregation)** — maintain a separate read model (view database) that pre-joins data from multiple services. Each service publishes events; a read-model projector consumes them and maintains a denormalized read store.

```bash
OrderCreated event   ──► projector ──► order_summary_view table
CustomerUpdated event ──► projector ──► order_summary_view table (update customer fields)
ProductPriceChanged   ──► projector ──► order_summary_view table (update product fields)

GET /order-summaries → single query against order_summary_view
```

The read model is eventually consistent. Acceptable for most query workloads; not for "read your own writes" scenarios unless you compensate with local caching or synchronous projection on write.

Use API composition for low-cardinality aggregations (one order with its customer). Use CQRS for list/search views that aggregate across entity types at scale.

---

## Inter-service Communication

### Decision Matrix

| Criterion | Sync REST/gRPC | Async Messaging |
| --- | --- | --- |
| Client needs immediate response | Required | Use request-reply pattern (reply-to queue) |
| Temporal decoupling needed | Not provided | Core feature |
| Service must be available at call time | Yes | No — producer and consumer can be down simultaneously |
| At-most-once semantics matter | Easier to achieve | Extra work (deduplication) |
| Event fan-out (1 producer, N consumers) | Must call each consumer | Native pub/sub |
| Request volume spikes | Backpressure falls on caller | Broker absorbs spikes into queue |

**Use gRPC over REST** when: you have many internal service-to-service calls (protobuf reduces payload size 3–10×), you need streaming (server-push, bidirectional), or you want a strongly typed contract enforced at compile time.

**Use async messaging** for: workflow steps (saga), cross-domain event notifications, integrations where the consumer is external or unreliable.

```protobuf
// gRPC contract — strict, versioned, efficient
syntax = "proto3";
service InventoryService {
  rpc ReserveStock(ReserveRequest) returns (ReserveResponse);
}
message ReserveRequest {
  string order_id    = 1;
  string product_id  = 2;
  int32  quantity    = 3;
}
```

---

## Distributed Tracing Integration

### W3C Trace Context Propagation

Every inter-service call must carry the `traceparent` header (W3C Trace Context standard). This header encodes: version, trace-id (128-bit), parent-span-id (64-bit), and trace flags.

```yaml
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
              VV  ────────trace-id────────────  ──parent-span──  flags
```

Propagate it explicitly in outgoing HTTP and messaging calls:

```java
// OpenTelemetry Java — HTTP client instrumentation propagates automatically
// For Kafka producers, inject into message headers:
W3CTraceContextPropagator propagator = W3CTraceContextPropagator.getInstance();
Map<String, String> headers = new HashMap<>();
propagator.inject(Context.current(), headers, Map::put);
producerRecord.headers().add("traceparent", headers.get("traceparent").getBytes());
```

```go
// Go — inject into outgoing HTTP request
propagator := otel.GetTextMapPropagator()
propagator.Inject(ctx, propagation.HeaderCarrier(req.Header))
```

Structured logs must include `trace_id` and `span_id` extracted from context so logs can be correlated with traces:

```json
{"level":"info","trace_id":"4bf92f3577b34da6a3ce929d0e0e4736","span_id":"00f067aa0ba902b7","msg":"payment charged"}
```

Use a collector (OpenTelemetry Collector → Jaeger/Tempo/Datadog) rather than SDK-direct export so you can change backends without redeploying services.

---

## Critical Rules / Gotchas

### 1. Never use distributed transactions (2PC) across services

The coordinator is a single point of failure. Network partitions leave participants in doubt state indefinitely. Use saga + outbox instead.

#### 2. Every handler must be idempotent

Brokers deliver at-least-once. Your consumers will see duplicates. Use idempotency keys (event IDs, request IDs) and deduplicate at the persistence layer, not in application logic.

```sql
INSERT INTO payments (id, order_id, amount, status)
VALUES ($eventId, $orderId, $amount, 'COMPLETED')
ON CONFLICT (id) DO NOTHING;  -- duplicate event = no-op
```

#### 3. Version your events before you need to

Adding a field to an event is backwards compatible. Removing or renaming a field breaks consumers silently if you have no schema registry. Use Avro or Protobuf with a schema registry (Confluent Schema Registry, AWS Glue). Never break wire format without a migration plan.

#### 4. Compensating transactions must account for concurrent modifications

Between a saga step committing and its compensation running, another process may have modified the same aggregate. Write compensations defensively — check preconditions before applying.

#### 5. One database per service is not optional

Sharing a database between services defeats service independence. If two services share a DB, a schema migration in one blocks the other. Use DB per service from day one; retrofitting it later is a major undertaking.

#### 6. Avoid chatty synchronous call chains

`ServiceA → ServiceB → ServiceC → ServiceD` synchronously means your p99 latency is the sum of all four, and a failure anywhere propagates up. Flatten deep call graphs using async events or query-side pre-computation (CQRS).

#### 7. Design for partial availability

Services will be down. Every synchronous caller must have a fallback (stale cache, degraded response, circuit open response). Never let `null` propagate from a failed downstream into a NullPointerException that crashes your service.

#### 8. Correlation IDs everywhere

Every request entering the system should get a correlation ID at the edge (API gateway). Every log line, every outbox event, every saga step must carry it. Without this, debugging a failure that spans five services is practically impossible.

---

## References

- [microservices.io — Saga Pattern](https://microservices.io/patterns/data/saga.html) — Chris Richardson's canonical pattern catalog
- [microservices.io — Transactional Outbox](https://microservices.io/patterns/data/transactional-outbox.html)
- [microservices.io — Circuit Breaker](https://microservices.io/patterns/reliability/circuit-breaker.html)
- [microservices.io — Decompose by Business Capability](https://microservices.io/patterns/decomposition/decompose-by-business-capability.html)
- [Saga Orchestration for Microservices Using the Outbox Pattern — InfoQ](https://www.infoq.com/articles/saga-orchestration-outbox/)
- [Microsoft Azure — Saga design pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/saga)
- [Resilience4j documentation](https://resilience4j.readme.io/)
- [W3C Trace Context specification](https://www.w3.org/TR/trace-context/)
- [Debezium — Outbox Event Router](https://debezium.io/documentation/reference/transformations/outbox-event-router.html)
- Chris Richardson, *Microservices Patterns* (Manning, 2018) — the definitive book; patterns catalog at microservices.io mirrors the book's content
