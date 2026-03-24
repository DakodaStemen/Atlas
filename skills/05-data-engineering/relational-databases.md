---
name: relational-databases
description: Relational database patterns covering MySQL/MariaDB (InnoDB, replication, query optimization, JSON support, partitioning, security) and SQLite (WAL mode, PRAGMA tuning, FTS5, JSON, ATTACH, connection pooling). Use when working with MySQL, MariaDB, or SQLite databases.
domain: data-engineering
tags: [mysql, mariadb, sqlite, relational-database, innodb, replication, wal, fts5, query-optimization]
triggers: mysql, mariadb, sqlite, innodb, replication, WAL mode, FTS5, mysql optimization, sqlite pragma
---


# MySQL & MariaDB — Practical Reference

## Storage Engines

### InnoDB (default, use it)

InnoDB is the only storage engine worth reaching for in production MySQL. It provides:

- Row-level locking (not table-level), which allows genuine concurrency.
- ACID transactions with crash recovery via redo/undo logs.
- Foreign key enforcement.
- MVCC (Multi-Version Concurrency Control) so readers never block writers.

Every table should explicitly use InnoDB. Run with `--sql_mode=NO_ENGINE_SUBSTITUTION` so MySQL refuses to silently swap engines on `CREATE TABLE`.

### MyISAM — avoid

MyISAM uses table-level locking. A single write blocks all readers. It has no crash recovery. The only surviving legitimate use case is full-text search on very old MySQL versions (5.x) where InnoDB full-text was not yet mature. On MySQL 8+, InnoDB full-text covers that need. Do not use MyISAM for new tables.

### MariaDB extras

MariaDB ships additional engines not available in MySQL community:

- **Aria** — crash-safe MyISAM replacement. Used internally by MariaDB for system tables. Better for read-heavy workloads that do not need full ACID.
- **MyRocks** — RocksDB-backed engine, excellent write amplification characteristics on SSD-heavy servers. Good for write-intensive time-series or log-style data where compression matters.
- **ColumnStore** — columnar storage for analytical queries, a first-class citizen rather than an external plugin.


## Indexing

### B-tree (default)

The default and most widely applicable index type. Supports equality, range (`>`, `<`, `BETWEEN`), prefix matches (`LIKE 'foo%'`), and `ORDER BY` / `GROUP BY`. Almost every index you create is a B-tree.

### Hash

Only available in MEMORY tables and as an adaptive mechanism inside InnoDB (adaptive hash index, which the engine manages automatically). Cannot be created manually for InnoDB tables on disk. Useful only for exact-equality lookups; does not support ranges or sorting.

### Full-text

`FULLTEXT` indexes on `CHAR`, `VARCHAR`, or `TEXT` columns. Supports `MATCH(...) AGAINST(...)` syntax. InnoDB full-text is production-ready since MySQL 5.6. Use `IN BOOLEAN MODE` for `+must -exclude prefix*` patterns; `IN NATURAL LANGUAGE MODE` for relevance ranking.

### Spatial (R-tree)

`SPATIAL` indexes on geometry columns. Useful for GIS queries. Requires `NOT NULL` columns.

### Descending indexes (MySQL 8+ / MariaDB 10.8+)

`CREATE INDEX idx ON t (created_at DESC)`. Eliminates a filesort when the query's `ORDER BY` matches the descending direction. Before MySQL 8, the DESC keyword in an index definition was accepted but ignored.


## EXPLAIN Output

Run `EXPLAIN` before any query you care about. Run `EXPLAIN ANALYZE` (MySQL 8.0.18+, MariaDB 10.9+) to get actual row counts and timing, not just estimates.

| Column | What to look for |
| --- | --- |
| `type` | Best to worst: `system` → `const` → `eq_ref` → `ref` → `range` → `index` → `ALL`. `ALL` is a full table scan — almost always bad on large tables. |
| `key` | Which index is actually used. `NULL` means no index. |
| `key_len` | Number of bytes used from the index. Tells you how many columns of a composite index the optimizer used. |
| `rows` | Estimated rows examined. Multiply across nested loops for total cost estimate. |
| `filtered` | Percentage of rows surviving the WHERE clause after index filtering. Low values with high `rows` means lots of wasted I/O. |
| `Extra` | Critical signals: `Using filesort` (sort not satisfied by index), `Using temporary` (implicit temp table, costly for large sets), `Using index` (covering index hit), `Using index condition` (ICP — filter pushed into storage engine). |

**`const` and `eq_ref` are ideal.** `const` means the optimizer resolves the row at planning time (lookup by primary key or unique index with a constant). `eq_ref` appears in JOINs where each row from the left table matches exactly one row via a unique index on the right table.

**`range` is acceptable.** Means the index is used for a bounded range scan. Reasonable for selective ranges.

**`index` without `Using index` in Extra is deceptive.** It means a full index scan — still reads every leaf node of the index, just avoids the row heap. Not as bad as `ALL` but still O(n).


## Transactions and Isolation Levels

InnoDB supports all four SQL standard isolation levels. The default is `REPEATABLE READ`.

| Level | Dirty Read | Non-repeatable Read | Phantom Read |
| --- | --- | --- | --- |
| `READ UNCOMMITTED` | yes | yes | yes |
| `READ COMMITTED` | no | yes | yes |
| `REPEATABLE READ` | no | no | no* |
| `SERIALIZABLE` | no | no | no |

*InnoDB's REPEATABLE READ prevents phantoms for consistent (snapshot) reads but not for locking reads (`SELECT ... FOR UPDATE`/`FOR SHARE`).

**Autocommit and performance.** With `autocommit=1` (the default), every single-statement DML is its own transaction, including the overhead of flushing the transaction log. For bulk loads, wrap batches in explicit transactions:

```sql
START TRANSACTION;
-- 500–1000 INSERTs
COMMIT;
```

This amortises log flush cost across the batch.

**`innodb_flush_log_at_trx_commit`.** Controls the durability/performance tradeoff:

- `1` (default): flush and sync to disk on every commit. Fully durable; slowest.
- `2`: write to OS buffer on commit, sync once per second. Survives MySQL crash but not OS crash.
- `0`: write and sync once per second. Fastest; can lose up to 1 second of committed transactions.

For replicas or non-critical staging environments, `2` is a reasonable compromise.

**`sync_binlog`.** Set to `1` for full durability (sync binlog on every commit). Combined with `innodb_flush_log_at_trx_commit=1` this is "double-1" — the safe baseline for primary servers.

**Deadlocks are normal, not a bug.** InnoDB detects deadlocks and rolls back the transaction with the least undo work. Always retry on `ER_LOCK_DEADLOCK` (error 1213). Reduce deadlock frequency by acquiring locks in a consistent order across transactions.


## Replication

### Primary / Replica (async, default)

The primary writes changes to the binary log. Each replica has an I/O thread that streams the binlog and a SQL thread that applies it. Lag is the key operational metric — replicas are eventually consistent, not synchronous.

**ROW-based binlog** (`binlog_format=ROW`) replicates the actual before/after row images rather than the SQL statement. More reliable — statement-based replication breaks on non-deterministic functions like `NOW()`, `RAND()`, `UUID()`. Always use ROW or MIXED (which falls back to ROW for unsafe statements).

**GTID replication** (`gtid_mode=ON`, `enforce_gtid_consistency=ON`) assigns a globally unique ID to every transaction. Makes failover and replica re-pointing trivial — no need to calculate binlog file+position manually. Use GTIDs for any new replication setup.

**Semi-synchronous replication** (`rpl_semi_sync_master_enabled=ON`): the primary waits for at least one replica to acknowledge receipt of the binlog event before committing. Prevents data loss on primary crash at the cost of slightly higher write latency. Available in MySQL via plugin; built-in on MariaDB.

### Group Replication / InnoDB Cluster (MySQL)

Multi-master with automatic failover. The cluster uses Paxos consensus to agree on transaction order. Use MySQL Shell + InnoDB Cluster for managed setup. Appropriate when you need automatic primary election without an external orchestrator.

### Galera Cluster (MariaDB)

Synchronous multi-master replication using write-set certification. Every node can accept writes. Causality is guaranteed — reads on any node are always current. Tradeoff: write latency scales with network round-trip time across nodes, and large transactions can generate flow control stalls.


## Window Functions

Both MySQL 8.0+ and MariaDB 10.2+ support SQL window functions.

```sql
-- Running total
SELECT
  order_date,
  amount,
  SUM(amount) OVER (ORDER BY order_date ROWS UNBOUNDED PRECEDING) AS running_total
FROM orders;

-- Rank within partition
SELECT
  department,
  employee,
  salary,
  RANK() OVER (PARTITION BY department ORDER BY salary DESC) AS dept_rank
FROM employees;

-- LAG/LEAD for adjacent row comparison
SELECT
  dt,
  revenue,
  revenue - LAG(revenue) OVER (ORDER BY dt) AS day_over_day
FROM daily_revenue;
```

Common window functions: `ROW_NUMBER()`, `RANK()`, `DENSE_RANK()`, `NTILE()`, `LEAD()`, `LAG()`, `FIRST_VALUE()`, `LAST_VALUE()`, `NTH_VALUE()`, aggregate functions (`SUM`, `AVG`, `COUNT`, `MIN`, `MAX`) with an OVER clause.

**Frame clauses:** `ROWS` vs `RANGE`. `ROWS BETWEEN 6 PRECEDING AND CURRENT ROW` counts physical rows. `RANGE BETWEEN INTERVAL 7 DAY PRECEDING AND CURRENT ROW` uses value-based boundaries — useful for rolling time windows with gaps.


## MySQL 8 vs MariaDB: Key Differences

| Area | MySQL 8 | MariaDB 10/11 |
| --- | --- | --- |
| Metadata storage | Native Data Dictionary (InnoDB-based, no `.frm` files) | `.frm` files (legacy) |
| JSON | Native binary type, `->` / `->>` operators, `JSON_TABLE()` | `LONGTEXT` + JSON functions; no `->/->>`, no `JSON_TABLE()` |
| Thread pooling | Enterprise Edition only | Included in open-source |
| Replication HA | InnoDB Cluster + Group Replication (built-in) | Galera Cluster (built-in) |
| Temporal tables | Not supported | System-versioned tables (10.3+): automatic row history |
| `CREATE OR REPLACE PROCEDURE` | Not supported; must `DROP` first | Supported (10.1+) |
| Storage engines | InnoDB (practical default) | InnoDB + Aria + MyRocks + ColumnStore |
| Oracle SQL compat | Minimal | Sequences, PL/SQL-like syntax (10.3+) |
| Optimizer | MySQL optimizer, hash join (8.0.18+) | Optimizer with additional strategies; historically faster on some join patterns |
| Licensing | Dual (GPL community + commercial) | Fully GPL |
| Query cache | Removed in 8.0 | Retained but off by default; do not enable |

### Compatibility gotchas when migrating MySQL → MariaDB or vice versa

- `->` and `->>` JSON operators exist only in MySQL. Replace with `JSON_UNQUOTE(JSON_EXTRACT(...))`.
- `JSON_TABLE()` does not exist in MariaDB. Rewrite as procedural code or application-side processing.
- `GTID` implementation differs slightly — MariaDB uses `domain_id-server_id-sequence` format vs MySQL's `server_uuid:sequence`. Cross-engine replication with GTIDs requires careful handling.
- System-versioned tables (`FOR SYSTEM_TIME AS OF ...`) are MariaDB-only. No MySQL equivalent.
- MySQL 8 requires `caching_sha2_password` auth plugin by default. Older MySQL clients and MariaDB clients may fail. Either change `default_authentication_plugin=mysql_native_password` or upgrade all clients.


---


# SQLite Production Patterns

## When SQLite is appropriate for production

SQLite is the right choice when:

- The database lives on a single machine alongside the application (embedded pattern).
- Read traffic dominates or writes are serializable at acceptable throughput (thousands of writes/second is achievable with WAL).
- You want zero network round-trips for queries — latency is memory-speed, not TCP-speed.
- The dataset fits in storage on one node (SQLite handles multi-GB files fine; multi-TB is pushing it).
- You're deploying at the edge where spinning up Postgres is impractical or expensive.

SQLite is the wrong choice when:

- You need true horizontal write scaling across multiple nodes simultaneously.
- Multiple machines must write to the same database concurrently (network filesystems break WAL).
- Transactions routinely exceed ~100 MB (rollback journal modes are faster; WAL can fail above 1 GB).
- You need built-in role-based access control (SQLite has no auth layer — implement it in the application).


## PRAGMA tuning reference

Apply these after opening the connection, before any queries. Most are per-connection and do not persist.

```sql
-- Enable WAL (persists to disk)
PRAGMA journal_mode = WAL;

-- Wait up to 5s for a locked database instead of immediately returning SQLITE_BUSY
PRAGMA busy_timeout = 5000;

-- NORMAL: flush to OS on commit but don't wait for OS to flush to disk.
-- Safe with WAL; slightly faster than FULL. Use FULL if you need power-loss durability.
PRAGMA synchronous = NORMAL;

-- 64 MB in-process page cache (negative = kibibytes)
PRAGMA cache_size = -64000;

-- Store temp tables and indices in RAM
PRAGMA temp_store = MEMORY;

-- Enforce foreign key constraints (off by default)
PRAGMA foreign_keys = ON;

-- Memory-mapped I/O: map up to 128 MB of the database file (tune to your dataset size)
PRAGMA mmap_size = 134217728;

-- Re-analyze query planner statistics after bulk changes
PRAGMA optimize;
```

`synchronous = NORMAL` with WAL is the recommended production balance. With WAL, a crash cannot corrupt the database even at NORMAL because the WAL itself provides the durability barrier. Use `FULL` only if you need explicit power-loss guarantees.


## Schema and query patterns

**Integer primary keys as rowid aliases.** Declaring `id INTEGER PRIMARY KEY` makes `id` an alias for SQLite's internal rowid — no separate B-tree needed, lookups by id are as fast as possible.

**WITHOUT ROWID tables.** For tables where the primary key is not an integer (e.g., a UUID or compound key) and you never need rowid-based access, `WITHOUT ROWID` stores rows directly in the primary key index, saving space and improving scan performance.

```sql
CREATE TABLE sessions (
    token TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
) WITHOUT ROWID;
```

**Strict tables.** SQLite's default type affinity is famously loose. For new tables, `STRICT` enforces column types:

```sql
CREATE TABLE products (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL,
    price REAL NOT NULL
) STRICT;
```

**Multi-column indexes.** SQLite can use a composite index for queries that filter the leftmost prefix of its columns:

```sql
CREATE INDEX idx_orders_user_status ON orders(user_id, status);
-- Usable for: WHERE user_id = ?
-- Usable for: WHERE user_id = ? AND status = ?
-- NOT usable for: WHERE status = ?  (skips the leftmost column)
```

**EXPLAIN QUERY PLAN.** Check index usage before deploying expensive queries:

```sql
EXPLAIN QUERY PLAN
SELECT * FROM orders WHERE user_id = 42 AND status = 'open';
```

Look for `SCAN` (bad on large tables) vs `SEARCH … USING INDEX` (good).


## JSON functions

SQLite 3.38+ includes `json_*` functions and the `->` / `->>` operators for inline JSON columns.

```sql
-- Store structured data in a JSON column
CREATE TABLE events (
    id      INTEGER PRIMARY KEY,
    type    TEXT NOT NULL,
    payload TEXT NOT NULL  -- JSON blob
);

-- Extract a field (returns JSON)
SELECT payload -> '$.user_id' FROM events;

-- Extract a field as a native SQL value
SELECT payload ->> '$.user_id' FROM events WHERE type = 'login';

-- Index a JSON field for fast lookup
CREATE INDEX idx_events_user ON events(payload ->> '$.user_id');

-- Aggregate into a JSON array
SELECT json_group_array(id) FROM events WHERE type = 'login';
```

JSON columns work best for semi-structured or variable attributes. For columns you query or filter frequently, promote them to real columns and index them normally.


## Replication: Litestream

[Litestream](https://litestream.io/) runs as a sidecar process that tails the SQLite WAL and streams changes to S3-compatible object storage every second. This gives you:

- **Point-in-time recovery** — restore to any second in your retention window.
- **Off-site durability** — database survives the loss of the host.
- **Zero application changes** — Litestream intercepts at the filesystem level.

What Litestream is **not**: it is not a live read replica. It is a streaming backup tool.

Minimal `litestream.yml`:

```yaml
dbs:
  - path: /data/app.db
    replicas:
      - url: s3://my-bucket/app.db
        retention: 72h
        sync-interval: 1s
```

Restore:

```bash
litestream restore -o /data/app.db s3://my-bucket/app.db
```

Deployment pattern: package Litestream alongside the application container. On startup, restore from S3 if no local database exists; then start the app. On shutdown, Litestream flushes any pending WAL frames.


## Quick-start PRAGMA block

Copy this into your connection initialization:

```sql
PRAGMA journal_mode  = WAL;
PRAGMA busy_timeout  = 5000;
PRAGMA synchronous   = NORMAL;
PRAGMA cache_size    = -64000;
PRAGMA foreign_keys  = ON;
PRAGMA temp_store    = MEMORY;
PRAGMA mmap_size     = 134217728;
```

These seven lines cover the majority of production SQLite tuning needs.
