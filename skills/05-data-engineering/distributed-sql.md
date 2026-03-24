---
name: distributed-sql
description: Patterns, trade-offs, and operational guidance for CockroachDB (distributed SQL), PlanetScale (Vitess/MySQL), and Neon (serverless PostgreSQL). Covers multi-region topology, primary key design, non-blocking schema changes, database branching, serverless autoscaling, and connection handling.
domain: data
category: database
tags: [CockroachDB, PlanetScale, Neon, distributed-SQL, serverless-postgres, branching, Vitess, pgBouncer, multi-region, schema-changes]
triggers: [cockroachdb, planetscale, neon, distributed sql, serverless postgres, database branching, non-blocking schema, multi-region database, pgbouncer serverless, vitess sharding]
---

# Distributed & Serverless SQL Patterns

## Platform Overview

| Platform | Engine | Primary Value | Best Fit |
| --- | --- | --- | --- |
| CockroachDB | PostgreSQL-compatible distributed SQL | Geo-distributed ACID, automatic sharding | Multi-region apps, global consistency, HA-critical workloads |
| PlanetScale | MySQL (Vitess) | Branch-based schema workflow, horizontal MySQL scale | High-write MySQL workloads, teams shipping schema changes frequently |
| Neon | PostgreSQL | Serverless autoscaling, instant DB branching, scale-to-zero | Serverless/edge apps, per-PR preview environments, variable traffic |

---

## CockroachDB

### How It Works

CockroachDB automatically shards data into ranges (default 512 MB each). Each range has a Raft consensus group with replicas distributed across nodes or regions. The leaseholder replica serves reads and coordinates writes for its range. The system routes queries transparently — the application sees a standard PostgreSQL wire protocol.

### Primary Key Design

The most consequential schema decision in CockroachDB is how you structure the primary key. Because data is ordered and sharded by primary key, monotonically increasing keys (auto-increment integers, timestamp-prefixed IDs) funnel all inserts into a single range, creating a write hotspot.

#### Use UUIDs for distributed writes

```sql
CREATE TABLE orders (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id UUID NOT NULL,
    region STRING NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);
```

`gen_random_uuid()` produces random v4 UUIDs, distributing inserts across all ranges. For sequential workloads where you need ordering, use hash-sharded indexes:

```sql
CREATE TABLE events (
    id INT8 DEFAULT unique_rowid() PRIMARY KEY USING HASH WITH (bucket_count = 8),
    payload JSONB
);
```

The `USING HASH` clause prefixes the key with a hash bucket, spreading writes across 8 buckets while preserving range scan capability within each bucket.

### Multi-Region: Table Locality

Every table in a multi-region database has a locality setting. Set the database home region first:

```sql
ALTER DATABASE myapp PRIMARY REGION "us-east1";
ALTER DATABASE myapp ADD REGION "eu-west1";
ALTER DATABASE myapp ADD REGION "ap-southeast1";
```

**REGIONAL BY TABLE** — the entire table is homed in one region. Leaseholder stays in that region. Best for tables that are only accessed from one region (admin data, config).

```sql
ALTER TABLE configs SET LOCALITY REGIONAL BY TABLE IN "us-east1";
```

**REGIONAL BY ROW** — each row is homed in a specific region via a hidden `crdb_region` column. CockroachDB routes reads and writes to the appropriate regional leaseholder automatically. Best for user data partitioned by where users live.

```sql
ALTER TABLE users SET LOCALITY REGIONAL BY ROW;
-- crdb_region is automatically added and used for routing
-- Explicitly set it during writes:
INSERT INTO users (id, email, crdb_region) VALUES (gen_random_uuid(), 'a@b.com', 'us-east1');
```

With `REGIONAL BY ROW`, a user in Europe reading their own profile hits the `eu-west1` leaseholder — single-digit millisecond latency instead of a cross-continental round-trip.

**GLOBAL** — all regions keep up-to-date replicas using a non-blocking replication protocol. Reads are fast everywhere; writes are slower (global consensus). Use for low-write reference data (currency codes, feature flags) that every region needs to read quickly.

```sql
ALTER TABLE feature_flags SET LOCALITY GLOBAL;
```

### Survival Goals

Survival goals determine how many failure domains CockroachDB can survive while keeping the cluster writable:

```sql
-- Survive the loss of a single availability zone (default, lower latency)
ALTER DATABASE myapp SURVIVE ZONE FAILURE;

-- Survive the loss of an entire region (requires 3+ regions, higher write latency)
ALTER DATABASE myapp SURVIVE REGION FAILURE;
```

`REGION FAILURE` requires at least 3 regions and increases write latency because the Raft quorum must span regions. Use it only when you genuinely need to keep serving writes through a full region outage.

### Transaction Patterns

CockroachDB uses optimistic concurrency with serializable isolation by default. Contention on popular rows causes transaction retries — this is expected, not an error condition. Client libraries should use the built-in retry loop:

```sql
-- Use SELECT FOR UPDATE to acquire locks early and reduce retries
BEGIN;
SELECT * FROM accounts WHERE id = $1 FOR UPDATE;
UPDATE accounts SET balance = balance - 100 WHERE id = $1;
COMMIT;
```

Keep transactions short. Long-running transactions hold locks across ranges and increase contention across the entire cluster.

### Query Analysis

```sql
EXPLAIN ANALYZE (DISTSQL) SELECT * FROM orders WHERE user_id = $1;
```

The DISTSQL plan shows which nodes processed which ranges. Look for unexpected full-table scans or single-node execution on queries that should fan out. For write-heavy paths, check for "hot ranges" in the Admin UI under Metrics → Hot Ranges.

### Anti-Patterns

- Sequential primary keys on high-write tables (creates a write hotspot range)
- Long-running transactions with many row locks
- `SELECT *` with implicit ordering relying on primary key sequence (undefined in distributed systems)
- Not testing backup restoration — RTO/RPO assumptions are only valid if restores actually work
- Over-indexing: every secondary index is a separate range replication cost

---

## PlanetScale

### Architecture

PlanetScale runs MySQL behind Vitess, a horizontal sharding layer originally built at YouTube. Each PlanetScale database is a Vitess cluster. VTGate is the query router that sits between your app and the underlying MySQL shards. VTTablet manages individual MySQL instances. The platform abstracts sharding entirely for most teams — you interact with it as a standard MySQL database.

#### Key Vitess safety features active by default

- Hot row protection: limits concurrent mutations on the same row to prevent lock pile-ups
- Query consolidation: deduplicates identical in-flight read queries
- Automatic row limits on unbounded queries
- Connection pooling at VTGate (multiplexes application connections onto fewer MySQL connections)

### Branching Workflow

PlanetScale's branching model is the central operational pattern. A branch is an isolated Vitess cluster with a copy of the production schema (not production data). Treat branches exactly like git branches.

```text
main (production) ──┬── feature/add-user-preferences
                    └── feature/drop-legacy-columns
```

#### Workflow

1. Create a development branch from `main`
2. Apply schema changes on the branch — use any DDL freely, it's isolated
3. Test your application against the branch connection string
4. Open a deploy request to merge schema changes back to `main`
5. PlanetScale checks for conflicts and queues the migration

A deploy request is the equivalent of a pull request, but for your database schema. It can require peer review before queuing.

### Non-Blocking Schema Changes

The non-blocking migration runs via **gh-ost** under Vitess VReplication. For an `ALTER TABLE`, PlanetScale:

1. Creates a shadow table (`_tablename_ghc`) with the new schema
2. Streams existing rows from the original table to the shadow table in batches
3. Continuously applies new binlog events (writes to the original) onto the shadow table in real time
4. Performs a low-impact table swap once the shadow table has caught up

No table locks. Production traffic continues uninterrupted throughout. The migration is traffic-aware — it throttles the row copy rate during traffic spikes to avoid consuming resources needed for live queries.

### Three-Way Merge and Conflict Detection

When you submit a deploy request, PlanetScale computes two diffs:

- `diff1`: your branch schema vs. `main` at the time you branched
- `diff2`: current `main` schema vs. `main` at the time you branched

It then tests whether `diff1(diff2(main)) == diff2(diff1(main))`. If the results diverge, that's a conflict. If one diff produces an invalid state when applied over the other, that's also a conflict. This catches subtle issues like two branches adding columns with the same name but different types, or ordering dependencies that produce different final schemas.

PlanetScale does **not** treat index ordering as a conflict — reordering indexes is semantically equivalent for query execution.

Conflicts are reported before the deploy request enters the queue, so you find out immediately rather than waiting hours for a queued migration to fail.

### Vitess Limitations Developers Hit

- **No foreign key constraints** by default (Vitess disables them). Referential integrity must be enforced at the application layer or via Vitess managed foreign keys (available but with constraints).
- **No multi-shard transactions** with full ACID guarantees — cross-shard writes use 2PC which has edge cases. Design schemas to keep related writes within a single shard (keyspace ID).
- **Schema changes only via deploy requests** on production — direct DDL on the `main` branch is blocked. This is intentional but requires workflow adjustment.
- **MySQL semantics**, not PostgreSQL. Extensions, `RETURNING`, CTEs with writes, and other PG-specific features don't apply.

### Connection Pooling

VTGate handles connection pooling internally. Your application connects to VTGate (standard MySQL protocol) and VTGate multiplexes onto MySQL instances. The effective connection limit your app sees is much higher than the underlying MySQL max_connections. For serverless environments connecting to PlanetScale, use the `@main` connection string variant which routes through VTGate's pooled connections:

```sql
mysql://user:pass@host/dbname?ssl-mode=REQUIRED
```

Keep application-side connection pools small (5–20 connections) — VTGate already pools aggressively, and large app pools just add unnecessary overhead.

---

## Neon

### Architecture (Neon)

Neon separates storage and compute at the infrastructure level. Compute is a standard PostgreSQL process running in an ephemeral VM. Storage is a distributed system with three layers:

- **Safekeepers**: Write-ahead log (WAL) replicas that persist transaction records durably before the compute acknowledges commits
- **Pageservers**: Serve data pages to compute on demand using copy-on-write. Store pages as a chain of deltas, not full copies
- **Cloud object storage (S3)**: Cost-efficient tier for infrequently accessed pages

The compute has no local disk. Every page read that isn't in the compute's buffer cache is fetched from the Pageserver over the network. This is the source of cold-start latency.

### Database Branching

Branching works via the copy-on-write storage layer. When you branch, the new compute points to the same Pageserver data as the parent — no data is copied. Only writes to the branch create new delta records in storage.

```bash
# Neon CLI
neon branches create --name preview/pr-142 --parent main
neon connection-string preview/pr-142
```

This is instant regardless of database size. A 100 GB production database branches in under a second. Use this for:

- **Per-PR preview environments**: each pull request gets its own branch with real production schema and a recent data snapshot. CI tears it down when the PR closes.
- **Staging environments**: branch from `main` daily, test migrations against real data shape
- **Point-in-time recovery testing**: branch from a historical LSN to inspect data at a past state

```bash
# Branch from a point in time (e.g., before a bad migration)
neon branches create --name recovery/investigate --parent main --timestamp 2024-11-01T14:00:00Z
```

### Autoscaling

Neon's autoscaler adjusts the compute instance size based on CPU and memory pressure, within a configured min/max range. A Compute Unit (CU) is approximately 0.25 vCPU and 1 GB RAM. Configure bounds per branch:

```yaml
min_cu: 0.25   # scale to zero when idle
max_cu: 4      # maximum 4 CU = ~1 vCPU, 4 GB RAM
```

**Auto-suspend** (scale-to-zero): the compute VM is suspended after a configurable inactivity period (default 5 minutes on free tier, configurable down to 0 or disabled). The next connection wakes it. WAL remains durable in Safekeepers during suspension.

#### Auto-suspend trade-offs

- Development/preview branches: enable it, zero cost during idle
- Production: disable it or set a high threshold. Cold starts add 500ms–3s to the first query after a suspend event, which is unacceptable for user-facing latency

### Connection Handling for Serverless

The core problem: serverless functions (Vercel, Cloudflare Workers, AWS Lambda) open a new database connection per invocation, potentially thousands per second. PostgreSQL has a hard connection limit (~100 per CU of compute). Exceeding it causes `FATAL: too many connections`.

#### Solution: always use Neon's built-in PgBouncer pooler

Neon provides two connection strings per branch:

- Direct: `postgresql://user:pass@ep-xxx.region.aws.neon.tech/dbname` — standard PostgreSQL, limited connections
- Pooled: `postgresql://user:pass@ep-xxx-pooler.region.aws.neon.tech/dbname` — routes through PgBouncer

Use the pooled endpoint for all application connections. PgBouncer runs in **transaction mode** on Neon, which means:

- Up to 10,000 client connections multiplex onto ~20–100 actual PostgreSQL connections
- Each transaction gets a backend connection; idle clients hold none
- Limitation: session-level features (`SET`, `LISTEN/NOTIFY`, prepared statements in session mode, advisory locks) do not persist across transactions. Use `SET LOCAL` or pass configuration per-transaction.

```python
# SQLAlchemy — always use the pooled host
engine = create_engine(
    "postgresql+psycopg2://user:pass@ep-xxx-pooler.region.aws.neon.tech/dbname",
    pool_size=5,           # small pool; PgBouncer handles the fan-out
    max_overflow=2,
    pool_pre_ping=True,    # detects connections broken by auto-suspend
    connect_args={"sslmode": "require"}
)
```

`pool_pre_ping=True` is important on Neon: if the compute suspended while a connection sat in your app's pool, the connection is stale. Pre-ping sends a cheap `SELECT 1` before using the connection, catching the stale state before it causes a query failure.

### Cold Start Mitigation

| Technique | Effect |
| --- | --- |
| Use pooled endpoint | PgBouncer maintains warm Postgres connections; only the first query after a full suspend pays the wake cost |
| Disable auto-suspend on production | Eliminates cold starts entirely; incurs idle compute cost |
| Set `min_cu > 0` | Compute never fully suspends, just idles at minimum size |
| Application-level warm-up | Schedule a lightweight ping on a cron (e.g., every 4 minutes) to keep compute alive without paying for idle |
| `pool_pre_ping` | Catches stale connections in app pool after an unexpected suspend |

For most production APIs, the right answer is to disable auto-suspend and set `min_cu = 0.25`. The cost difference is negligible relative to the latency impact of cold starts on user-facing requests.

### Serverless Driver

For edge runtimes (Cloudflare Workers, Vercel Edge Functions) that cannot use TCP, Neon provides an HTTP-based driver:

```typescript
import { neon } from '@neondatabase/serverless';

const sql = neon(process.env.DATABASE_URL!);
// Executes over HTTPS — works in any edge runtime
const users = await sql`SELECT * FROM users WHERE id = ${userId}`;
```

This bypasses the TCP connection lifecycle entirely. Each query is an HTTP request to Neon's query API. Suitable for low-frequency edge queries; for high-throughput paths, prefer a pooled TCP connection from a Node.js runtime.

---

## When to Use Which

### Use CockroachDB when

- You need genuine multi-region active-active writes with serializable ACID
- Data sovereignty requires rows to physically reside in specific geographic regions
- You're migrating from PostgreSQL and want full compatibility
- High availability with automatic failover is non-negotiable

#### Use PlanetScale when

- Your team ships schema changes frequently and needs a safe, reviewable workflow
- You're on MySQL and need horizontal write scalability beyond what a single primary can provide
- You want the schema-as-code model where DDL goes through the same review process as application code
- You can work within MySQL semantics and the no-foreign-key constraint

#### Use Neon when

- You're building on serverless/edge infrastructure and need scale-to-zero economics
- You want per-PR preview environments with real data for testing
- Your workload is bursty or unpredictable and you don't want to provision for peak
- You need PostgreSQL with the full ecosystem (extensions, PostGIS, pgvector, etc.) without managing a server

#### Neon vs CockroachDB for PostgreSQL

Neon is single-region PostgreSQL with serverless scaling. CockroachDB is distributed PostgreSQL designed for multi-region. If you need global distribution and regional data placement, CockroachDB. If you need serverless economics and developer branching workflows with no global distribution requirement, Neon.

---

## Connection String Patterns Cheat Sheet

```bash
# CockroachDB (PostgreSQL wire protocol)
postgresql://user:pass@free-tier.cockroachlabs.cloud:26257/defaultdb?sslmode=verify-full

# PlanetScale (MySQL protocol, SSL required)
mysql://user:pass@aws.connect.psdb.cloud/dbname?ssl-ca=/etc/ssl/cert.pem

# Neon — direct (avoid in serverless)
postgresql://user:pass@ep-xxx.us-east-2.aws.neon.tech/dbname?sslmode=require

# Neon — pooled via PgBouncer (use this in production)
postgresql://user:pass@ep-xxx-pooler.us-east-2.aws.neon.tech/dbname?sslmode=require
```
