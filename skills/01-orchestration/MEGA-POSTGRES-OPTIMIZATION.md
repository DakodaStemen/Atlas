---
name: Add Indexes on WHERE and JOIN Columns
description: ## Add Indexes on WHERE and JOIN Columns
 
 Queries filtering or joining on unindexed columns cause full table scans, which become exponentially slower as tables grow.
tags: indexes, performance, sequential-scan, query-optimization
---

## Add Indexes on WHERE and JOIN Columns

Queries filtering or joining on unindexed columns cause full table scans, which become exponentially slower as tables grow.

### Incorrect (sequential scan on large table)

```sql
-- No index on customer_id causes full table scan
select * from orders where customer_id = 123;

-- EXPLAIN shows: Seq Scan on orders (cost=0.00..25000.00 rows=100 width=85)
```

#### Correct (index scan)

```sql
-- Create index on frequently filtered column
create index orders_customer_id_idx on orders (customer_id);

select * from orders where customer_id = 123;

-- EXPLAIN shows: Index Scan using orders_customer_id_idx (cost=0.42..8.44 rows=100 width=85)
```

For JOIN columns, always index the foreign key side:

```sql
-- Index the referencing column
create index orders_customer_id_idx on orders (customer_id);

select c.name, o.total
from customers c
join orders o on o.customer_id = c.id;
```

Reference: [Query Optimization](https://supabase.com/docs/guides/database/query-optimization)

## When to use

Use when the user asks about or needs: Add Indexes on WHERE and JOIN Columns.
﻿---
name: Batch INSERT Statements for Bulk Data
description: ## Batch INSERT Statements for Bulk Data
 
 Individual INSERT statements have high overhead. Batch multiple rows in single statements or use COPY.
tags: batch, insert, bulk, performance, copy
---

## Batch INSERT Statements for Bulk Data

Individual INSERT statements have high overhead. Batch multiple rows in single statements or use COPY.

### Incorrect (individual inserts)

```sql
-- Each insert is a separate transaction and round trip
insert into events (user_id, action) values (1, 'click');
insert into events (user_id, action) values (1, 'view');
insert into events (user_id, action) values (2, 'click');
-- ... 1000 more individual inserts

-- 1000 inserts = 1000 round trips = slow
```

#### Correct (batch insert)

```sql
-- Multiple rows in single statement
insert into events (user_id, action) values
  (1, 'click'),
  (1, 'view'),
  (2, 'click'),
  -- ... up to ~1000 rows per batch
  (999, 'view');

-- One round trip for 1000 rows
```

For large imports, use COPY:

```sql
-- COPY is fastest for bulk loading
copy events (user_id, action, created_at)
from '/path/to/data.csv'
with (format csv, header true);

-- Or from stdin in application
copy events (user_id, action) from stdin with (format csv);
1,click
1,view
2,click
\.
```

Reference: [COPY](https://www.postgresql.org/docs/current/sql-copy.html)

## When to use

Use when the user asks about or needs: Batch INSERT Statements for Bulk Data.
﻿---
name: Choose the Right Index Type for Your Data
description: ## Choose the Right Index Type for Your Data
 
 Different index types excel at different query patterns. The default B-tree isn't always optimal.
tags: indexes, btree, gin, gist, brin, hash, index-types
---

## Choose the Right Index Type for Your Data

Different index types excel at different query patterns. The default B-tree isn't always optimal.

### Incorrect (B-tree for JSONB containment)

```sql
-- B-tree cannot optimize containment operators
create index products_attrs_idx on products (attributes);
select * from products where attributes @> '{"color": "red"}';
-- Full table scan - B-tree doesn't support @> operator
```

#### Correct (GIN for JSONB)

```sql
-- GIN supports @>, ?, ?&, ?| operators
create index products_attrs_idx on products using gin (attributes);
select * from products where attributes @> '{"color": "red"}';
```

Index type guide:

```sql
-- B-tree (default): =, <, >, BETWEEN, IN, IS NULL
create index users_created_idx on users (created_at);

-- GIN: arrays, JSONB, full-text search
create index posts_tags_idx on posts using gin (tags);

-- GiST: geometric data, range types, nearest-neighbor (KNN) queries
create index locations_idx on places using gist (location);

-- BRIN: large time-series tables (10-100x smaller)
create index events_time_idx on events using brin (created_at);

-- Hash: equality-only (slightly faster than B-tree for =)
create index sessions_token_idx on sessions using hash (token);
```

Reference: [Index Types](https://www.postgresql.org/docs/current/indexes-types.html)

## When to use

Use when the user asks about or needs: Choose the Right Index Type for Your Data.
﻿---
name: Create Composite Indexes for Multi-Column Queries
description: ## Create Composite Indexes for Multi-Column Queries
 
 When queries filter on multiple columns, a composite index is more efficient than separate single-column indexes.
tags: indexes, composite-index, multi-column, query-optimization
---

## Create Composite Indexes for Multi-Column Queries

When queries filter on multiple columns, a composite index is more efficient than separate single-column indexes.

### Incorrect (separate indexes require bitmap scan)

```sql
-- Two separate indexes
create index orders_status_idx on orders (status);
create index orders_created_idx on orders (created_at);

-- Query must combine both indexes (slower)
select * from orders where status = 'pending' and created_at > '2024-01-01';
```

#### Correct (composite index)

```sql
-- Single composite index (leftmost column first for equality checks)
create index orders_status_created_idx on orders (status, created_at);

-- Query uses one efficient index scan
select * from orders where status = 'pending' and created_at > '2024-01-01';
```

**Column order matters** - place equality columns first, range columns last:

```sql
-- Good: status (=) before created_at (>)
create index idx on orders (status, created_at);

-- Works for: WHERE status = 'pending'
-- Works for: WHERE status = 'pending' AND created_at > '2024-01-01'
-- Does NOT work for: WHERE created_at > '2024-01-01' (leftmost prefix rule)
```

Reference: [Multicolumn Indexes](https://www.postgresql.org/docs/current/indexes-multicolumn.html)

## When to use

Use when the user asks about or needs: Create Composite Indexes for Multi-Column Queries.
﻿---
name: Index Foreign Key Columns
description: ## Index Foreign Key Columns
 
 Postgres does not automatically index foreign key columns. Missing indexes cause slow JOINs and CASCADE operations.
tags: foreign-key, indexes, joins, schema
---

## Index Foreign Key Columns

Postgres does not automatically index foreign key columns. Missing indexes cause slow JOINs and CASCADE operations.

### Incorrect (unindexed foreign key)

```sql
create table orders (
  id bigint generated always as identity primary key,
  customer_id bigint references customers(id) on delete cascade,
  total numeric(10,2)
);

-- No index on customer_id!
-- JOINs and ON DELETE CASCADE both require full table scan
select * from orders where customer_id = 123;  -- Seq Scan
delete from customers where id = 123;          -- Locks table, scans all orders
```

#### Correct (indexed foreign key)

```sql
create table orders (
  id bigint generated always as identity primary key,
  customer_id bigint references customers(id) on delete cascade,
  total numeric(10,2)
);

-- Always index the FK column
create index orders_customer_id_idx on orders (customer_id);

-- Now JOINs and cascades are fast
select * from orders where customer_id = 123;  -- Index Scan
delete from customers where id = 123;          -- Uses index, fast cascade
```

Find missing FK indexes:

```sql
select
  conrelid::regclass as table_name,
  a.attname as fk_column
from pg_constraint c
join pg_attribute a on a.attrelid = c.conrelid and a.attnum = any(c.conkey)
where c.contype = 'f'
  and not exists (
    select 1 from pg_index i
    where i.indrelid = c.conrelid and a.attnum = any(i.indkey)
  );
```

Reference: [Foreign Keys](https://www.postgresql.org/docs/current/ddl-constraints.html#DDL-CONSTRAINTS-FK)

## When to use

Use when the user asks about or needs: Index Foreign Key Columns.
﻿---
name: Index JSONB Columns for Efficient Querying
description: ## Index JSONB Columns for Efficient Querying
 
 JSONB queries without indexes scan the entire table. Use GIN indexes for containment queries.
tags: jsonb, gin, indexes, json
---

## Index JSONB Columns for Efficient Querying

JSONB queries without indexes scan the entire table. Use GIN indexes for containment queries.

### Incorrect (no index on JSONB)

```sql
create table products (
  id bigint primary key,
  attributes jsonb
);

-- Full table scan for every query
select * from products where attributes @> '{"color": "red"}';
select * from products where attributes->>'brand' = 'Nike';
```

#### Correct (GIN index for JSONB)

```sql
-- GIN index for containment operators (@>, ?, ?&, ?|)
create index products_attrs_gin on products using gin (attributes);

-- Now containment queries use the index
select * from products where attributes @> '{"color": "red"}';

-- For specific key lookups, use expression index
create index products_brand_idx on products ((attributes->>'brand'));
select * from products where attributes->>'brand' = 'Nike';
```

Choose the right operator class:

```sql
-- jsonb_ops (default): supports all operators, larger index
create index idx1 on products using gin (attributes);

-- jsonb_path_ops: only @> operator, but 2-3x smaller index
create index idx2 on products using gin (attributes jsonb_path_ops);
```

Reference: [JSONB Indexes](https://www.postgresql.org/docs/current/datatype-json.html#JSON-INDEXING)

## When to use

Use when the user asks about or needs: Index JSONB Columns for Efficient Querying.
﻿---
name: Select Optimal Primary Key Strategy
description: ## Select Optimal Primary Key Strategy
 
 Primary key choice affects insert performance, index size, and replication
tags: primary-key, identity, uuid, serial, schema
---

## Select Optimal Primary Key Strategy

Primary key choice affects insert performance, index size, and replication
efficiency.

### Incorrect (problematic PK choices)

```sql
-- identity is the SQL-standard approach
create table users (
  id serial primary key  -- Works, but IDENTITY is recommended
);

-- Random UUIDs (v4) cause index fragmentation
create table orders (
  id uuid default gen_random_uuid() primary key  -- UUIDv4 = random = scattered inserts
);
```

#### Correct (optimal PK strategies)

```sql
-- Use IDENTITY for sequential IDs (SQL-standard, best for most cases)
create table users (
  id bigint generated always as identity primary key
);

-- For distributed systems needing UUIDs, use UUIDv7 (time-ordered)
-- Requires pg_uuidv7 extension: create extension pg_uuidv7;
create table orders (
  id uuid default uuid_generate_v7() primary key  -- Time-ordered, no fragmentation
);

-- Alternative: time-prefixed IDs for sortable, distributed IDs (no extension needed)
create table events (
  id text default concat(
    to_char(now() at time zone 'utc', 'YYYYMMDDHH24MISSMS'),
    gen_random_uuid()::text
  ) primary key
);
```

Guidelines:

- Single database: `bigint identity` (sequential, 8 bytes, SQL-standard)
- Distributed/exposed IDs: UUIDv7 (requires pg_uuidv7) or ULID (time-ordered, no
  fragmentation)
- `serial` works but `identity` is SQL-standard and preferred for new
  applications
- Avoid random UUIDs (v4) as primary keys on large tables (causes index
  fragmentation)

Reference:
[Identity Columns](https://www.postgresql.org/docs/current/sql-createtable.html#SQL-CREATETABLE-PARMS-GENERATED-IDENTITY)

## When to use

Use when the user asks about or needs: Select Optimal Primary Key Strategy.
﻿---
name: Use Advisory Locks for Application-Level Locking
description: ## Use Advisory Locks for Application-Level Locking
 
 Advisory locks provide application-level coordination without requiring database rows to lock.
tags: advisory-locks, coordination, application-locks
---

## Use Advisory Locks for Application-Level Locking

Advisory locks provide application-level coordination without requiring database rows to lock.

### Incorrect (creating rows just for locking)

```sql
-- Creating dummy rows to lock on
create table resource_locks (
  resource_name text primary key
);

insert into resource_locks values ('report_generator');

-- Lock by selecting the row
select * from resource_locks where resource_name = 'report_generator' for update;
```

#### Correct (advisory locks)

```sql
-- Session-level advisory lock (released on disconnect or unlock)
select pg_advisory_lock(hashtext('report_generator'));
-- ... do exclusive work ...
select pg_advisory_unlock(hashtext('report_generator'));

-- Transaction-level lock (released on commit/rollback)
begin;
select pg_advisory_xact_lock(hashtext('daily_report'));
-- ... do work ...
commit;  -- Lock automatically released
```

Try-lock for non-blocking operations:

```sql
-- Returns immediately with true/false instead of waiting
select pg_try_advisory_lock(hashtext('resource_name'));

-- Use in application
if (acquired) {
  -- Do work
  select pg_advisory_unlock(hashtext('resource_name'));
} else {
  -- Skip or retry later
}
```

Reference: [Advisory Locks](https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS)

## When to use

Use when the user asks about or needs: Use Advisory Locks for Application-Level Locking.
﻿---
name: Use Covering Indexes to Avoid Table Lookups
description: ## Use Covering Indexes to Avoid Table Lookups
 
 Covering indexes include all columns needed by a query, enabling index-only scans that skip the table entirely.
tags: indexes, covering-index, include, index-only-scan
---

## Use Covering Indexes to Avoid Table Lookups

Covering indexes include all columns needed by a query, enabling index-only scans that skip the table entirely.

### Incorrect (index scan + heap fetch)

```sql
create index users_email_idx on users (email);

-- Must fetch name and created_at from table heap
select email, name, created_at from users where email = 'user@example.com';
```

#### Correct (index-only scan with INCLUDE)

```sql
-- Include non-searchable columns in the index
create index users_email_idx on users (email) include (name, created_at);

-- All columns served from index, no table access needed
select email, name, created_at from users where email = 'user@example.com';
```

Use INCLUDE for columns you SELECT but don't filter on:

```sql
-- Searching by status, but also need customer_id and total
create index orders_status_idx on orders (status) include (customer_id, total);

select status, customer_id, total from orders where status = 'shipped';
```

Reference: [Index-Only Scans](https://www.postgresql.org/docs/current/indexes-index-only-scans.html)

## When to use

Use when the user asks about or needs: Use Covering Indexes to Avoid Table Lookups.
﻿---
name: Use Partial Indexes for Filtered Queries
description: ## Use Partial Indexes for Filtered Queries
 
 Partial indexes only include rows matching a WHERE condition, making them smaller and faster when queries consistently filter on the same condition.
tags: indexes, partial-index, query-optimization, storage
---

## Use Partial Indexes for Filtered Queries

Partial indexes only include rows matching a WHERE condition, making them smaller and faster when queries consistently filter on the same condition.

### Incorrect (full index includes irrelevant rows)

```sql
-- Index includes all rows, even soft-deleted ones
create index users_email_idx on users (email);

-- Query always filters active users
select * from users where email = 'user@example.com' and deleted_at is null;
```

#### Correct (partial index matches query filter)

```sql
-- Index only includes active users
create index users_active_email_idx on users (email)
where deleted_at is null;

-- Query uses the smaller, faster index
select * from users where email = 'user@example.com' and deleted_at is null;
```

Common use cases for partial indexes:

```sql
-- Only pending orders (status rarely changes once completed)
create index orders_pending_idx on orders (created_at)
where status = 'pending';

-- Only non-null values
create index products_sku_idx on products (sku)
where sku is not null;
```

Reference: [Partial Indexes](https://www.postgresql.org/docs/current/indexes-partial.html)

## When to use

Use when the user asks about or needs: Use Partial Indexes for Filtered Queries.
﻿---
name: Use Prepared Statements Correctly with Pooling
description: ## Use Prepared Statements Correctly with Pooling
 
 Prepared statements are tied to individual database connections. In transaction-mode pooling, connections are shared, causing conflicts.
tags: prepared-statements, connection-pooling, transaction-mode
---

## Use Prepared Statements Correctly with Pooling

Prepared statements are tied to individual database connections. In transaction-mode pooling, connections are shared, causing conflicts.

### Incorrect (named prepared statements with transaction pooling)

```sql
-- Named prepared statement
prepare get_user as select * from users where id = $1;

-- In transaction mode pooling, next request may get different connection
execute get_user(123);
-- ERROR: prepared statement "get_user" does not exist
```

#### Correct (use unnamed statements or session mode)

```sql
-- Option 1: Use unnamed prepared statements (most ORMs do this automatically)
-- The query is prepared and executed in a single protocol message

-- Option 2: Deallocate after use in transaction mode
prepare get_user as select * from users where id = $1;
execute get_user(123);
deallocate get_user;

-- Option 3: Use session mode pooling (port 5432 vs 6543)
-- Connection is held for entire session, prepared statements persist
```

Check your driver settings:

```sql
-- Many drivers use prepared statements by default
-- Node.js pg: { prepare: false } to disable
-- JDBC: prepareThreshold=0 to disable
```

Reference: [Prepared Statements with Pooling](https://supabase.com/docs/guides/database/connecting-to-postgres#connection-pool-modes)

## When to use

Use when the user asks about or needs: Use Prepared Statements Correctly with Pooling.
﻿---
name: Use SKIP LOCKED for Non-Blocking Queue Processing
description: ## Use SKIP LOCKED for Non-Blocking Queue Processing
 
 When multiple workers process a queue, SKIP LOCKED allows workers to process different rows without waiting.
tags: skip-locked, queue, workers, concurrency
---

## Use SKIP LOCKED for Non-Blocking Queue Processing

When multiple workers process a queue, SKIP LOCKED allows workers to process different rows without waiting.

### Incorrect (workers block each other)

```sql
-- Worker 1 and Worker 2 both try to get next job
begin;
select * from jobs where status = 'pending' order by created_at limit 1 for update;
-- Worker 2 waits for Worker 1's lock to release!
```

#### Correct (SKIP LOCKED for parallel processing)

```sql
-- Each worker skips locked rows and gets the next available
begin;
select * from jobs
where status = 'pending'
order by created_at
limit 1
for update skip locked;

-- Worker 1 gets job 1, Worker 2 gets job 2 (no waiting)

update jobs set status = 'processing' where id = $1;
commit;
```

Complete queue pattern:

```sql
-- Atomic claim-and-update in one statement
update jobs
set status = 'processing', worker_id = $1, started_at = now()
where id = (
  select id from jobs
  where status = 'pending'
  order by created_at
  limit 1
  for update skip locked
)
returning *;
```

Reference: [SELECT FOR UPDATE SKIP LOCKED](https://www.postgresql.org/docs/current/sql-select.html#SQL-FOR-UPDATE-SHARE)

## When to use

Use when the user asks about or needs: Use SKIP LOCKED for Non-Blocking Queue Processing.
﻿---
name: Use tsvector for Full-Text Search
description: ## Use tsvector for Full-Text Search
 
 LIKE with wildcards can't use indexes. Full-text search with tsvector is orders of magnitude faster.
tags: full-text-search, tsvector, gin, search
---

## Use tsvector for Full-Text Search

LIKE with wildcards can't use indexes. Full-text search with tsvector is orders of magnitude faster.

### Incorrect (LIKE pattern matching)

```sql
-- Cannot use index, scans all rows
select * from articles where content like '%postgresql%';

-- Case-insensitive makes it worse
select * from articles where lower(content) like '%postgresql%';
```

#### Correct (full-text search with tsvector)

```sql
-- Add tsvector column and index
alter table articles add column search_vector tsvector
  generated always as (to_tsvector('english', coalesce(title,'') || ' ' || coalesce(content,''))) stored;

create index articles_search_idx on articles using gin (search_vector);

-- Fast full-text search
select * from articles
where search_vector @@ to_tsquery('english', 'postgresql & performance');

-- With ranking
select *, ts_rank(search_vector, query) as rank
from articles, to_tsquery('english', 'postgresql') query
where search_vector @@ query
order by rank desc;
```

Search multiple terms:

```sql
-- AND: both terms required
to_tsquery('postgresql & performance')

-- OR: either term
to_tsquery('postgresql | mysql')

-- Prefix matching
to_tsquery('post:*')
```

Reference: [Full Text Search](https://supabase.com/docs/guides/database/full-text-search)

## When to use

Use when the user asks about or needs: Use tsvector for Full-Text Search.
﻿---
name: Use UPSERT for Insert-or-Update Operations
description: ## Use UPSERT for Insert-or-Update Operations
 
 Using separate SELECT-then-INSERT/UPDATE creates race conditions. Use INSERT ... ON CONFLICT for atomic upserts.
tags: upsert, on-conflict, insert, update
---

## Use UPSERT for Insert-or-Update Operations

Using separate SELECT-then-INSERT/UPDATE creates race conditions. Use INSERT ... ON CONFLICT for atomic upserts.

### Incorrect (check-then-insert race condition)

```sql
-- Race condition: two requests check simultaneously
select * from settings where user_id = 123 and key = 'theme';
-- Both find nothing

-- Both try to insert
insert into settings (user_id, key, value) values (123, 'theme', 'dark');
-- One succeeds, one fails with duplicate key error!
```

#### Correct (atomic UPSERT)

```sql
-- Single atomic operation
insert into settings (user_id, key, value)
values (123, 'theme', 'dark')
on conflict (user_id, key)
do update set value = excluded.value, updated_at = now();

-- Returns the inserted/updated row
insert into settings (user_id, key, value)
values (123, 'theme', 'dark')
on conflict (user_id, key)
do update set value = excluded.value
returning *;
```

Insert-or-ignore pattern:

```sql
-- Insert only if not exists (no update)
insert into page_views (page_id, user_id)
values (1, 123)
on conflict (page_id, user_id) do nothing;
```

Reference: [INSERT ON CONFLICT](https://www.postgresql.org/docs/current/sql-insert.html#SQL-ON-CONFLICT)

## When to use

Use when the user asks about or needs: Use UPSERT for Insert-or-Update Operations.
﻿---
name: Keep Transactions Short to Reduce Lock Contention
description: ## Keep Transactions Short to Reduce Lock Contention
 
 Long-running transactions hold locks that block other queries. Keep transactions as short as possible.
tags: transactions, locking, contention, performance
---

## Keep Transactions Short to Reduce Lock Contention

Long-running transactions hold locks that block other queries. Keep transactions as short as possible.

### Incorrect (long transaction with external calls)

```sql
begin;
select * from orders where id = 1 for update;  -- Lock acquired

-- Application makes HTTP call to payment API (2-5 seconds)
-- Other queries on this row are blocked!

update orders set status = 'paid' where id = 1;
commit;  -- Lock held for entire duration
```

#### Correct (minimal transaction scope)

```sql
-- Validate data and call APIs outside transaction
-- Application: response = await paymentAPI.charge(...)

-- Only hold lock for the actual update
begin;
update orders
set status = 'paid', payment_id = $1
where id = $2 and status = 'pending'
returning *;
commit;  -- Lock held for milliseconds
```

Use `statement_timeout` to prevent runaway transactions:

```sql
-- Abort queries running longer than 30 seconds
set statement_timeout = '30s';

-- Or per-session
set local statement_timeout = '5s';
```

Reference: [Transaction Management](https://www.postgresql.org/docs/current/tutorial-transactions.html)

## When to use

Use when the user asks about or needs: Keep Transactions Short to Reduce Lock Contention.
﻿---
name: Prevent Deadlocks with Consistent Lock Ordering
description: ## Prevent Deadlocks with Consistent Lock Ordering
 
 Deadlocks occur when transactions lock resources in different orders. Always
tags: deadlocks, locking, transactions, ordering
---

## Prevent Deadlocks with Consistent Lock Ordering

Deadlocks occur when transactions lock resources in different orders. Always
acquire locks in a consistent order.

### Incorrect (inconsistent lock ordering)

```sql
-- Transaction A                    -- Transaction B
begin;                              begin;
update accounts                     update accounts
set balance = balance - 100         set balance = balance - 50
where id = 1;                       where id = 2;  -- B locks row 2

update accounts                     update accounts
set balance = balance + 100         set balance = balance + 50
where id = 2;  -- A waits for B     where id = 1;  -- B waits for A

-- DEADLOCK! Both waiting for each other
```

#### Correct (lock rows in consistent order first)

```sql
-- Explicitly acquire locks in ID order before updating
begin;
select * from accounts where id in (1, 2) order by id for update;

-- Now perform updates in any order - locks already held
update accounts set balance = balance - 100 where id = 1;
update accounts set balance = balance + 100 where id = 2;
commit;
```

Alternative: use a single statement to update atomically:

```sql
-- Single statement acquires all locks atomically
begin;
update accounts
set balance = balance + case id
  when 1 then -100
  when 2 then 100
end
where id in (1, 2);
commit;
```

Detect deadlocks in logs:

```sql
-- Check for recent deadlocks
select * from pg_stat_database where deadlocks > 0;

-- Enable deadlock logging
set log_lock_waits = on;
set deadlock_timeout = '1s';
```

Reference:
[Deadlocks](https://www.postgresql.org/docs/current/explicit-locking.html#LOCKING-DEADLOCKS)

## When to use

Use when the user asks about or needs: Prevent Deadlocks with Consistent Lock Ordering.


---

<!-- merged from: choose-appropriate-data-types.md -->

﻿---
name: Choose Appropriate Data Types
description: ## Choose Appropriate Data Types
 
 Using the right data types reduces storage, improves query performance, and prevents bugs.
tags: data-types, schema, storage, performance
---

## Choose Appropriate Data Types

Using the right data types reduces storage, improves query performance, and prevents bugs.

### Incorrect (wrong data types)

```sql
create table users (
  id int,                    -- Will overflow at 2.1 billion
  email varchar(255),        -- Unnecessary length limit
  created_at timestamp,      -- Missing timezone info
  is_active varchar(5),      -- String for boolean
  price varchar(20)          -- String for numeric
);
```

#### Correct (appropriate data types)

```sql
create table users (
  id bigint generated always as identity primary key,  -- 9 quintillion max
  email text,                     -- No artificial limit, same performance as varchar
  created_at timestamptz,         -- Always store timezone-aware timestamps
  is_active boolean default true, -- 1 byte vs variable string length
  price numeric(10,2)             -- Exact decimal arithmetic
);
```

Key guidelines:

```sql
-- IDs: use bigint, not int (future-proofing)
-- Strings: use text, not varchar(n) unless constraint needed
-- Time: use timestamptz, not timestamp
-- Money: use numeric, not float (precision matters)
-- Enums: use text with check constraint or create enum type
```

Reference: [Data Types](https://www.postgresql.org/docs/current/datatype.html)


---

<!-- merged from: configure-idle-connection-timeouts.md -->

﻿---
name: Configure Idle Connection Timeouts
description: ## Configure Idle Connection Timeouts
 
 Idle connections waste resources. Configure timeouts to automatically reclaim them.
tags: connections, timeout, idle, resource-management
---

## Configure Idle Connection Timeouts

Idle connections waste resources. Configure timeouts to automatically reclaim them.

### Incorrect (connections held indefinitely)

```sql
-- No timeout configured
show idle_in_transaction_session_timeout;  -- 0 (disabled)

-- Connections stay open forever, even when idle
select pid, state, state_change, query
from pg_stat_activity
where state = 'idle in transaction';
-- Shows transactions idle for hours, holding locks
```

#### Correct (automatic cleanup of idle connections)

```sql
-- Terminate connections idle in transaction after 30 seconds
alter system set idle_in_transaction_session_timeout = '30s';

-- Terminate completely idle connections after 10 minutes
alter system set idle_session_timeout = '10min';

-- Reload configuration
select pg_reload_conf();
```

For pooled connections, configure at the pooler level:

```ini
# pgbouncer.ini
server_idle_timeout = 60
client_idle_timeout = 300
```

Reference: [Connection Timeouts](https://www.postgresql.org/docs/current/runtime-config-client.html#GUC-IDLE-IN-TRANSACTION-SESSION-TIMEOUT)


---

<!-- merged from: set-appropriate-connection-limits.md -->

﻿---
name: Set Appropriate Connection Limits
description: ## Set Appropriate Connection Limits
 
 Too many connections exhaust memory and degrade performance. Set limits based on available resources.
tags: connections, max-connections, limits, stability
---

## Set Appropriate Connection Limits

Too many connections exhaust memory and degrade performance. Set limits based on available resources.

### Incorrect (unlimited or excessive connections)

```sql
-- Default max_connections = 100, but often increased blindly
show max_connections;  -- 500 (way too high for 4GB RAM)

-- Each connection uses 1-3MB RAM
-- 500 connections * 2MB = 1GB just for connections!
-- Out of memory errors under load
```

#### Correct (calculate based on resources)

```sql
-- Formula: max_connections = (RAM in MB / 5MB per connection) - reserved
-- For 4GB RAM: (4096 / 5) - 10 = ~800 theoretical max
-- But practically, 100-200 is better for query performance

-- Recommended settings for 4GB RAM
alter system set max_connections = 100;

-- Also set work_mem appropriately
-- work_mem * max_connections should not exceed 25% of RAM
alter system set work_mem = '8MB';  -- 8MB * 100 = 800MB max
```

Monitor connection usage:

```sql
select count(*), state from pg_stat_activity group by state;
```

Reference: [Database Connections](https://supabase.com/docs/guides/platform/performance#connection-management)


---

<!-- merged from: eliminate-n1-queries-with-batch-loading.md -->

﻿---
name: Eliminate N+1 Queries with Batch Loading
description: ## Eliminate N+1 Queries with Batch Loading
 
 N+1 queries execute one query per item in a loop. Batch them into a single query using arrays or JOINs.
tags: n-plus-one, batch, performance, queries
---

## Eliminate N+1 Queries with Batch Loading

N+1 queries execute one query per item in a loop. Batch them into a single query using arrays or JOINs.

### Incorrect (N+1 queries)

```sql
-- First query: get all users
select id from users where active = true;  -- Returns 100 IDs

-- Then N queries, one per user
select * from orders where user_id = 1;
select * from orders where user_id = 2;
select * from orders where user_id = 3;
-- ... 97 more queries!

-- Total: 101 round trips to database
```

#### Correct (single batch query)

```sql
-- Collect IDs and query once with ANY
select * from orders where user_id = any(array[1, 2, 3, ...]);

-- Or use JOIN instead of loop
select u.id, u.name, o.*
from users u
left join orders o on o.user_id = u.id
where u.active = true;

-- Total: 1 round trip
```

Application pattern:

```sql
-- Instead of looping in application code:
-- for user in users: db.query("SELECT * FROM orders WHERE user_id = $1", user.id)

-- Pass array parameter:
select * from orders where user_id = any($1::bigint[]);
-- Application passes: [1, 2, 3, 4, 5, ...]
```

Reference: [N+1 Query Problem](https://supabase.com/docs/guides/database/query-optimization)