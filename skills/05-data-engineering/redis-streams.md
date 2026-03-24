---
name: redis-streams
description: Patterns and best practices for Redis Streams — covering the full command surface (XADD, XREAD, XRANGE, XLEN, XREADGROUP, XACK, XPENDING, XCLAIM, XTRIM), consumer group design, pending entry recovery, trimming strategies, backpressure, monitoring, and a clear decision framework for when to use Streams vs pub/sub vs Kafka.
domain: backend
category: messaging
tags: [Redis, Streams, XADD, XREAD, consumer-groups, event-log, XACK, XPENDING, XCLAIM, messaging, event-driven]
triggers: ["redis stream", "redis streams", "XADD", "XREAD", "XREADGROUP", "XACK", "XPENDING", "consumer group redis", "redis event log", "redis message queue", "redis vs kafka"]
---

# Redis Streams

Redis Streams is a persistent, append-only log data structure built into Redis. Each entry has a server-assigned ID (`<milliseconds>-<sequence>`), making it an ordered event log. Unlike pub/sub (which is fire-and-forget), streams survive restarts and allow consumers to replay history or pick up where they left off.

---

## Core Commands

### XADD — Append an entry

```text
XADD stream_key [MAXLEN [~] count] * field1 value1 field2 value2 ...
```

- `*` tells Redis to auto-generate the ID. Explicit IDs (`1700000000000-0`) are allowed but rare.
- Combine with `MAXLEN` to cap stream length inline; prefer approximate trimming (`~`) for CPU efficiency.

```text
# auto-ID, no trimming
XADD events * type page_view user_id 42 path /home

# auto-ID, trim to ~10 000 entries (approximate)
XADD events MAXLEN ~ 10000 * type page_view user_id 42 path /home
```

### XREAD — Read without a consumer group

```text
XREAD [COUNT n] [BLOCK ms] STREAMS key [key ...] id [id ...]
```

- Use ID `0` to read from the beginning; use `$` to read only new entries from now.
- `BLOCK 0` blocks indefinitely until new data arrives — useful for tailing a stream.

```text
# read up to 100 entries from beginning
XREAD COUNT 100 STREAMS events 0

# block forever, receive only entries added after this call
XREAD BLOCK 0 STREAMS events $
```

`XREAD` delivers every matching entry to every caller — it has no exclusivity. For parallel processing, use consumer groups.

### XRANGE / XREVRANGE — Scan a range

```text
XRANGE stream_key start end [COUNT n]
XREVRANGE stream_key end start [COUNT n]
```

Use `-` and `+` as "lowest possible" and "highest possible" IDs.

```text
# full history
XRANGE events - +

# last 50 entries
XREVRANGE events + - COUNT 50

# entries between two timestamps
XRANGE events 1700000000000-0 1700003600000-0
```

### XLEN — Entry count

```text
XLEN events        # returns current number of entries
```

### XTRIM — Explicit trimming

```text
XTRIM stream_key MAXLEN [~] count
XTRIM stream_key MINID [~] threshold_id
```

- `MAXLEN ~ 50000` keeps approximately the most recent 50 000 entries.
- `MINID ~ 1700000000000-0` deletes entries older than a given ID (use a millisecond timestamp as the ID prefix to approximate time-based retention).
- Prefer `~` (approximate) in all production cases; exact trimming forces Redis to walk partial macro-nodes and is significantly slower.

---

## Consumer Groups

A consumer group tracks a read cursor and a Pending Entries List (PEL) for unacknowledged messages. Multiple named consumers inside a group share the stream — each entry goes to exactly one consumer.

### Create a group

```text
XGROUP CREATE stream_key group_name start_id [MKSTREAM]
```

- `$` — deliver only messages added after this point (live consumers).
- `0` — deliver all existing messages first (catch-up consumers).
- `MKSTREAM` — create the stream key if it doesn't exist yet.

```text
XGROUP CREATE events workers $ MKSTREAM
XGROUP CREATE events reprocessing 0
```

### XREADGROUP — Consume as a group member

```text
XREADGROUP GROUP group_name consumer_name [COUNT n] [BLOCK ms] [NOACK] STREAMS key [key ...] id [id ...]
```

- ID `>` means "give me the next undelivered message". This is the normal read path.
- ID `0` (or any explicit ID) means "re-read my own pending messages that I haven't acknowledged yet". Use this on startup to drain the backlog before switching to `>`.

```text
# normal consumption — get next undelivered entry
XREADGROUP GROUP workers consumer-1 COUNT 10 BLOCK 5000 STREAMS events >

# recovery on startup — drain own pending backlog first
XREADGROUP GROUP workers consumer-1 COUNT 100 STREAMS events 0
```

### XACK — Acknowledge processing

```text
XACK stream_key group_name id [id ...]
```

Removes the entry from the PEL. Call this only after the work is durably committed. Acknowledging too early risks data loss if the consumer crashes between ACK and actual processing. Acknowledge too late and the PEL grows, degrading XREADGROUP performance.

```text
XACK events workers 1700000000000-0 1700000000001-0
```

---

## Pending Entries and Recovery

Every message delivered via `XREADGROUP` enters the PEL until `XACK`ed. If a consumer crashes, those messages stay pending — they are never silently redelivered.

### XPENDING — Inspect the backlog

```text
# summary: total count, ID range, per-consumer counts
XPENDING events workers

# detailed list: IDs, owning consumer, idle time ms, delivery count
XPENDING events workers - + 20

# filtered to one consumer
XPENDING events workers - + 20 consumer-1
```

### XCLAIM / XAUTOCLAIM — Steal a stalled message

Transfer ownership of a pending entry to another consumer. The idle threshold prevents stealing a message that is actively being processed.

```text
# claim a single message idle for >60 000 ms
XCLAIM events workers consumer-2 60000 1700000000000-0

# bulk claim: all entries idle > 60 s, starting from 0-0, up to 100 at a time
# returns the claimed entries AND the next cursor for pagination
XAUTOCLAIM events workers consumer-2 60000 0-0 COUNT 100
```

The return value of `XAUTOCLAIM` includes a cursor; loop until the cursor is `0-0` to process all stale entries.

### Startup recovery pattern

```python
# 1. drain own pending backlog (messages from a previous crash)
while True:
    entries = xreadgroup(group, consumer, count=100, streams={key: "0"})
    if not entries:
        break
    for id, fields in entries:
        process(fields)
        xack(key, group, id)

# 2. switch to new messages
while True:
    entries = xreadgroup(group, consumer, count=10, block=5000, streams={key: ">"})
    for id, fields in entries:
        process(fields)
        xack(key, group, id)
```

### Dead-letter queue

Track the delivery count returned by `XPENDING`. After N retries (typically 3–5), move the entry to a separate DLQ stream for manual inspection rather than retrying indefinitely.

```python
MAX_RETRIES = 3

for id, consumer, idle_ms, delivery_count in pending_entries:
    if delivery_count > MAX_RETRIES:
        xadd("events:dlq", {"original_id": id, "reason": "max_retries", **original_fields})
        xack("events", "workers", id)
    elif idle_ms > 60_000:
        xclaim("events", "workers", active_consumer, 60000, id)
```

---

## Trimming Strategies

Streams grow without bound unless explicitly trimmed. Two main approaches:

### 1. Inline trimming via XADD MAXLEN

Cheapest operationally — trim happens at write time.

```text
XADD events MAXLEN ~ 100000 * type click user_id 7
```

#### 2. Periodic XTRIM by age (MINID)

Keeps entries no older than some window. Compute the MINID from the current epoch minus the retention window.

```python
retention_ms = 7 * 24 * 60 * 60 * 1000   # 7 days
cutoff_id = f"{int(time.time() * 1000) - retention_ms}-0"
xtrim("events", minid="~", threshold=cutoff_id)
```

#### 3. XDEL — avoid in production

`XDEL` marks individual entries deleted but does not free memory until the entire radix-tree macro-node is empty. Heavy use creates "Swiss cheese" fragmentation. Use XTRIM instead for bulk removal.

---

## Backpressure Handling

Redis Streams have no native flow-control protocol. Backpressure must be implemented at the application layer.

- **Monitor XLEN**: if stream length exceeds a high-water mark, slow or pause producers (return 429 or introduce write delays).
- **Monitor PEL size**: a growing PEL means consumers are not keeping up or are crashing. Alert on it before it becomes a memory crisis — the PEL is a separate radix tree and large ones visibly degrade `XREADGROUP` latency.
- **Scale consumers horizontally**: add more named consumers to the same group. Entries distribute across all of them automatically.
- **Use COUNT judiciously**: consuming in batches of 10–100 is more efficient than one-at-a-time.
- **Separate slow and fast paths**: use distinct streams or consumer groups for priority tiers rather than mixing latency-sensitive and bulk work in one stream.

---

## Monitoring

| Metric | How to observe | Alert threshold (indicative) |
| --- | --- | --- |
| Stream length (lag) | `XLEN stream_key` | > 10 000 unprocessed |
| PEL size | `XPENDING stream_key group` summary | > 500 pending |
| Consumer idle time | `XPENDING` detailed, `idle_ms` column | consumer idle > 5 min with pending entries |
| Oldest unacked entry age | Min ID in `XPENDING` range | > 2× your SLA |
| Memory | `MEMORY USAGE stream_key` | proportional to entry size × MAXLEN |

Run periodic jobs (not inline with consumers) that call `XPENDING` and emit these values to your metrics system (Prometheus, Datadog, etc.).

---

## Redis Streams vs Pub/Sub vs Kafka

### Redis Pub/Sub

| Attribute | Pub/Sub | Streams |
| --- | --- | --- |
| Persistence | None — lost if no subscriber | Yes — survives restart |
| Replay | No | Yes |
| Acknowledgment | No | Yes (XACK) |
| Consumer groups | No | Yes |

Use pub/sub for ephemeral fan-out where message loss is acceptable (live presence, UI push). Use Streams whenever durability or at-least-once delivery matters.

### Redis Streams vs Kafka

| Attribute | Redis Streams | Kafka |
| --- | --- | --- |
| Latency | Sub-millisecond (in-memory) | Low-tens of ms |
| Throughput | High; bounded by RAM | Extremely high; disk-backed |
| Retention | TTL/MAXLEN trimming required | Configurable indefinite |
| Ordering | Per-stream (single partition model) | Per-partition within a topic |
| Consumer groups | Yes; job-queue semantics | Yes; partition-based assignment |
| Rebalancing | Manual or application logic | Built-in |
| Operational cost | Low (Redis already in stack) | High (ZooKeeper / KRaft, brokers) |
| Replay at scale | Limited by memory budget | Strong native support |

**The key architectural gap**: Redis has no native partitioning. One stream is effectively one Kafka partition. To replicate Kafka's ordered-per-key semantics at scale, you need multiple streams with application-side sharding — `events:0` through `events:N`, routing by `hash(key) % N`. This is doable but adds complexity Kafka handles natively.

#### Use Redis Streams when

- You already run Redis and want to avoid another operational dependency.
- Volumes fit comfortably in memory (tens of millions of entries, not terabytes).
- Sub-millisecond latency is a hard requirement.
- Retention windows are bounded (hours to days, not months).
- You need a simple job queue with at-least-once delivery and retry.

#### Use Kafka when

- You need multi-month or indefinite retention for audit or replay purposes.
- Per-key ordering across millions of keys is required.
- You are processing hundreds of MB/s or more and need horizontal broker scaling.
- You need battle-tested consumer group rebalancing without application code.

---

## Redis Streams as an Event Log

Streams are a natural fit for lightweight event sourcing:

- **Append-only**: entries cannot be mutated; the log is immutable by convention.
- **ID as timestamp**: the auto-generated `<ms>-<seq>` ID is a wall-clock timestamp, usable for time-range queries via `XRANGE`.
- **Multiple independent readers**: create a separate consumer group per downstream (analytics, search indexer, audit) without impacting each other. Each group maintains its own read cursor.
- **Snapshot + replay**: snapshot application state periodically to an external store (Redis Hash, PostgreSQL); on recovery, replay only entries since the snapshot ID.

Keep individual entry payloads small. For large blobs (images, full documents), store the payload in a separate store (S3, Redis key) and put only the reference ID in the stream entry.

---

## Common Pitfalls

**Not draining pending on startup.** Consumers that jump straight to `>` will skip any messages they were assigned before crashing. Always read `0` first until the result is empty.

**Acknowledging before durable commit.** If you XACK then write to the database and the database write fails, the message is gone. ACK after the side-effect is confirmed.

**Unbounded PEL from crashed consumers.** A dead consumer's pending entries accumulate silently. Build a reaper that calls `XPENDING`, identifies entries idle beyond a threshold, and either `XCLAIM`s them to a live worker or routes them to the DLQ.

**Using XDEL for cleanup.** Sparse deletion fragments the radix tree without freeing memory. Trim from the head with XTRIM instead.

**Exact MAXLEN trimming in hot paths.** `XADD events MAXLEN 10000 * ...` forces exact trimming on every write. Use `~` unless the exact count is a hard contract.

**Growing PEL from high-NOACK usage.** `NOACK` skips PEL insertion and is fine for truly fire-and-forget workloads, but mixing it with retry logic leads to silent drops. Use it deliberately.
