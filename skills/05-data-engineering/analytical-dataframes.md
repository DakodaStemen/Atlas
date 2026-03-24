---
name: analytical-dataframes
description: Analytical DataFrame libraries covering Polars/Pandas (data manipulation, lazy evaluation, groupby, joins, performance optimization) and DuckDB (embedded OLAP, SQL on files, Parquet/CSV, window functions, extensions). Use when doing data analysis, transformation, or embedded analytics.
domain: data-engineering
tags: [polars, pandas, duckdb, dataframes, analytics, embedded-analytics, sql-on-files, data-analysis]
triggers: polars, pandas, duckdb, dataframe, embedded analytics, sql on parquet, analytical query, data analysis
---


# Polars and pandas DataFrame Patterns

## Decision Matrix

Pick based on data size, operation type, and downstream dependencies — not hype.

| Factor | Use pandas | Use Polars |
| --- | --- | --- |
| Dataset size | < 1M rows | > 1M rows, or near-RAM limits |
| Operation type | Ad-hoc analysis, one-offs | Aggregation, filter-heavy, ETL |
| Downstream deps | Matplotlib, Seaborn, scikit-learn | Parquet pipelines, Arrow ecosystem |
| Concurrency | Single-threaded is fine | Need parallelism on a single node |
| Out-of-core | Chunking is acceptable | Streaming with LazyFrame |
| Team familiarity | Existing pandas codebase | Greenfield or performance-critical service |

Hybrid is valid and common: Polars handles the heavy lifting (filter, aggregate, join), then `.to_pandas()` for the final hand-off to visualization or ML libraries.


## Polars: Lazy vs. Eager API

Default to lazy. Eager exists for exploration and small data only.

```python
import polars as pl

# Eager — reads and computes immediately, no optimization
df = pl.read_csv("data.csv")
result = df.filter(pl.col("amount") > 100).group_by("category").agg(pl.col("amount").sum())

# Lazy — builds a query plan, nothing executes until .collect()
result = (
    pl.scan_csv("data.csv")          # scan_* functions: lazy equivalents of read_*
    .filter(pl.col("amount") > 100)  # predicate pushdown: filter applied at scan time
    .select(["category", "amount"])  # projection pushdown: only these columns loaded
    .group_by("category")
    .agg(pl.col("amount").sum())
    .collect()                        # execution happens here
)
```

`scan_*` variants: `scan_csv`, `scan_parquet`, `scan_ndjson`, `scan_ipc`. When the source is already in memory, call `.lazy()` to enter the lazy API.

### What the query optimizer does automatically

- **Predicate pushdown**: Filters are pushed to the earliest possible point in the plan, ideally into the file scan itself, so rows that fail the filter are never read into RAM.
- **Projection pushdown**: Columns not referenced downstream are never loaded.
- **Common subexpression elimination**: Shared computation is calculated once.
- **Parallel execution**: Independent branches of the plan run on separate threads.

### Inspect the plan

```python
lf = pl.scan_csv("data.csv").filter(pl.col("x") > 5).group_by("y").agg(pl.sum("x"))

print(lf.explain())           # optimized plan (what actually runs)
print(lf.explain(optimized=False))  # naive plan (what you wrote)
```

### Streaming for data that exceeds RAM

```python
result = (
    pl.scan_parquet("huge_file.parquet")
    .filter(pl.col("region") == "EU")
    .group_by("product_id")
    .agg(pl.col("revenue").sum())
    .collect(streaming=True)   # processes in chunks, constant memory footprint
)
```

Streaming works with most aggregation operations. It does not support all expression types — check `lf.explain(streaming=True)` to verify a plan is actually streamed.


## GroupBy and Aggregation

### Polars

```python
# Multiple aggregations in one pass (parallel within the group)
result = (
    df.group_by("region", "category")
    .agg(
        pl.col("revenue").sum().alias("total_revenue"),
        pl.col("revenue").mean().alias("avg_revenue"),
        pl.col("revenue").max().alias("max_revenue"),
        pl.col("order_id").n_unique().alias("unique_orders"),
    )
    .sort("total_revenue", descending=True)
)

# Rolling aggregation over a sorted key
df.sort("date").group_by_rolling("date", period="7d").agg(
    pl.col("sales").sum().alias("rolling_7d")
)
```

### Window functions (group transform without collapsing rows)

pandas uses `.transform()` — Polars uses `.over()`:

```python
# pandas
df["rank_in_group"] = df.groupby("category")["revenue"].rank(ascending=False)

# Polars
df.with_columns(
    pl.col("revenue").rank(descending=True).over("category").alias("rank_in_group")
)
```

`.over()` computes per group but returns a column aligned to the original DataFrame's length — no merge step required.

### pandas groupby

```python
result = (
    df.groupby(["region", "category"], as_index=False)
    .agg(
        total_revenue=("revenue", "sum"),
        avg_revenue=("revenue", "mean"),
        unique_orders=("order_id", "nunique"),
    )
)
```


## Null Handling

This is one of the sharpest behavioral differences.

### pandas: NaN vs. None, dtype-dependent

- `float64` columns use `NaN` (a floating-point sentinel).
- Integer columns with nulls are silently upcast to `float64` in pandas < 1.0; pandas 1.0+ has nullable integer types (`Int64`) but they are opt-in.
- `object` columns may hold `None`, `NaN`, or actual values — detection requires `.isna()` which handles both.

```python
# pandas null ops
df["col"].isna()
df["col"].fillna(0)
df["col"].dropna()
df.dropna(subset=["col_a", "col_b"])
```

### Polars: null is always null

Every dtype has a distinct null representation — no NaN/None ambiguity, no integer-to-float coercion:

```python
# Polars null ops
df.filter(pl.col("col").is_null())
df.filter(pl.col("col").is_not_null())
df.with_columns(pl.col("col").fill_null(0))
df.with_columns(pl.col("col").fill_null(strategy="forward"))  # ffill
df.drop_nulls(subset=["col_a", "col_b"])

# Check null count per column
df.null_count()
```

To explicitly handle NaN (which can appear in float columns imported from NumPy/pandas):

```python
df.with_columns(pl.col("x").fill_nan(None))  # convert NaN → null first, then fill_null
```


## Migrating from pandas to Polars

### Common pattern translations

| pandas | Polars |
| --- | --- |
| `df["col"]` | `df["col"]` (returns Series) or `df.select("col")` |
| `df[df["x"] > 5]` | `df.filter(pl.col("x") > 5)` |
| `df.assign(c=df.a + df.b)` | `df.with_columns((pl.col("a") + pl.col("b")).alias("c"))` |
| `df.rename({"old": "new"})` | `df.rename({"old": "new"})` |
| `df.drop("col", axis=1)` | `df.drop("col")` |
| `df.sort_values("col", ascending=False)` | `df.sort("col", descending=True)` |
| `df.head(10)` | `df.head(10)` |
| `df.shape` | `df.shape` |
| `df.dtypes` | `df.schema` |
| `df.groupby("c")["x"].transform(len)` | `df.with_columns(pl.col("x").count().over("c"))` |
| `df.pipe(f1).pipe(f2).pipe(f3)` | `df.with_columns(f1_expr, f2_expr, f3_expr)` |

### Pipe anti-pattern (critical)

```python
# SLOW — each pipe call is a separate sequential execution context
df.pipe(add_feature_a).pipe(add_feature_b).pipe(add_feature_c)

# FAST — one context, parallel execution
df.with_columns(
    feature_a_expr,
    feature_b_expr,
    feature_c_expr,
)

# Pattern: return an expression from helper functions, not a DataFrame
def compute_margin(cost_col: str, price_col: str) -> pl.Expr:
    return ((pl.col(price_col) - pl.col(cost_col)) / pl.col(price_col)).alias("margin")

df.with_columns(compute_margin("cost", "price"))
```

### No index in Polars

Polars has no row index. Remove all `.loc`, `.iloc`, `.reset_index()`, `.set_index()` references. Row position is accessed via `.row(i)` or slicing `df[i]`. There is no `SettingWithCopyWarning` because Polars enforces immutability — operations always return a new DataFrame.


## Polars Streaming: Large Dataset Pattern

```python
# Process a directory of Parquet files with constant memory
(
    pl.scan_parquet("data/events_*.parquet")
    .filter(pl.col("event_type").is_in(["purchase", "refund"]))
    .with_columns(
        (pl.col("amount") * pl.col("fx_rate")).alias("amount_usd")
    )
    .group_by("user_id", "event_type")
    .agg(
        pl.col("amount_usd").sum().alias("total_usd"),
        pl.col("event_id").count().alias("event_count"),
    )
    .sort("total_usd", descending=True)
    .collect(streaming=True)
)
```

For truly huge datasets (hundreds of GB), the streaming engine processes the plan in batches. It is equivalent to writing your own chunking loop, but the query optimizer handles ordering, early filtering, and column pruning automatically.


---


# DuckDB Analytics

## What DuckDB Is

DuckDB is an in-process OLAP SQL database — no server, no daemon, no network round-trip. It embeds directly into your Python process (or any other host), shares the same memory address space, and applies a columnar-vectorized execution engine to analytical queries. Think SQLite but designed from first principles for read-heavy, aggregation-heavy workloads rather than transactional ones.

It fills the gap between pandas (single-threaded, row-oriented) and a full data warehouse (Snowflake, BigQuery) by offering near-warehouse SQL power on a laptop with zero infrastructure.

### Core architecture traits

- Columnar storage — reads only the columns a query needs
- Vectorized batch execution — processes 1,024–2,048 values per CPU operation, exploiting SIMD
- Morsel-driven parallelism — operators are parallelism-aware; all CPU cores used by default
- Out-of-core processing — spills to disk transparently when data exceeds RAM; data sources (Parquet, CSV) are never fully materialized


## Installation

```bash
pip install duckdb
```

DuckDB has zero mandatory external dependencies. Extensions (httpfs, json, spatial, aws) install on first use or explicitly:

```python
import duckdb
con = duckdb.connect()
con.install_extension("httpfs")
con.load_extension("httpfs")
```


## Reading Files Directly

DuckDB reads files as if they were tables — no import step required.

### Parquet

```sql
-- Single file
SELECT * FROM 'data/events.parquet';

-- Multiple explicit files
SELECT * FROM read_parquet(['jan.parquet', 'feb.parquet', 'mar.parquet']);

-- Glob — reads all matching files as one logical table
SELECT event_type, COUNT(*) FROM 'data/events/*.parquet' GROUP BY 1;

-- Recursive glob
SELECT * FROM 'warehouse/**/*.parquet';

-- Schema unification when files have different columns
SELECT * FROM read_parquet('data/*.parquet', union_by_name=true);
```

### CSV

```sql
SELECT * FROM 'sales.csv';

-- DuckDB sniffs schema automatically; override when needed
SELECT * FROM read_csv('sales.csv',
    delim=',',
    header=true,
    columns={'id': 'INTEGER', 'amount': 'DOUBLE', 'ts': 'TIMESTAMP'});
```

### JSON

```sql
SELECT * FROM 'logs.json';
SELECT * FROM read_json_auto('logs/*.json');

-- Unnest nested arrays
SELECT unnest(items) AS item FROM read_json_auto('orders.json');

-- Flatten nested objects for analytics — flattening accelerates aggregation queries
SELECT
    json_extract_string(payload, '$.user_id') AS user_id,
    json_extract_string(payload, '$.event')   AS event
FROM read_json_auto('events.json');
```

### Remote files (S3, GCS, HTTPS)

```sql
-- Requires httpfs extension
INSTALL httpfs; LOAD httpfs;

SET s3_region     = 'us-east-1';
SET s3_access_key_id     = 'AKIA...';
SET s3_secret_access_key = 'secret';

SELECT * FROM 's3://my-bucket/data/*.parquet';
SELECT * FROM 'https://example.com/data.csv';
```


## Parquet Columnar Optimization

Parquet is DuckDB's native home. CSV is up to 600× slower than Parquet for analytical reads. Always convert if you control the pipeline.

### Row group sizing

- Default: 122,880 rows per group
- Minimum: 2,048 (DuckDB's vector size)
- Larger row groups → better compression, higher memory during write
- Smaller row groups → finer-grained predicate pushdown, better parallel reads
- Rule of thumb: row groups per file should be at least as large as the number of CPU threads that will scan the file; additional groups benefit selective queries

**Predicate pushdown:** DuckDB reads Parquet row group statistics (min/max per column) and skips entire row groups that cannot satisfy the WHERE clause — no code required on your part.

**Column pruning:** Only the columns referenced in the SELECT and WHERE are read from disk automatically.

**Compression:** ZSTD balances compression ratio and decompression speed well for analytics. SNAPPY is faster to decompress but compresses less.

**Metadata cache:** Enable when scanning the same files repeatedly:

```sql
SET parquet_metadata_cache = true;
```

#### Hive partitioning for very large datasets

```sql
-- Partition by date reduces scan to relevant partitions automatically
SELECT * FROM read_parquet('s3://bucket/events/**/*.parquet', hive_partitioning=true)
WHERE year = 2024 AND month = 3;
```


## ASOF Joins

ASOF joins solve the temporal alignment problem: "what was the price *as of* this timestamp?" Each left-side row matches the most recent right-side row with a timestamp `<=` the left timestamp. The left table never grows — at most one match per row.

```sql
-- Classic use case: portfolio valuation at historical prices
SELECT
    h.ticker,
    h.trade_ts,
    h.shares,
    p.price,
    p.price * h.shares AS position_value
FROM holdings h
ASOF JOIN prices p
    ON h.ticker = p.ticker
   AND h.trade_ts >= p.price_ts;

-- Left ASOF — preserve holdings with no price data (price = NULL)
SELECT h.ticker, h.trade_ts, p.price
FROM holdings h
ASOF LEFT JOIN prices p
    ON h.ticker = p.ticker
   AND h.trade_ts >= p.price_ts;

-- Simplified USING syntax (inequality column listed last)
SELECT ticker, h.trade_ts, price
FROM holdings h
ASOF JOIN prices p USING (ticker, trade_ts);
```

**Use cases:** sensor readings at irregular intervals, matching log events to config snapshots, aligning two time series with different cadences, event attribution with processing delay.

**Memory note:** large ASOF joins may spill to disk. The `asof_loop_join_threshold` pragma (default 64) controls the loop-join fallback; raise it when memory is tight but extra time is acceptable:

```sql
PRAGMA asof_loop_join_threshold = 256;
```


## DuckDB vs Pandas vs Polars

| | DuckDB | Polars | Pandas |
| --- | --- | --- | --- |
| **Language** | SQL (Python, R, etc. clients) | Python/Rust, DataFrame API | Python, DataFrame API |
| **Speed vs pandas** | ~9.4× faster | ~8.7× faster | baseline |
| **Paradigm** | Relational SQL | Lazy/eager DataFrame | Eager DataFrame |
| **Larger-than-RAM** | Yes (spill to disk) | Yes (lazy streaming) | No |
| **Complex SQL (CTEs, window fns)** | First class | Workarounds needed | Limited |
| **Ecosystem / visualization** | Needs export to pandas/Arrow | Needs export to pandas | Direct: Seaborn, Plotly, sklearn |
| **Learning curve** | SQL knowledge required | New API to learn | Lowest for Python devs |
| **Zero-copy interop** | Via Apache Arrow | Via Apache Arrow | Arrow backend in v2+ |

### Decision heuristic

- You already know SQL and need to crunch files or DataFrames → DuckDB
- You want a fast modern DataFrame API in Python with lazy optimization → Polars
- You need to hand data to sklearn, Seaborn, or any legacy library → convert to pandas at the end
- Use all three: DuckDB for SQL transforms, Polars for DataFrame logic, pandas only at the output boundary

#### Hybrid pattern (common in notebooks)

```python
import duckdb, polars as pl, pandas as pd

# Heavy aggregation in DuckDB
summary = duckdb.sql("""
    SELECT date_trunc('day', ts) AS day, region, SUM(revenue) AS rev
    FROM 's3://bucket/events/**/*.parquet'
    WHERE ts >= '2024-01-01'
    GROUP BY 1, 2
""").fetch_arrow_table()

# Polars for further transformation (zero-copy from Arrow)
df_pl = pl.from_arrow(summary).with_columns(
    (pl.col("rev") / pl.col("rev").sum()).alias("share")
)

# Pandas for plotting
df_pd = df_pl.to_pandas()
df_pd.plot(x="day", y="rev")
```


## MotherDuck (Cloud DuckDB)

MotherDuck is the managed cloud extension of DuckDB. It adds:

- Cloud-hosted DuckDB databases accessible from any DuckDB client
- Hybrid execution — queries run partly local, partly in the cloud depending on where the data lives
- Collaboration: shared databases accessible to multiple users
- Handles datasets too large for a single machine

```python
# Connect to MotherDuck (token from MotherDuck dashboard)
con = duckdb.connect("md:?motherduck_token=<token>")

# Hybrid query: local Parquet joined to cloud table
con.sql("""
    SELECT l.user_id, c.plan
    FROM 'local_events.parquet' l
    JOIN my_cloud_db.customers c USING (user_id)
""")
```

MotherDuck is well-suited for: teams that need shared analytical databases, datasets too large for a laptop, and replacing a lightweight cloud data warehouse while keeping the DuckDB SQL dialect.


## Practical Notes

- Always prefer Parquet over CSV in any pipeline you control. The read speed difference is dramatic.
- Use built-in DuckDB functions before writing Python UDFs — they run in the vectorized engine; UDFs drop out to Python row-by-row.
- Flatten JSON before aggregating. Nested JSON parsed inline in aggregation queries is noticeably slower than a materialized flattened table.
- The default connection uses all available threads. In a multi-process server context, cap threads to avoid contention: `SET threads = 2`.
- `SUMMARIZE` is the fastest way to profile any new dataset; it runs a single parallel scan.
- DuckDB's SQL dialect is PostgreSQL-compatible for most constructs. Differences are minor and documented.
- For Jupyter notebooks, `%load_ext duckdb_magic` is not needed — `duckdb.sql(...)` works directly in any cell.
