---
name: data-pipeline-comprehensive
description: Comprehensive data pipeline patterns covering batch processing, streaming, ETL/ELT design, DAG orchestration (Airflow/Dagster/Prefect), CDC (Debezium), pipeline testing, and data observability. Use when designing, building, testing, or monitoring any data pipeline.
domain: data-engineering
tags: [pipeline, batch, streaming, etl, elt, orchestration, cdc, testing, observability, airflow, dagster, debezium]
triggers: data pipeline, batch processing, streaming pipeline, ETL, ELT, pipeline orchestration, CDC, change data capture, pipeline testing, data observability, DAG design, backpressure, windowing, debezium, pipeline monitoring
---

# Data Pipeline Comprehensive Guide

## 1. Batch Processing Patterns

### Job Design

- Each batch job should do one thing well. Accept explicit time ranges (start_date, end_date) rather than computing "yesterday" internally. Make jobs idempotent: running the same job twice for the same time range produces identical results.
- Process data in partitions (by date, region, tenant) rather than as a monolithic batch for parallelism and partial reruns.

### Scheduling and Dependencies

- Use cron expressions or orchestrator schedules (Airflow, Dagster). Account for source system latency: if upstream data lands at 02:00, schedule downstream at 03:00 with a sensor/check.
- Define explicit dependencies using DAGs. Never rely on implicit timing. Use data sensors or completion markers (_SUCCESS files, metadata table entries) to gate downstream execution.

### Checkpointing and Retry

- For long-running jobs, checkpoint at logical boundaries (per partition/chunk). On restart, resume from last successful checkpoint. Store checkpoint state durably (database, object storage), not local disk.
- Configure automatic retries with exponential backoff for transient failures. Set max retry count (2-3). Distinguish retriable from permanent errors.

### Backfill

- Design every job to accept arbitrary date ranges from day one. Process in reverse chronological order for large backfills. Limit concurrency to avoid overwhelming source systems.

### Resource Management

- Right-size compute for data volume. Set memory limits and fail explicitly rather than OOM cascading. Monitor CPU, memory, and I/O per job.

## 2. Streaming Pipeline Design

### Event Time and Ordering

- Always prefer event time over processing time. Embed event timestamps in message payloads. Use watermarks to track event-time progress.
- Order is guaranteed only within a single partition/shard. Use consistent partitioning by entity key. Never assume global ordering across partitions.

### Delivery Guarantees

- **At-most-once**: Fire and forget, for non-critical metrics.
- **At-least-once**: Retry on failure, requires idempotent consumers.
- **Exactly-once**: Transactional writes or idempotent consumers with deduplication. Requires end-to-end support (Kafka transactions + Flink checkpointing).

### Windowing Strategies

- **Tumbling**: Fixed-size, non-overlapping (every 5 minutes).
- **Hopping**: Fixed-size, overlapping (10-minute window every 5 minutes).
- **Session**: Gap-based, dynamic size per key (close after 30 minutes inactivity).
- **Global**: Custom triggers for specialized use cases.

### Late-Arriving Data

- Define an allowed lateness threshold. Side-output late events for separate handling. Update aggregations when late data arrives within threshold. Drop or log events exceeding threshold.

### Backpressure

- Monitor consumer lag as primary health metric. Implement bounded queues with explicit overflow policies. Scale consumers horizontally when lag grows. Alert on sustained lag growth, not momentary spikes.

### State Management

- Streaming stateful operations require checkpointed state. Use framework-managed state (Flink RocksDB, Kafka Streams state stores). Set TTLs on state entries. Size state carefully to avoid OOM.

### Schema in Streaming

- Use a schema registry (Confluent, Apicurio) to enforce compatibility. Consumers must handle schema evolution gracefully.

### Error Handling

- Route poison messages to dead-letter topics. Never let a single bad message block the entire partition.

## 3. ETL/ELT Pipeline Design

### ETL vs ELT Decision

- **ETL**: When target has limited compute or sensitive data must be filtered before landing.
- **ELT**: When target warehouse has strong compute (BigQuery, Snowflake, Databricks) and schema-on-read flexibility is beneficial.

### Extraction Patterns

- **Full extraction**: Small reference tables.
- **Incremental**: Watermarks (timestamp or incrementing ID) for large transactional tables.
- **CDC**: Near-real-time requirements (see section 5).
- **API-based**: Rate limiting, pagination, retry with exponential backoff.

### Transformation Strategies

- Stage raw data first (bronze), then cleaning/conforming (silver), then business aggregations (gold). Never transform in-place on the source. SQL-based in ELT; Spark/Beam/Python for complex logic in ETL.

### Loading Methods

- Prefer bulk/batch loads (COPY INTO, bq load) over row-by-row inserts. Use merge/upsert for idempotent loads. Truncate-and-reload only for small dimension tables. Append-only with deduplication for event/fact tables.

### Idempotency

- Every pipeline run with same input must produce same output. Use merge statements or delete-then-insert by partition. Tag records with batch ID or run timestamp.

### Data Contracts

- Define explicit schemas at pipeline boundaries. Validate incoming data against contracts before processing. Version contracts and handle backward compatibility.

## 4. DAG Orchestration (Airflow, Dagster, Prefect)

### DAG Design Principles

- Keep DAGs shallow and wide. Each task: atomic, idempotent, independently retriable. No task-to-task data passing through orchestrator (XCom is for metadata only). Use external storage for intermediate data.

### Airflow Patterns

- Use TaskFlow API (@task decorator). Prefer KubernetesPodOperator or DockerOperator for isolation. Set execution_timeout on every operator. Use pools for concurrency limits. Never use SubDAGs; use TaskGroups.

### Dagster Patterns

- Model as Software-Defined Assets. Use partitions for time-based processing. Leverage IO Managers for standardized data I/O. Define asset checks for inline data quality.

### Prefect Patterns

- Use flow and task decorators with automatic retry. Leverage blocks for infrastructure config. Use work pools for execution environment management.

### SLA Monitoring

- Define SLAs for pipeline completion time. Alert at 80% of SLA threshold as warning. Track historical execution times. Define SLAs with stakeholders, not engineers alone.

### Failure Handling

- Task-level retries for transient failures. On_failure_callback for contextual alerts. Circuit breakers for upstream dependencies. Separate infrastructure failures (retry) from data failures (alert/investigate).

### Scaling

- Separate orchestrator control plane from execution. Use Celery/Kubernetes/cloud-native executors. Monitor scheduler latency. Keep DAG parsing fast: avoid heavy imports at module level.

## 5. Change Data Capture (CDC)

### Approaches

| Approach | Best For | Limitations |
|----------|----------|-------------|
| **Log-based** (preferred) | Zero source impact, captures all changes including deletes | Requires DB config (WAL/binlog) |
| **Trigger-based** | When log access unavailable | Adds write overhead, trigger maintenance |
| **Timestamp-based** | Append-mostly tables, non-real-time | Misses deletes, misses intermediate changes |
| **Snapshot-based** | Small tables, initial loads | High resource cost |

### Debezium Patterns

- Deploy as Kafka Connect connectors. Use outbox pattern for microservice event publishing. Configure snapshot mode carefully: `initial` (full then streaming), `schema_only` (stream from current position), `never` (assume topics have data).

### Schema Evolution in CDC

- Source schema changes propagate through CDC. Use schema registry with backward compatibility. Consumers handle unknown fields gracefully. Test schema changes in staging.

### Ordering and Consistency

- CDC events ordered per-table, per-primary-key within single database. Cross-table consistency requires careful consumer design. Debezium provides transaction metadata for reconstructing transaction boundaries.

### Monitoring CDC

- Track replication lag. Monitor connector status with auto-restart. Alert on schema change events. Track throughput and error rates. Monitor replication slots to prevent WAL bloat (PostgreSQL).

## 6. Pipeline Testing

### Unit Testing Transformations

- Isolate transformation logic from I/O. Test pure functions with DataFrames in/out. Cover edge cases: nulls, empties, type coercions, boundary values, duplicate keys. Use small hand-crafted datasets (5-20 rows). Test in-memory.

### Integration Testing

- Test full pipeline source-to-sink in controlled environment. Use test databases, local FS, or Testcontainers. Validate row counts, schema conformance, key data values. Test happy path and failure scenarios.

### Data Contract Testing

- Contracts specify: schema, semantics, SLAs, quality rules. Consumer tests validate incoming data meets contract. Producer tests validate output conforms to advertised contract.

### Test Fixtures

- Create reusable fixtures for common scenarios. Generate synthetic data with realistic distributions (Faker). Include edge cases: Unicode, extreme values, null patterns, timezone edges. Version alongside pipeline code.

### Regression Testing

- When a bug is found, add a test case with the specific failing data. Build regression suite that grows with each incident.

### CI/CD for Data Pipelines

- Unit tests on every commit. Integration tests on pull requests. Gate deployments on test passage. Deploy to staging first, validate, then promote.

## 7. Data Observability

### Five Pillars

1. **Freshness**: Is data up to date? Track latest record timestamps vs expected update frequency.
2. **Volume**: Is expected amount present? Baselines via rolling averages, alert on >30% drop or >200% spike.
3. **Schema**: Has structure changed? Compare current vs last known after every pipeline run.
4. **Distribution**: Are values in expected ranges? Profile null rates, distinct counts, min/max, mean/stddev.
5. **Lineage**: Where did data come from, where does it go?

### Pipeline Performance Observability

- Track execution duration per task and per pipeline. Alert on duration anomalies (2x normal). Monitor resource utilization. Track queue depth and scheduling lag.

### Incident Response

- **S1**: Critical data missing, business impact. **S2**: Quality degraded, partial impact. **S3**: Anomaly detected, investigation needed.
- Establish on-call rotation. Steps: detect, assess impact, communicate, fix, validate, post-mortem. Track MTTD and MTTR.

### Tooling

- **Self-hosted**: Elementary (dbt-native), Great Expectations, Soda Core.
- **Managed**: Monte Carlo, Bigeye, Anomalo.
- Start open-source, evaluate managed when team/pipeline count grows.

### Cost of Observability

- Run expensive checks (full distribution) daily. Run cheap checks (row count, freshness) hourly or per-run. Sample large tables instead of profiling every row.

## Master Checklist

- [ ] Jobs parameterized with explicit time ranges and idempotent
- [ ] Dependencies declared explicitly in DAGs
- [ ] Checkpointing for jobs >15 minutes
- [ ] Retry with exponential backoff and max attempts
- [ ] Event time semantics with watermarks (streaming)
- [ ] Delivery guarantee chosen and documented
- [ ] Dead-letter topics for poison messages
- [ ] CDC approach chosen with documented rationale
- [ ] Schema registry with compatibility mode
- [ ] Transformation logic unit tested (isolated from I/O)
- [ ] Data contracts at producer-consumer boundaries
- [ ] Freshness, volume, schema, distribution monitoring active
- [ ] Incident severity levels and response procedures defined
- [ ] Data health dashboard built and reviewed daily
