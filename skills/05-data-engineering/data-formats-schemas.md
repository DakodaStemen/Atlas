---
name: data-formats-schemas
description: Data formats and schemas covering JSON Schema (validation, complex types, conditional schemas, draft-07/2020-12), data lakehouse formats (Delta Lake, Apache Iceberg, Apache Hudi — features, operations, maintenance), and related serialization patterns. Use when defining data contracts, validating payloads, or choosing table formats.
domain: data-engineering
tags: [json-schema, delta-lake, iceberg, hudi, lakehouse, data-formats, validation, table-format]
triggers: json schema, delta lake, apache iceberg, apache hudi, lakehouse, data format, schema validation, table format
---

﻿---
name: JSON Schemas
description: # JSON Schemas
 
 This document defines the JSON schemas used by skill-creator.

## evals.json

Defines the evals for a skill. Located at `evals/evals.json` within the skill directory.

```json
{
  "skill_name": "example-skill",
  "evals": [
    {
      "id": 1,
      "prompt": "User's example prompt",
      "expected_output": "Description of expected result",
      "files": ["evals/files/sample1.pdf"],
      "expectations": [
        "The output includes X",
        "The skill used script Y"
      ]
    }
  ]
}
```

### Fields

- `skill_name`: Name matching the skill's frontmatter
- `evals[].id`: Unique integer identifier
- `evals[].prompt`: The task to execute
- `evals[].expected_output`: Human-readable description of success
- `evals[].files`: Optional list of input file paths (relative to skill root)
- `evals[].expectations`: List of verifiable statements


## grading.json

Output from the grader agent. Located at `<run-dir>/grading.json`.

```json
{
  "expectations": [
    {
      "text": "The output includes the name 'John Smith'",
      "passed": true,
      "evidence": "Found in transcript Step 3: 'Extracted names: John Smith, Sarah Johnson'"
    },
    {
      "text": "The spreadsheet has a SUM formula in cell B10",
      "passed": false,
      "evidence": "No spreadsheet was created. The output was a text file."
    }
  ],
  "summary": {
    "passed": 2,
    "failed": 1,
    "total": 3,
    "pass_rate": 0.67
  },
  "execution_metrics": {
    "tool_calls": {
      "Read": 5,
      "Write": 2,
      "Bash": 8
    },
    "total_tool_calls": 15,
    "total_steps": 6,
    "errors_encountered": 0,
    "output_chars": 12450,
    "transcript_chars": 3200
  },
  "timing": {
    "executor_duration_seconds": 165.0,
    "grader_duration_seconds": 26.0,
    "total_duration_seconds": 191.0
  },
  "claims": [
    {
      "claim": "The form has 12 fillable fields",
      "type": "factual",
      "verified": true,
      "evidence": "Counted 12 fields in field_info.json"
    }
  ],
  "user_notes_summary": {
    "uncertainties": ["Used 2023 data, may be stale"],
    "needs_review": [],
    "workarounds": ["Fell back to text overlay for non-fillable fields"]
  },
  "eval_feedback": {
    "suggestions": [
      {
        "assertion": "The output includes the name 'John Smith'",
        "reason": "A hallucinated document that mentions the name would also pass"
      }
    ],
    "overall": "Assertions check presence but not correctness."
  }
}
```

### Fields (grading.json)

- `expectations[]`: Graded expectations with evidence
- `summary`: Aggregate pass/fail counts
- `execution_metrics`: Tool usage and output size (from executor's metrics.json)
- `timing`: Wall clock timing (from timing.json)
- `claims`: Extracted and verified claims from the output
- `user_notes_summary`: Issues flagged by the executor
- `eval_feedback`: (optional) Improvement suggestions for the evals, only present when the grader identifies issues worth raising


## timing.json

Wall clock timing for a run. Located at `<run-dir>/timing.json`.

**How to capture:** When a subagent task completes, the task notification includes `total_tokens` and `duration_ms`. Save these immediately — they are not persisted anywhere else and cannot be recovered after the fact.

```json
{
  "total_tokens": 84852,
  "duration_ms": 23332,
  "total_duration_seconds": 23.3,
  "executor_start": "2026-01-15T10:30:00Z",
  "executor_end": "2026-01-15T10:32:45Z",
  "executor_duration_seconds": 165.0,
  "grader_start": "2026-01-15T10:32:46Z",
  "grader_end": "2026-01-15T10:33:12Z",
  "grader_duration_seconds": 26.0
}
```


## comparison.json

Output from blind comparator. Located at `<grading-dir>/comparison-N.json`.

```json
{
  "winner": "A",
  "reasoning": "Output A provides a complete solution with proper formatting and all required fields. Output B is missing the date field and has formatting inconsistencies.",
  "rubric": {
    "A": {
      "content": {
        "correctness": 5,
        "completeness": 5,
        "accuracy": 4
      },
      "structure": {
        "organization": 4,
        "formatting": 5,
        "usability": 4
      },
      "content_score": 4.7,
      "structure_score": 4.3,
      "overall_score": 9.0
    },
    "B": {
      "content": {
        "correctness": 3,
        "completeness": 2,
        "accuracy": 3
      },
      "structure": {
        "organization": 3,
        "formatting": 2,
        "usability": 3
      },
      "content_score": 2.7,
      "structure_score": 2.7,
      "overall_score": 5.4
    }
  },
  "output_quality": {
    "A": {
      "score": 9,
      "strengths": ["Complete solution", "Well-formatted", "All fields present"],
      "weaknesses": ["Minor style inconsistency in header"]
    },
    "B": {
      "score": 5,
      "strengths": ["Readable output", "Correct basic structure"],
      "weaknesses": ["Missing date field", "Formatting inconsistencies", "Partial data extraction"]
    }
  },
  "expectation_results": {
    "A": {
      "passed": 4,
      "total": 5,
      "pass_rate": 0.80,
      "details": [
        {"text": "Output includes name", "passed": true}
      ]
    },
    "B": {
      "passed": 3,
      "total": 5,
      "pass_rate": 0.60,
      "details": [
        {"text": "Output includes name", "passed": true}
      ]
    }
  }
}
```


---


# Data Lakehouse Table Formats

Apache Iceberg, Delta Lake, and Apache Hudi are the three dominant open table formats that bring ACID semantics, schema evolution, and time travel to object storage (S3, GCS, ADLS). They are not storage formats — they sit as a metadata and transaction layer on top of Parquet (and occasionally Avro/ORC) files.


## Open Table Format Concepts

### Metadata Layer

All three formats separate the **metadata layer** from the **data layer**. Data files are immutable Parquet (or Avro). The metadata layer tracks which files constitute the current table version and provides the transactional contract.

```text
Table
├── metadata/          ← manifest lists, snapshot pointers, schema, partition specs
└── data/              ← immutable Parquet files (never overwritten)
```

### Manifest Files

Iceberg uses **manifest lists** → **manifest files** → data file paths. Each manifest file records the data files it covers along with column-level statistics (min/max, null counts) enabling aggressive file pruning at the manifest scan stage, before any file is opened.

Delta Lake uses a **transaction log** (the `_delta_log/` directory) of JSON commit files plus periodic Parquet **checkpoint files** (every 10 commits by default). The log records add/remove actions for data files.

Hudi uses a **timeline** (`.hoodie/` directory) of instants (commit, deltacommit, compaction, rollback) stored as small files. The timeline is the source of truth for what is committed.

### Snapshot Isolation

All three provide snapshot isolation: a reader always sees a consistent point-in-time view regardless of concurrent writers. Writers use **Optimistic Concurrency Control (OCC)** — they read the current snapshot, compute their changes, then attempt an atomic commit. If another writer committed first, the loser retries or fails.

Hudi additionally supports **Non-blocking Concurrency Control (NBCC)** introduced in 2024, where competing writers do not fail or retry — they are resolved at merge time.

### ACID on Object Storage

Object stores (S3, GCS) are not atomic at the directory level. These formats fake atomic commits by:

1. Writing new data files to a staging location.
2. Writing a new metadata file (Iceberg) or log entry (Delta/Hudi) that atomically makes those files visible.
3. Using catalog-level CAS (Iceberg REST catalog) or DynamoDB locks (Delta multi-cluster) to serialize concurrent commits.

Old data files are not deleted on commit — they become unreachable and are removed by maintenance jobs (`VACUUM`, `expire_snapshots`, Hudi `clean`).


## Iceberg: Time Travel

Iceberg snapshots are immutable. Every commit produces a new snapshot. Querying a historical snapshot is first-class.

```sql
-- By snapshot ID
SELECT * FROM catalog.db.events VERSION AS OF 8075905685798181680;

-- By timestamp
SELECT * FROM catalog.db.events
TIMESTAMP AS OF '2024-06-15 12:00:00';

-- Spark DataFrame API
spark.read
  .option("as-of-timestamp", "1718445600000")   -- epoch millis
  .table("catalog.db.events")

spark.read
  .option("snapshot-id", "8075905685798181680")
  .table("catalog.db.events")
```

### Rollback to a snapshot

```sql
-- Spark procedure
CALL catalog.system.rollback_to_snapshot('db.events', 8075905685798181680);

-- Rollback to timestamp
CALL catalog.system.rollback_to_timestamp('db.events', TIMESTAMP '2024-06-01 00:00:00');
```

#### Expire old snapshots (reclaim storage)

```sql
CALL catalog.system.expire_snapshots(
  table => 'db.events',
  older_than => TIMESTAMP '2024-05-01 00:00:00',
  retain_last => 5
);
```

`expire_snapshots` removes snapshot metadata and schedules orphan data files for deletion. Run before or alongside `remove_orphan_files` to reclaim actual storage.


## Iceberg: Engine Support

| Engine | Read | Write | Notes |
| --- | --- | --- | --- |
| Apache Spark | Yes | Yes | Native via `iceberg-spark-runtime` JAR |
| Apache Flink | Yes | Yes | Full DML including CDC upserts |
| Trino | Yes | Yes | Best non-Spark read/write support |
| Dremio | Yes | Yes | Arctic catalog (Nessie) integration |
| AWS Athena | Yes | Yes (v3) | Athena v3 engine; Glue catalog required |
| Snowflake | Yes | No | External Iceberg tables; read-only |
| StarRocks | Yes | Yes | Growing support |
| Hive | Yes | Limited | Read works; write via HiveIcebergStorageHandler |


## Delta Lake: DML

### MERGE INTO (Upsert)

```sql
MERGE INTO target_table AS t
USING source_table AS s
ON t.id = s.id
WHEN MATCHED AND s.op = 'delete' THEN DELETE
WHEN MATCHED THEN UPDATE SET
  t.value   = s.value,
  t.updated = s.updated
WHEN NOT MATCHED THEN INSERT (id, value, updated)
  VALUES (s.id, s.value, s.updated)
WHEN NOT MATCHED BY SOURCE AND t.last_seen < current_date() - INTERVAL 90 DAYS
  THEN UPDATE SET t.status = 'stale';   -- requires Delta 2.4+
```

The `WHEN NOT MATCHED BY SOURCE` clause enables full outer merge semantics — handling target rows with no match in the source.

### UPDATE and DELETE

```sql
UPDATE orders SET status = 'cancelled' WHERE order_date < '2023-01-01' AND status = 'pending';

DELETE FROM events WHERE user_id IN (SELECT user_id FROM gdpr_deletions);
```

Logical deletes write a `remove` action in the Delta Log. Physical deletion requires `VACUUM`:

```sql
-- Default 7-day retention; reduce carefully
VACUUM my_table RETAIN 168 HOURS;
```

### COPY INTO

```sql
COPY INTO my_table
FROM 's3://my-bucket/incoming/'
FILEFORMAT = PARQUET
COPY_OPTIONS ('mergeSchema' = 'true');
```

`COPY INTO` is idempotent — it tracks which files have been loaded and skips re-ingestion, making it safe to schedule repeatedly.


## Apache Hudi

Hudi (Hadoop Upserts Deletes and Incrementals) was created at Uber in 2016 to handle high-frequency CDC at scale. It is now a top-level Apache project.

### Copy-on-Write vs Merge-on-Read

#### Copy-on-Write (CoW)

- Every update rewrites the entire Parquet file containing the affected row.
- Reads are always clean Parquet — no merge overhead at read time.
- Best for read-heavy, low-update-frequency tables.
- Write amplification is high for small, frequent updates.

#### Merge-on-Read (MoR)

- Updates are appended to Avro **delta log files** alongside base Parquet files.
- Reads merge base + delta files on the fly (or use a read-optimized view with only base files).
- Compaction consolidates delta logs into new base Parquet files asynchronously.
- Best for write-heavy, high-upsert workloads (CDC ingestion, event streams).

```text
MoR table structure:
data/
├── base-file-001.parquet        ← clean base file
├── .base-file-001.log.1_1-...  ← delta log (updates/deletes)
├── .base-file-001.log.2_1-...  ← more delta entries
└── base-file-002.parquet        ← another base file (no pending deltas)
```

### Upsert with Record Keys

Hudi enforces a **primary key** (`hoodie.datasource.write.recordkey.field`) and a **precombine field** (`hoodie.datasource.write.precombine.field`) to resolve duplicates within a batch:

```python
hudi_options = {
    "hoodie.table.name":                     "orders",
    "hoodie.datasource.write.recordkey.field":  "order_id",
    "hoodie.datasource.write.precombine.field": "updated_at",
    "hoodie.datasource.write.partitionpath.field": "order_date",
    "hoodie.datasource.write.operation":     "upsert",
    "hoodie.upsert.shuffle.parallelism":     200,
}

df.write.format("hudi").options(**hudi_options).mode("append").save("s3://my-bucket/orders/")
```

Hudi's indexing layer locates which existing file group contains each record key before writing — this is what makes upserts efficient. Index types: Bloom (default), HBase (for global dedup across partitions), Bucket (deterministic, no index lookup), Record-Level Index (LSM-based, fastest for large tables).

### Hudi Timeline

The Hudi timeline (`.hoodie/` directory) records every table operation as an **instant** with three states: `requested` → `inflight` → `completed`.

```text
.hoodie/
├── 20240615120000000.commit           ← completed upsert
├── 20240615120500000.deltacommit      ← MoR delta write
├── 20240615121000000.compaction.requested
├── 20240615121000000.compaction.inflight
├── 20240615121000000.commit           ← compaction completed
└── hoodie.properties                  ← table metadata
```

Timeline instants: `commit` (CoW write), `deltacommit` (MoR write), `compaction`, `clean`, `rollback`, `savepoint`.

**Incremental queries** use the timeline to read only rows changed after a given instant:

```python
# Read all changes since a specific commit time
incremental_df = (
    spark.read.format("hudi")
    .option("hoodie.datasource.query.type", "incremental")
    .option("hoodie.datasource.read.begin.instanttime", "20240615120000000")
    .load("s3://my-bucket/orders/")
)
```


## Comparison Matrix

| Feature | Apache Iceberg | Delta Lake | Apache Hudi |
| --- | --- | --- | --- |
| **ACID transactions** | Yes (OCC) | Yes (OCC) | Yes (OCC + NBCC) |
| **Row-level deletes** | Spec v2 (delete files) | Deletion Vectors | Native (CoW/MoR) |
| **Upsert performance** | Trailing (no native upsert path) | Good (MERGE INTO) | Best (native, indexed) |
| **Streaming source/sink** | Flink native; Spark batch | First-class Spark/Delta | DeltaStreamer / Hudi Streamer |
| **CDC / change capture** | Append-only incremental reads | Change Data Feed (v2.0+) | Full CDC (before/after images) |
| **Schema evolution** | Full (field IDs, no rewrite) | Good (additive easy; breaking harder) | Good |
| **Time travel** | Snapshot-based, full SQL | Version/timestamp, full SQL | Savepoints + timeline |
| **Partition evolution** | Yes (non-destructive) | Liquid Clustering (3.1+) | Partition path changes require migration |
| **Multi-engine write** | Best (spec-driven) | Spark-primary | Spark/Flink primary |
| **Catalog requirement** | Required | Optional (directory-based) | Optional (directory-based) |
| **Multi-writer safety** | OCC + catalog CAS | OCC (DynamoDB for multi-cluster) | OCC + NBCC |
| **Small files handling** | Manual (`rewrite_data_files`) | OPTIMIZE (manual or scheduled) | Automatic file sizing |
| **Community / governance** | Apache (Netflix, Apple, Dremio) | Linux Foundation (Databricks) | Apache (Uber, Onehouse) |
| **Compaction** | `rewrite_data_files` procedure | OPTIMIZE command | Async or sync compaction job |


## References

- [Apache Iceberg Documentation](https://iceberg.apache.org/docs/latest/)
- [Apache Iceberg Table Spec](https://iceberg.apache.org/spec/)
- [Delta Lake Documentation](https://docs.delta.io/latest/)
- [Delta Lake Protocol Spec](https://github.com/delta-io/delta/blob/master/PROTOCOL.md)
- [Apache Hudi Documentation](https://hudi.apache.org/docs/overview/)
- [Onehouse: Hudi vs Delta vs Iceberg Feature Comparison (2024)](https://www.onehouse.ai/blog/apache-hudi-vs-delta-lake-vs-apache-iceberg-lakehouse-feature-comparison)
- [Hudi Blog: Iceberg vs Delta Lake vs Hudi Architectures (Oct 2024)](https://hudi.apache.org/blog/2024/10/07/iceberg-vs-delta-lake-vs-hudi-a-comparative-look-at-lakehouse-architectures/)
- [Delta Lake MERGE INTO Reference](https://docs.delta.io/latest/delta-update.html)
- [Delta Lake Structured Streaming](https://docs.delta.io/latest/delta-streaming.html)
