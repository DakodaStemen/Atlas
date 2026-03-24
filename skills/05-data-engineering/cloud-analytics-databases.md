---
name: cloud-analytics-databases
description: Patterns for cloud analytics databases and specialized data stores including MongoDB (document modeling, aggregation, Atlas Search), Snowflake (warehouse architecture, cloning, streams), ClickHouse (columnar analytics, materialized views), and BigQuery (partitioning, clustering, BI Engine). Use when designing schemas or optimizing queries for these platforms.
domain: data-engineering
tags: [mongodb, snowflake, clickhouse, bigquery, analytics, nosql, data-warehouse, columnar]
triggers: mongodb, snowflake, clickhouse, bigquery, document database, columnar analytics, aggregation pipeline, data warehouse, BI Engine
---

# Cloud Analytics Databases

## 1. MongoDB Schema Design

### Embedding vs Referencing

- **Embed (One-to-Few)**: Data is small and always read together (user addresses). Keeps reads in a single document.
- **Reference (One-to-Many/Squillions)**: Related data is large or grows indefinitely (post comments). Prevents exceeding 16MB document limit.
- **Subset Pattern**: For high-read collections, embed only the most recent/relevant items (top 5 reviews). Move full history to separate collection.

### Aggregation Pipeline

Core stages: `$match` (filter early), `$group` (aggregate), `$project` (reshape), `$lookup` (left outer join), `$unwind` (flatten arrays), `$sort`, `$limit`.

Key rule: Place `$match` as early as possible to reduce documents flowing through the pipeline. Use `$match` before `$lookup` to minimize join input.

### Indexing

- Create indexes to support query patterns. Compound indexes: field order matters (equality → sort → range).
- Use `explain()` to verify index usage. Covered queries (all fields in index) avoid document fetches.
- TTL indexes for automatic document expiration. Unique indexes for constraint enforcement.

### Atlas Search

- Full-text search with analyzers, fuzzy matching, autocomplete. Use `$search` stage in aggregation pipeline.
- Define search indexes separately from database indexes. Supports faceting, highlighting, and scoring.

## 2. Snowflake

### Architecture

- Separation of storage and compute. Virtual warehouses scale independently. Multi-cluster warehouses for concurrency scaling.
- Micro-partitions: automatic, immutable, compressed columnar storage (50-500MB). Pruning eliminates irrelevant partitions during query planning.

### Key Features

- **Zero-copy cloning**: Instant clones of databases/schemas/tables for dev/test without copying data. `CREATE TABLE clone_t CLONE prod_t;`
- **Time travel**: Query historical data (`SELECT * FROM t AT (TIMESTAMP => '...')`). Undrop tables. Configurable retention (1-90 days).
- **Streams**: Track DML changes (inserts, updates, deletes) on tables. Enable CDC patterns. Use with tasks for automated processing.
- **Tasks**: Scheduled SQL execution. Can be chained (predecessor tasks). Use with streams for event-driven pipelines.

### Performance

- Cluster keys for large tables (>1TB) with common filter patterns. Use `RESULT_SCAN(LAST_QUERY_ID())` to avoid re-execution.
- Monitor with `QUERY_HISTORY` and `WAREHOUSE_METERING_HISTORY` views. Set resource monitors for cost control.

### Cost Control

- Right-size warehouses. Use auto-suspend (1-5 minutes). Separate warehouses for ETL vs BI. Monitor credit consumption daily.

## 3. ClickHouse

### When to Use

- Real-time analytics on billions of rows. Sub-second aggregation queries. Time-series data with high ingestion rates. OLAP workloads where PostgreSQL or MySQL are too slow.

### Table Engines

- **MergeTree**: Default for analytics. Supports primary key ordering, partitioning, TTL, sampling. Data sorted on disk by primary key for fast range scans.
- **ReplacingMergeTree**: Deduplication by primary key (eventual, on merge). Use `FINAL` keyword for guaranteed latest version.
- **AggregatingMergeTree**: Pre-aggregated storage using aggregate function states. Combine with materialized views for real-time rollups.
- **Distributed**: Queries across shards. Place on top of local MergeTree tables.

### Materialized Views

- Automatically transform data on INSERT. Act as triggers that populate target tables. Use for real-time aggregations, denormalization, and format conversion.

```sql
CREATE MATERIALIZED VIEW hourly_stats
ENGINE = AggregatingMergeTree()
ORDER BY (hour, source)
AS SELECT
    toStartOfHour(timestamp) AS hour,
    source,
    countState() AS count,
    sumState(value) AS total
FROM events
GROUP BY hour, source;
```

### Performance Tips

- Primary key = sort order on disk (not a unique constraint). Choose columns used in WHERE clauses.
- Use `LowCardinality(String)` for columns with <10K distinct values. Use `Nullable` sparingly (adds overhead).
- Batch inserts (>1000 rows per INSERT). Avoid single-row inserts. Use `Buffer` engine for high-frequency writes.

## 4. BigQuery

### Partitioned and Clustered Tables

Always partition by date/time and cluster by high-cardinality filter columns.

```sql
CREATE TABLE `project.dataset.events` (
    time TIMESTAMP,
    user_id STRING,
    event_type STRING,
    payload JSON
)
PARTITION BY DATE(time)
CLUSTER BY user_id, event_type;
```

### BI Engine

- In-memory acceleration for sub-second dashboard queries. Monitor with `INFORMATION_SCHEMA.BI_CAPACITIES` and `BI_CAPACITY_CHANGES`.
- Check acceleration status: query `INFORMATION_SCHEMA.JOBS` for `bi_engine_statistics`.

### Cost Optimization

- Use partitioning to minimize bytes scanned (partitioned queries cost less). Use clustering for additional pruning.
- `SELECT` only needed columns (columnar billing). Use `LIMIT` with caution (still scans full partition).
- Set project-level and user-level byte quotas. Monitor with `INFORMATION_SCHEMA.JOBS` for bytes_processed trends.

### Performance Monitoring

```sql
-- Top expensive queries
SELECT user_email, SUM(total_bytes_processed) as total_bytes,
       COUNT(*) as query_count
FROM `region-us`.INFORMATION_SCHEMA.JOBS
WHERE creation_time > TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 7 DAY)
GROUP BY user_email
ORDER BY total_bytes DESC
LIMIT 20;
```

### Slot Usage

- Monitor slot utilization vs reservation. Use `INFORMATION_SCHEMA.JOBS_TIMELINE` for per-second slot analysis.
- Use flex slots for burst capacity. Set slot commitments for predictable workloads.

## Comparison Matrix

| Feature | MongoDB | Snowflake | ClickHouse | BigQuery |
|---------|---------|-----------|------------|----------|
| **Model** | Document | Columnar SQL | Columnar SQL | Columnar SQL |
| **Best for** | Flexible schemas, real-time apps | Data warehouse, ELT | Real-time analytics, time-series | Serverless analytics, large scans |
| **Scaling** | Horizontal (sharding) | Elastic compute | Sharding + replication | Serverless |
| **Cost model** | Compute + storage | Credits (compute) | Self-hosted or cloud | Bytes scanned |
| **Latency** | Single-digit ms reads | Seconds | Sub-second aggregations | Seconds |
