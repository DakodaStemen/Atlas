---
name: big-data-engines
description: Big data processing engines covering Apache Spark (RDDs, DataFrames, SparkSQL, partitioning, joins, serialization, deployment) and Apache Flink (streaming, event time, watermarks, state management, checkpointing, Flink SQL). Use when processing large-scale batch or streaming data.
domain: data-engineering
tags: [spark, flink, big-data, distributed-processing, streaming, batch, sparkSQL, flink-sql]
triggers: apache spark, apache flink, spark, flink, sparkSQL, flink sql, distributed processing, big data engine
---


# Apache Spark & PySpark

## Core Concepts

### DataFrame vs RDD — always prefer DataFrame

RDDs are the low-level distributed collection API. DataFrames (and Datasets in
Scala/Java) sit on top of the Catalyst optimizer and Tungsten execution engine,
which apply predicate pushdown, column pruning, and code generation
automatically. An equivalent RDD job will almost always be slower and harder to
read.

When to accept RDD code: only when you need fine-grained control over partition
layout or are calling a library that requires RDD input and cannot be wrapped.

```python
# Avoid
rdd = sc.parallelize(data).map(lambda r: (r["id"], r["value"]))

# Prefer — Catalyst rewrites this into a columnar plan
df = spark.createDataFrame(data).select("id", "value")
```

### Lazy Evaluation

Transformations (`select`, `filter`, `join`, `groupBy`) build a logical plan;
nothing runs until an action (`count`, `collect`, `write`, `show`) is called.
The query optimizer rewrites the plan at action time.

Practical consequence: calling `df.filter(...).filter(...)` incurs no extra cost
over a single filter — Catalyst combines them. Calling `.count()` twice executes
the full plan twice; cache if you need to reuse.

### Transformations vs Actions

| Category | Examples | Notes |
| ---------- | ---------- | ------- |
| Narrow transformation | `map`, `filter`, `select`, `withColumn` | No shuffle; processed within a partition |
| Wide transformation | `groupBy`, `join`, `distinct`, `repartition` | Requires shuffle; expensive |
| Action | `count`, `collect`, `show`, `write`, `save` | Triggers execution |

Minimize wide transformations; when unavoidable, place them as late in the
pipeline as possible so filters have already reduced data volume.


## Partitioning

### How Spark Partitions Work

Each partition is an independent chunk of data processed by one task on one
executor. Too few partitions → cores sit idle; too many → scheduler overhead and
small-file problems.

Rule of thumb: target 100–200 MB of uncompressed data per partition, and set
`spark.sql.shuffle.partitions` to roughly 2–3× the number of available executor
cores.

Default `spark.sql.shuffle.partitions` is 200, which is fine for a large cluster
but will create hundreds of tiny tasks on a small one.

```python
spark.conf.set("spark.sql.shuffle.partitions", "48")  # tune to your cluster
```

### repartition vs coalesce

| | `repartition(n)` | `coalesce(n)` |
| --- | --- | --- |
| Direction | increase or decrease | decrease only |
| Shuffle | full shuffle | no shuffle (narrow) |
| Partition balance | perfectly balanced (round-robin by default) | uneven — existing partitions merged |
| Use case | before a join on a skewed key; increase parallelism | reduce partition count before writing without a shuffle |

```python
# After filtering down to 5% of rows, reduce partitions cheaply
small_df = large_df.filter(F.col("region") == "EU").coalesce(8)

# Before joining on a column with known skew, force even distribution
balanced = skewed_df.repartition(200, F.col("join_key"))
```

Prefer `repartition(n, col)` over plain `repartition(n)` when downstream joins
or aggregations use that column — it co-locates matching keys and eliminates a
second shuffle.


## Caching and Persistence

Cache DataFrames that are reused more than once in the same job (e.g., a cleaned
base table referenced by multiple downstream aggregations).

```python
df.cache()          # shorthand for MEMORY_AND_DISK
df.persist(StorageLevel.MEMORY_AND_DISK_SER)
df.unpersist()      # release when done
```

Storage levels (from `pyspark.StorageLevel`):

| Level | Memory | Disk | Serialized | Notes |
| ------- | -------- | ------ | ------------ | ------- |
| `MEMORY_ONLY` | yes | no | no | Fastest; recomputes partitions that don't fit |
| `MEMORY_AND_DISK` | yes | yes | no | Default `cache()`; spills to disk |
| `MEMORY_ONLY_SER` | yes | no | yes | Smaller footprint; CPU cost to deserialize |
| `MEMORY_AND_DISK_SER` | yes | yes | yes | Good default for large DataFrames |
| `DISK_ONLY` | no | yes | yes | Low memory pressure; high read latency |
| `OFF_HEAP` | off-heap | no | yes | Avoids GC; requires `spark.memory.offHeap.enabled=true` |

Practical guidance:

- Cache at the point where the DataFrame is fully cleaned/joined, not before.
- Call `unpersist()` explicitly; relying on LRU eviction delays memory release.
- Do not cache a DataFrame that is only used once.
- In Databricks, `DELTA_CACHE` (the disk-level SSD cache) is separate from
  Spark's in-memory cache and is managed automatically.


## Data Skew — Diagnosing and Fixing

Symptoms: one task runs 10× longer than the median in the Spark UI stage view;
the executor tab shows one executor consuming much more memory.

### Fix 1 — Rely on AQE skew join (Spark 3.x+)

Enable AQE and increase `skewedPartitionThresholdInBytes` to match your actual
skewed partition sizes. No code change required.

### Fix 2 — Salting

Add a random salt to the skewed key to distribute it across multiple partitions,
then join and aggregate in two phases.

```python
import pyspark.sql.functions as F

SALT = 10

# Explode the lookup side with all salt values
lookup_salted = lookup_df.withColumn(
    "salt", F.explode(F.array([F.lit(i) for i in range(SALT)]))
).withColumn("salted_key", F.concat_ws("_", F.col("join_key"), F.col("salt")))

# Salt the fact side randomly
fact_salted = fact_df.withColumn(
    "salted_key",
    F.concat_ws("_", F.col("join_key"), (F.rand() * SALT).cast("int").cast("string"))
)

result = fact_salted.join(lookup_salted, on="salted_key").drop("salt", "salted_key")
```

### Fix 3 — Isolate the hot key

Process the dominant key (e.g., `null` or a top-N value) separately with a
broadcast join, union back with the rest.


## File Formats

Prefer **Parquet** (columnar, splittable, predicate pushdown) or **Delta Lake**
(Parquet + ACID + time travel) for all persistent data. Avoid CSV and JSON as
intermediate formats — they are row-oriented and not splittable without custom
logic.

```python
# Write
df.write.mode("overwrite").parquet("s3://bucket/path/")

# Read with partition pruning (Spark reads only matching directories)
df = spark.read.parquet("s3://bucket/events/").filter(F.col("date") == "2024-01-15")

# Delta Lake
df.write.format("delta").mode("overwrite").save("/delta/events")
spark.read.format("delta").load("/delta/events")
```

For reads: `spark.sql.files.maxPartitionBytes` (default 128 MB) controls how
large Spark allows a partition to grow when reading file sources. Lowering this
increases parallelism; raising it reduces task count.


## Databricks-Specific Patterns

### Delta Cache vs Spark Cache

Databricks clusters have an SSD-backed Delta cache (also called I/O cache) that
caches remote Parquet/Delta files locally on the executor disk. It is transparent
— no API call needed. Spark's in-memory `cache()` is still useful for DataFrames
that are repeatedly transformed in a single job.

### Auto Optimize and Auto Compact

```sql
ALTER TABLE my_table SET TBLPROPERTIES (
  'delta.autoOptimize.optimizeWrite' = 'true',
  'delta.autoOptimize.autoCompact' = 'true'
);
```

Avoids small-file problems in streaming Delta sinks without manual `OPTIMIZE`
runs.

### OPTIMIZE and ZORDER

```sql
OPTIMIZE my_table ZORDER BY (user_id, event_date);
```

ZORDERing co-locates related data in files, maximizing the effect of Parquet
data-skipping for queries that filter on those columns.

### Photon Engine

Databricks' vectorized Photon engine accelerates SQL and DataFrame operations
automatically on Premium clusters. No code changes required; it is most effective
for aggregations, joins, and scans.

### Structured Streaming on Databricks — Trigger.AvailableNow

Preferred for scheduled incremental pipelines (replaces cron + batch jobs):

```python
(
    spark.readStream.format("delta").load("/delta/raw")
    .writeStream
    .format("delta")
    .trigger(availableNow=True)
    .option("checkpointLocation", "/checkpoints/incremental")
    .start()
    .awaitTermination()
)
```

### Unity Catalog

Reference tables as `catalog.schema.table`. Use `spark.table()` or SQL; avoid
hardcoded HDFS/S3 paths in production jobs.

```python
df = spark.table("main.sales.orders")
df.writeTo("main.sales.orders_clean").createOrReplace()
```


## Quick Reference: Key Configuration Properties

```text
# Execution
spark.sql.shuffle.partitions                              200
spark.sql.files.maxPartitionBytes                        134217728 (128 MB)
spark.sql.adaptive.enabled                               true
spark.sql.adaptive.coalescePartitions.enabled            true
spark.sql.adaptive.advisoryPartitionSizeInBytes          67108864 (64 MB)
spark.sql.adaptive.skewJoin.enabled                      true
spark.sql.adaptive.skewJoin.skewedPartitionFactor        5.0
spark.sql.adaptive.skewJoin.skewedPartitionThresholdInBytes  268435456 (256 MB)

# Joins
spark.sql.autoBroadcastJoinThreshold                     10485760 (10 MB)
spark.sql.broadcastTimeout                               300 (seconds)

# Caching
spark.sql.inMemoryColumnarStorage.compressed             true
spark.sql.inMemoryColumnarStorage.batchSize              10000

# Memory (set in cluster config)
spark.executor.memory                                    8g
spark.executor.memoryOverhead                            2g
spark.driver.memory                                      4g
spark.memory.fraction                                    0.6
spark.memory.storageFraction                             0.5
spark.memory.offHeap.enabled                             false

# Serialization
spark.serializer                                         org.apache.spark.serializer.KryoSerializer
```

---


# Apache Flink Stateful Stream Processing

## When to Use (vs Spark Streaming, vs Kafka Streams)

| Dimension | Flink | Spark Structured Streaming | Kafka Streams |
| --- | --- | --- | --- |
| Latency | True sub-millisecond (record-at-a-time) | Mini-batch, tens of ms minimum | Low (embedded in app process) |
| Statefulness | First-class keyed state, large state via RocksDB | External stores needed for large state | RocksDB-backed local state |
| Exactly-once | Native (aligned/unaligned checkpoints + 2PC sinks) | Native with WAL | Exactly-once transactions |
| Deployment | Standalone, YARN, Kubernetes | Spark cluster | Embedded JVM library |
| Operational complexity | High (separate cluster, JobManager HA) | Medium (Spark cluster) | Low (library, no cluster) |
| Best fit | Complex CEP, large mutable state, multi-stream joins, event-time correctness | Ad-hoc analytics, SQL-first teams already on Spark | Simple per-topic enrichment/aggregation |

Choose Flink when you need sub-second latency with large persistent state, complex event-time semantics, or multi-stream joins that exceed what Kafka Streams' changelog-based model handles cleanly.


## Event Time and Watermarks

Flink supports three time semantics:

- **Processing time** — wall-clock of the machine running the operator. Fast, non-deterministic.
- **Ingestion time** — timestamp assigned at source entry. Rarely used.
- **Event time** — timestamp embedded in the event itself. Deterministic, handles out-of-order data correctly.

A **watermark** with value `t` signals "no more events with timestamp ≤ t will arrive." Watermarks flow inline with the stream and advance the internal event-time clock at each operator.

```java
// Bounded out-of-orderness: assume events are at most 5 seconds late
WatermarkStrategy<MyEvent> strategy = WatermarkStrategy
    .<MyEvent>forBoundedOutOfOrderness(Duration.ofSeconds(5))
    .withTimestampAssigner((event, recordTimestamp) -> event.getEventTimestamp());

DataStream<MyEvent> stream = env.fromSource(
    mySource, strategy, "my-source"
);
```

For idle sources (e.g. partitions with no traffic), add `.withIdleness(Duration.ofMinutes(1))` to prevent watermark stalling.

**Custom watermark generator** for periodic emission:

```java
public class BoundedOutOfOrdernessGenerator
        implements WatermarkGenerator<MyEvent> {
    private final long maxOutOfOrderness = 5_000; // 5 sec
    private long currentMaxTimestamp;

    @Override
    public void onEvent(MyEvent event, long eventTimestamp,
                        WatermarkOutput output) {
        currentMaxTimestamp = Math.max(currentMaxTimestamp, eventTimestamp);
    }

    @Override
    public void onPeriodicEmit(WatermarkOutput output) {
        output.emitWatermark(new Watermark(currentMaxTimestamp - maxOutOfOrderness - 1));
    }
}
```

In multi-input operators, event time advances to the **minimum** across all input streams.


## Keyed State

Keyed state is a per-key embedded key-value store co-located with the operator. Only accessible inside `keyBy()` partitioned streams.

State is declared via descriptors and fetched from `RuntimeContext` inside `RichFunction.open()`.

### ValueState

```java
public class RunningAverageFunction
        extends RichFlatMapFunction<Tuple2<Long, Long>, Tuple2<Long, Long>> {

    private transient ValueState<Tuple2<Long, Long>> sumAndCount;

    @Override
    public void open(OpenContext ctx) {
        ValueStateDescriptor<Tuple2<Long, Long>> descriptor =
            new ValueStateDescriptor<>(
                "sumAndCount",
                TypeInformation.of(new TypeHint<Tuple2<Long, Long>>() {}),
                Tuple2.of(0L, 0L));
        sumAndCount = getRuntimeContext().getState(descriptor);
    }

    @Override
    public void flatMap(Tuple2<Long, Long> input,
                        Collector<Tuple2<Long, Long>> out) throws Exception {
        Tuple2<Long, Long> current = sumAndCount.value();
        current.f0 += 1;
        current.f1 += input.f1;
        sumAndCount.update(current);

        if (current.f0 >= 100) {
            out.collect(Tuple2.of(input.f0, current.f1 / current.f0));
            sumAndCount.clear();
        }
    }
}
```

### ListState and MapState

```java
// ListState — append-only list per key
ListStateDescriptor<String> listDesc =
    new ListStateDescriptor<>("events", String.class);
ListState<String> listState = getRuntimeContext().getListState(listDesc);
listState.add("new-event");
for (String e : listState.get()) { /* ... */ }

// MapState — map per key (efficient random access, avoids full deserialization)
MapStateDescriptor<String, Long> mapDesc =
    new MapStateDescriptor<>("counts", String.class, Long.class);
MapState<String, Long> mapState = getRuntimeContext().getMapState(mapDesc);
mapState.put("key", mapState.getOrDefault("key", 0L) + 1);
```

### State TTL

```java
StateTtlConfig ttlConfig = StateTtlConfig
    .newBuilder(Duration.ofHours(24))
    .setUpdateType(StateTtlConfig.UpdateType.OnCreateAndWrite)
    .setStateVisibility(StateTtlConfig.StateVisibility.NeverReturnExpired)
    .cleanupIncrementally(1000, true)   // check 1000 entries per cleanup, cleanup on read
    .build();

ValueStateDescriptor<MyState> descriptor =
    new ValueStateDescriptor<>("my-state", MyState.class);
descriptor.enableTimeToLive(ttlConfig);
```

### State backend choice

- **HashMapStateBackend** (default): state in JVM heap. Fast for small-to-medium state. Checkpoints to a configured filesystem. GC pressure at large scale.
- **EmbeddedRocksDBStateBackend**: state on disk via RocksDB. Supports state sizes far exceeding heap. Supports incremental checkpoints. ~2–10x slower per state access than heap.

```java
// Switch to RocksDB
env.setStateBackend(new EmbeddedRocksDBStateBackend(true)); // true = incremental

// Configure via flink-conf.yaml (preferred for ops)
// state.backend.type: rocksdb
// state.backend.incremental: true
// state.checkpoints.dir: hdfs:///flink/checkpoints
```


## Kafka Integration

### KafkaSource

```java
KafkaSource<String> source = KafkaSource.<String>builder()
    .setBootstrapServers("kafka:9092")
    .setTopics("input-topic")
    .setGroupId("flink-consumer-group")
    .setStartingOffsets(OffsetsInitializer.committedOffsets(OffsetResetStrategy.EARLIEST))
    .setValueOnlyDeserializer(new SimpleStringSchema())
    .build();

// With per-record event time watermarks
WatermarkStrategy<MyEvent> watermarks = WatermarkStrategy
    .<MyEvent>forBoundedOutOfOrderness(Duration.ofSeconds(5))
    .withTimestampAssigner((e, ts) -> e.getTimestamp())
    .withIdleness(Duration.ofMinutes(1));   // handle idle partitions

DataStream<MyEvent> stream = env.fromSource(
    kafkaSource, watermarks, "Kafka Source"
);
```

### KafkaSink with exactly-once

Exactly-once uses Kafka transactions (2PC). The Flink checkpoint commits the Kafka transaction atomically.

```java
KafkaSink<String> sink = KafkaSink.<String>builder()
    .setBootstrapServers("kafka:9092")
    .setRecordSerializer(KafkaRecordSerializationSchema.builder()
        .setTopic("output-topic")
        .setValueSerializationSchema(new SimpleStringSchema())
        .build())
    .setDeliveryGuarantee(DeliveryGuarantee.EXACTLY_ONCE)
    .setTransactionalIdPrefix("flink-txn-")   // must be unique per job
    .setKafkaProducerConfig(producerProps)
    .build();

stream.sinkTo(sink);
```

#### Requirements for exactly-once

- Checkpointing must be enabled with `EXACTLY_ONCE` mode.
- `transaction.timeout.ms` on the Kafka broker must be > checkpoint interval + restart time.
- Each parallel sink instance gets a unique transactional ID derived from the prefix + subtask index.
- Consumers must set `isolation.level=read_committed` to see only committed records.

**At-least-once** is simpler and appropriate when downstream deduplication handles duplicates:

```java
.setDeliveryGuarantee(DeliveryGuarantee.AT_LEAST_ONCE)
```


## Flink on Kubernetes

Flink supports native Kubernetes deployment. The Flink Kubernetes Operator (separate project) provides the `FlinkDeployment` CRD for declarative management.

### Application mode (recommended for production)

Each job gets its own cluster. JobManager runs the user code. Resources are released on job completion.

```bash
# Deploy application mode job
./bin/flink run-application \
  --target kubernetes-application \
  -Dkubernetes.cluster-id=my-app \
  -Dkubernetes.container.image.ref=my-registry/my-flink-job:1.0 \
  -Dkubernetes.namespace=flink \
  -Djobmanager.memory.process.size=1g \
  -Dtaskmanager.memory.process.size=2g \
  -Dtaskmanager.numberOfTaskSlots=2 \
  local:///opt/flink/usrlib/my-job.jar
```

### Session mode

Reusable cluster; multiple jobs share TaskManagers. More efficient for many small jobs, harder to isolate failures.

```bash
./bin/kubernetes-session.sh \
  -Dkubernetes.cluster-id=flink-session \
  -Dkubernetes.namespace=flink \
  -Dkubernetes.rest-service.exposed.type=ClusterIP

# Submit a job to existing session
./bin/flink run --target kubernetes-session \
  -Dkubernetes.cluster-id=flink-session ./my-job.jar
```

### High availability

```yaml
# flink-conf.yaml (or operator CRD spec)
high-availability.type: kubernetes
high-availability.storageDir: s3://my-bucket/flink/ha/
kubernetes.jobmanager.replicas: 2
```

RBAC prerequisite:

```bash
kubectl create clusterrolebinding flink-role-binding \
  --clusterrole=edit \
  --serviceaccount=flink:default
```

### Flink Kubernetes Operator (FlinkDeployment CRD)

```yaml
apiVersion: flink.apache.org/v1beta1
kind: FlinkDeployment
metadata:
  name: my-flink-job
  namespace: flink
spec:
  image: my-registry/my-flink-job:1.0
  flinkVersion: v1_18
  flinkConfiguration:
    taskmanager.numberOfTaskSlots: "2"
    state.backend.type: rocksdb
    state.backend.incremental: "true"
    state.checkpoints.dir: s3://bucket/checkpoints
    execution.checkpointing.interval: "10000"
    high-availability.type: kubernetes
    high-availability.storageDir: s3://bucket/ha
  serviceAccount: flink
  jobManager:
    resource:
      memory: "1024m"
      cpu: 0.5
    replicas: 2
  taskManager:
    resource:
      memory: "2048m"
      cpu: 1
  job:
    jarURI: local:///opt/flink/usrlib/my-job.jar
    parallelism: 4
    upgradeMode: savepoint
```


## Critical Rules and Gotchas

**Serialization.** Flink needs to serialize state and network records. Always use Flink's `TypeInformation` system or register Kryo/custom serializers. Avoid raw Java serialization — it breaks schema evolution. Register Kryo types explicitly:

```java
env.getConfig().registerTypeWithKryoSerializer(MyClass.class, MySerializer.class);
```

POJO types (all fields public or with getters/setters, no-arg constructor) are handled most efficiently. Avoid generic types that erase at runtime.

**State migration.** Changing a state descriptor name or type between deployments will lose or corrupt state on restore. Use `StateDescriptor.setQueryable()` only when you understand the implications. For schema evolution, use Avro or Protobuf serializers, not POJO auto-serialization.

**Watermark lateness.** Parallelism > Kafka partition count causes some subtasks to have no input, stalling watermarks for the whole operator. Fix: `.withIdleness(Duration.ofMinutes(1))` on the `WatermarkStrategy`.

**Late elements.** Without `allowedLateness`, late elements are silently dropped by default. Add `sideOutputLateData` to diagnose how many elements are being lost. In production, monitor the late data side output volume.

**Exactly-once Kafka sink pitfall.** The Kafka transaction timeout must exceed `checkpoint interval + max restart time`. If Kafka aborts the transaction before Flink commits it, you get data loss despite exactly-once configuration. Set `transaction.max.timeout.ms` on brokers and `transaction.timeout.ms` in producer config accordingly.

**State TTL cleanup is not guaranteed immediate.** TTL marks state as expired but actual removal happens lazily (on access) or via background compaction in RocksDB. Do not rely on expired state being absent from snapshots immediately.

**Global window requires a custom trigger.** Without it, the window never fires. A common bug is forgetting to attach a `CountTrigger` or `PurgingTrigger`.

**Avoid object reuse across records.** Flink may reuse objects between calls for efficiency. If you store a reference to an input object in state or a collection, clone it first.

**Non-keyed state (operator state).** `ListState` in non-keyed contexts (e.g., Kafka source offset tracking) uses `CheckpointedFunction`. Do not confuse with keyed state accessed via `RuntimeContext`.

