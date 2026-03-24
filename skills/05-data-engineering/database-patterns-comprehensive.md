---
name: database-patterns-comprehensive
description: Comprehensive database patterns covering Postgres performance, Supabase/RLS, schema migrations, caching (Redis), event sourcing/CQRS, data modeling, multi-tenancy, Cassandra, TimescaleDB, event-driven architecture, and general best practices. Use for any relational/NoSQL database design, optimization, or operations task.
domain: data-engineering
tags: [database, postgres, supabase, rls, migrations, caching, redis, event-sourcing, cqrs, multi-tenancy, cassandra, timescale, data-modeling]
triggers: database, postgres, supabase, RLS, migration, caching, redis, event sourcing, CQRS, data modeling, normalization, multi-tenancy, cassandra, timescaledb, event driven, database performance
---

# Database Patterns Comprehensive Guide

## 1. Postgres Performance

### Indexing

- Create indexes aligned with WHERE, JOIN, ORDER BY patterns. Use `EXPLAIN ANALYZE` to verify index usage.
- Composite indexes: column order matters (most selective first for equality, range columns last).
- Partial indexes: `CREATE INDEX idx ON orders(status) WHERE status = 'pending'` for filtered subsets.
- Use `pg_stat_user_indexes` to find unused indexes and remove them.

### Query Optimization

- Use `EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)` to diagnose slow queries.
- Avoid `SELECT *`; specify columns. Avoid functions on indexed columns in WHERE clauses.
- Use CTEs sparingly in older Postgres (pre-12 they're optimization fences). Prefer subqueries when performance matters.
- `VACUUM ANALYZE` regularly to maintain table statistics.

### Connection Management

- Use connection pooling (PgBouncer, Supavisor). Set pool size based on `max_connections` and workload.
- Transaction-mode pooling for short-lived queries. Session-mode only when using prepared statements or temp tables.

### Partitioning

- Use declarative partitioning for tables >1M rows or >1GB. Range partitioning by date is most common.
- Partition pruning requires the partition key in WHERE clause. Monitor partition sizes for balance.

## 2. Supabase and RLS

### Row-Level Security

- Enable RLS on every table: `ALTER TABLE t ENABLE ROW LEVEL SECURITY`.
- Create policies with `USING` (read filter) and `WITH CHECK` (write validation).
- Use `auth.uid()` for user-scoped access. Use `auth.jwt() ->> 'role'` for role-based access.

### RLS Performance

- Optimize RLS policies: avoid subqueries in policy expressions. Use `EXISTS` with indexed foreign keys instead of `IN` with subselects.
- Create indexes on columns used in policy expressions. Test policy performance with `EXPLAIN ANALYZE`.
- Consider security-definer functions for complex authorization logic (bypasses RLS within function).

### Best Practices

- Always test with `SET ROLE` to verify policies. Create separate policies for SELECT, INSERT, UPDATE, DELETE.
- Use `FORCE ROW LEVEL SECURITY` on table owner to apply policies even to the owner role.

## 3. Schema Migrations Safety

### Rules

- Never run DDL in a transaction with DML on large tables. Add columns as nullable first, then backfill, then add constraints.
- Use `CREATE INDEX CONCURRENTLY` to avoid locking. Use `ALTER TABLE ... ADD CONSTRAINT ... NOT VALID` then `VALIDATE CONSTRAINT` separately.
- Always have a rollback migration. Test against production-sized data.

### Adding Constraints Safely

```sql
-- Step 1: Add constraint without validation (instant, no lock)
ALTER TABLE orders ADD CONSTRAINT chk_amount CHECK (amount > 0) NOT VALID;

-- Step 2: Validate existing rows (allows concurrent reads/writes)
ALTER TABLE orders VALIDATE CONSTRAINT chk_amount;
```

### Migration Versioning

- Use numbered migrations (001_create_users.sql, 002_add_email.sql). Never modify a migration that has been applied to any environment. One concern per migration file.
- Tools: Flyway, Alembic, dbmate, golang-migrate, Prisma Migrate.

## 4. Caching Strategies

### Cache-Aside (Lazy Loading)

- Read: check cache → miss → read DB → populate cache → return. Write: update DB → invalidate cache.
- Most common pattern. Simple. Risk of stale data between write and invalidation.

### Write-Through

- Write: update cache and DB simultaneously. Guarantees cache consistency. Higher write latency.

### Write-Behind (Write-Back)

- Write: update cache → async write to DB. Lower write latency. Risk of data loss if cache fails.

### Redis Caching Patterns

- Use TTL on every cached key. Use `SET key value EX seconds NX` for cache-aside with expiry.
- Cache serialized JSON or MessagePack. Use hash types for structured objects.
- Implement cache stampede protection: lock on cache miss, or use probabilistic early expiration.
- Monitor hit rate (`INFO stats`). Target >90% hit rate for effective caching.

### Cache Invalidation

- TTL-based: simple, eventually consistent. Event-based: invalidate on DB write (more complex, more consistent).
- Tag-based: group related keys under a tag, invalidate all at once.

## 5. Event Sourcing and CQRS

### Event Sourcing

- Store state changes as immutable events rather than current state. Events are facts that happened: `OrderPlaced`, `ItemAdded`, `PaymentReceived`.
- Reconstruct current state by replaying events. Use snapshots to avoid replaying from the beginning for entities with many events.
- Event store must be append-only. Events are immutable: never update or delete.

### CQRS (Command Query Responsibility Segregation)

- Separate write model (commands) from read model (queries). Write model validates and persists events. Read model(s) are projections optimized for specific query patterns.
- Multiple read models can serve different views (list view, detail view, analytics).
- Read models can be rebuilt from events at any time.

### When to Use

- Audit requirements (financial, compliance). Complex domain with many state transitions. Multiple read patterns from same data. When NOT to use: simple CRUD, small-scale apps, when team lacks event sourcing experience.

## 6. Data Modeling and Normalization

### Normal Forms (Practical Application)

- **1NF**: Atomic values, no repeating groups. Every table should meet this.
- **2NF**: No partial dependencies (all non-key columns depend on the full primary key). Eliminate these in transactional systems.
- **3NF**: No transitive dependencies (non-key columns don't depend on other non-key columns). Standard for OLTP databases.
- **Denormalization**: Intentionally violate normal forms for read performance. Common in OLAP, analytics, and caching. Document the tradeoff.

### Naming Conventions

- Tables: plural nouns (users, orders). Columns: snake_case. Foreign keys: `referenced_table_id`. Indexes: `idx_table_column`. Constraints: `chk_table_rule`, `uq_table_column`.

## 7. Multi-Tenancy Isolation

### Strategies

| Strategy | Isolation | Complexity | Cost | Use Case |
|----------|-----------|-----------|------|----------|
| **Shared tables** (tenant_id column) | Low | Low | Low | SaaS with many small tenants |
| **Schema per tenant** | Medium | Medium | Medium | Compliance needs per tenant |
| **Database per tenant** | High | High | High | Enterprise, strict isolation |

### Shared Table Patterns

- Add `tenant_id` to every table. Include `tenant_id` in every query WHERE clause. Use RLS (Postgres) or application middleware to enforce. Include `tenant_id` in all indexes and unique constraints.

### Cross-Tenant Protection

- Never allow queries without tenant_id filter. Test for tenant data leakage in integration tests. Use separate connection pools or credentials per tenant for database-per-tenant.

## 8. Event-Driven Architecture

### Patterns

- **Event notification**: Publish thin events (entity changed), consumers fetch details. Low coupling.
- **Event-carried state transfer**: Publish full entity state in events. Consumers don't need to call back. Higher coupling but lower latency.
- **Event sourcing**: Events are the source of truth (see section 5).

### Guarantees

- Use transactional outbox pattern: write event to outbox table in same transaction as state change. Separate process reads outbox and publishes to broker.
- Idempotent consumers: handle duplicate events gracefully (use event ID for deduplication).

### Topic Design

- One topic per entity type (orders, payments, users). Use entity ID as partition key for ordering. Version event schemas. Use dead-letter topics for unprocessable events.

## 9. Specialized Databases

### Cassandra/CQL

- Design tables around query patterns (one table per query). Partition key determines data distribution. Clustering key determines sort order within partition.
- Avoid large partitions (>100MB). No joins, no subqueries. Use materialized views or denormalized tables for different access patterns.

### TimescaleDB

- Hypertables for time-series data. Automatic partitioning by time. Continuous aggregates for pre-computed rollups.
- Compression for old data. Retention policies for automatic data lifecycle.

## Checklist

- [ ] Indexes aligned with query patterns, verified with EXPLAIN ANALYZE
- [ ] Connection pooling configured
- [ ] RLS policies tested with SET ROLE
- [ ] Migrations are reversible, tested against production-sized data
- [ ] Caching strategy chosen with TTL and invalidation policy
- [ ] Event sourcing snapshots configured (if using event sourcing)
- [ ] Multi-tenancy isolation strategy documented and tested for leakage
- [ ] Event-driven consumers are idempotent
