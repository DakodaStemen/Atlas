---
name: data-architecture-patterns
description: Comprehensive data architecture patterns covering data lake design (medallion/bronze-silver-gold), warehouse optimization, dimensional modeling (star/snowflake schemas), partitioning strategies, schema evolution, and serialization formats. Use when designing storage layers, choosing file formats, or planning data modeling.
domain: data-engineering
tags: [data-lake, warehouse, dimensional-modeling, partitioning, schema-evolution, serialization, parquet, avro, medallion, star-schema]
triggers: data lake, medallion architecture, bronze silver gold, data warehouse, dimensional model, star schema, snowflake schema, partitioning, schema evolution, serialization format, parquet, avro, file format, SCD, slowly changing dimension
---

# Data Architecture Patterns

## 1. Data Lake Architecture (Medallion Pattern)

### Zone Design

- **Bronze**: Raw, immutable ingested data. Preserve original schema/format. Add ingestion metadata (source, timestamp, batch ID). Append-only writes. Retain per compliance policy.
- **Silver**: Cleaning, deduplication, type casting, joins. "Single source of truth" layer. Partition by commonly filtered columns. Use Delta Lake, Iceberg, or Hudi for ACID transactions and time travel.
- **Gold**: Pre-computed aggregations, feature stores, denormalized datasets. Optimized for specific consumers (BI, ML, API). Smaller volume, higher business value.

### File Format Selection

| Format | Use Case | Key Properties |
|--------|----------|---------------|
| **Parquet** | Analytical queries | Columnar, excellent compression, predicate pushdown |
| **Avro** | Streaming, schema evolution | Row-based, schema embedded in file |
| **ORC** | Hive-heavy ecosystems | Similar to Parquet, better Hive integration |
| **JSON/CSV** | Bronze landing only | Convert immediately after ingestion |

### Partitioning

- Partition by time (year/month/day) as default for event data. Add secondary partitions only when queries consistently filter on them. Avoid over-partitioning (too many small files). Target 128MB-1GB per partition file.

### Compaction

- Schedule regular compaction jobs. Use table formats (Delta, Iceberg) with native compaction. Monitor file counts per partition.

### Catalog Integration

- Register all silver and gold tables in metadata catalog (Hive Metastore, AWS Glue, Unity Catalog). Include schema, partitioning, ownership, data classification. Catalogs are the entry point for governance.

### Access Control

- Least-privilege at zone level. Bronze: data engineers. Silver: data engineers + analysts. Gold: broad read access. Column-level masking for sensitive fields.

## 2. Data Warehouse Optimization

### Query Performance

- **Indexing**: Create indexes aligned with WHERE, JOIN, and ORDER BY patterns. Use composite indexes for multi-column filters. Partial indexes for filtered subsets. Monitor unused indexes and remove them.
- **Materialized views**: Pre-compute expensive aggregations. Refresh on schedule or incrementally. Use for dashboards with known query patterns. Track staleness.
- **Clustering**: In columnar warehouses (BigQuery, Snowflake, Redshift), cluster tables by high-cardinality filter columns. Recluster periodically.

### Storage Optimization

- Use columnar storage for analytical workloads. Compress aggressively (Snowflake automatic, Redshift ENCODE). Archive cold data to cheaper storage tiers. Set retention policies per table.

### Workload Management

- Separate compute for ETL and BI queries (Snowflake warehouses, Redshift WLM queues). Set query timeout limits. Monitor and kill long-running queries. Use result caching where available.

### Cost Control

- Monitor scan volumes per query. Use partitioning/clustering to minimize scanned data. Set cost alerts per user/team. Review and optimize expensive queries weekly.

## 3. Dimensional Modeling

### Star Schema

- Central **fact table** containing business measurements (revenue, quantity, duration) with foreign keys to dimension tables. Dimension tables are denormalized, contain descriptive attributes, and enable slicing/dicing. Facts are numeric, additive, and timestamped.

### Snowflake Schema

- Normalized dimensions (e.g., product → category → department). Saves storage but adds join complexity. Use only when dimension tables are very large and updates are frequent.

### Fact Table Types

- **Transaction facts**: One row per event (order, click, payment). Most common.
- **Periodic snapshots**: One row per entity per period (daily account balance, monthly inventory).
- **Accumulating snapshots**: One row per process instance with milestone timestamps (order placed, shipped, delivered).

### Slowly Changing Dimensions (SCD)

- **Type 1**: Overwrite. No history preserved. Simple, used when history doesn't matter.
- **Type 2**: New row with effective dates (valid_from, valid_to). Full history preserved. Most common for auditable dimensions.
- **Type 3**: Additional column for previous value. Limited history (one prior value only).
- **Type 6 (hybrid)**: Combines Type 1 + 2 + 3 for both current and historical access.

### Conformed Dimensions

- Shared dimensions used identically across multiple fact tables. Date, customer, product dimensions should be conformed. Governance required to prevent drift.

### Grain

- Define the grain (level of detail) of every fact table before designing it. One row = one transaction? One day? One session? Grain drives everything: which dimensions attach, which facts are additive, what queries are possible.

## 4. Partitioning Strategies

### Horizontal Partitioning (Sharding)

- **Range**: Partition by value ranges (dates, IDs). Natural for time-series. Risk of hot partitions if distribution is skewed.
- **Hash**: Distribute by hash of partition key. Even distribution, but range scans require hitting all partitions.
- **List**: Partition by explicit value lists (regions, categories). Good when cardinality is low and stable.
- **Composite**: Combine range + hash (partition by date, then hash within each date).

### When to Partition

- Tables >1M rows or >1GB. Queries consistently filter on a specific column. Maintenance operations (archival, purging) needed at partition level. Parallel processing benefits from partition-level isolation.

### Anti-Patterns

- Over-partitioning: Too many small partitions degrade query planning and metadata management.
- Wrong partition key: Choosing a key that doesn't align with query patterns forces full-partition scans.
- Skewed partitions: One partition much larger than others creates hot spots.

## 5. Schema Evolution Strategies

### Compatibility Modes (Schema Registry)

- **Backward compatible**: New schema can read old data. Safe for consumers: add optional fields, widen types.
- **Forward compatible**: Old schema can read new data. Safe for producers: remove optional fields, narrow types.
- **Full compatible**: Both backward and forward. Most restrictive but safest.

### Safe Changes

- Adding optional columns (with defaults). Widening types (int32 → int64). Adding new enum values at the end.

### Breaking Changes (Require Coordination)

- Removing columns. Renaming columns. Narrowing types. Changing column semantics.

### Migration Patterns

- **Dual-write**: Write to both old and new schema during transition. Validate consistency. Switch readers to new schema. Stop writing old schema.
- **Shadow tables**: Create new table with new schema, backfill from old, swap when validated.
- **Versioned schemas**: Include schema version in data. Consumers handle multiple versions.

### Tooling

- Schema registries (Confluent, Apicurio, AWS Glue) for streaming schemas. Database migration tools (Flyway, Alembic, dbmate) for relational schemas. Always test migrations against production-sized data in staging.

## 6. Data Serialization Formats

### Binary Formats

| Format | Schema | Compression | Use Case |
|--------|--------|-------------|----------|
| **Protocol Buffers** | Required (.proto) | Excellent | gRPC services, inter-service messaging |
| **Avro** | Embedded | Good | Kafka, data lake ingestion, schema evolution |
| **MessagePack** | Schema-less | Good | Drop-in JSON replacement needing speed |
| **CBOR** | Schema-less | Good | IoT, constrained environments |
| **FlatBuffers** | Required | Zero-copy | Game engines, performance-critical paths |

### Text Formats

| Format | Use Case |
|--------|----------|
| **JSON** | APIs, config, human-readable interchange |
| **YAML** | Configuration files |
| **CSV/TSV** | Tabular data exchange, spreadsheet import/export |
| **XML** | Legacy systems, SOAP, enterprise integration |

### Selection Criteria

- **Schema evolution needed?** → Avro or Protobuf.
- **Human readability needed?** → JSON or YAML.
- **Maximum performance?** → FlatBuffers (zero-copy) or Protobuf.
- **Streaming with Kafka?** → Avro with schema registry.
- **Columnar analytics?** → Parquet (file) or Arrow (in-memory).

## Master Checklist

- [ ] Zone boundaries (bronze/silver/gold) defined with clear contracts
- [ ] File format chosen per zone with rationale
- [ ] Partitioning strategy aligned with query patterns
- [ ] Table format (Delta/Iceberg/Hudi) evaluated for ACID needs
- [ ] Fact table grain defined before design
- [ ] SCD type chosen for each dimension with documented rationale
- [ ] Conformed dimensions identified and governed
- [ ] Schema compatibility mode set in registry
- [ ] Migration tested against production-sized data in staging
- [ ] Serialization format chosen based on use case requirements
