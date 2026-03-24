---
name: vector-databases
description: Use when the user is working with vector databases, embeddings storage, ANN indexing, or semantic/hybrid search. Covers Pinecone, Weaviate, Qdrant, Chroma, and pgvector — including when to pick each, HNSW vs IVFFlat tradeoffs, filtering strategies, upsert patterns, namespace/collection management, hybrid search, and index tuning.
domain: data
category: vector
tags: [vector-database, Pinecone, Weaviate, Qdrant, Chroma, pgvector, HNSW, IVFFlat, ANN, RAG, embeddings, hybrid-search]
triggers: ["vector database", "vector search", "embeddings storage", "ANN index", "HNSW", "IVFFlat", "pgvector", "Pinecone", "Qdrant", "Weaviate", "Chroma", "semantic search", "RAG retrieval"]
---

# Vector Databases

## When to Use Which Database

Pick the database that matches your deployment model and data size. The wrong choice here is a painful migration later.

| Situation | Use |
| --- | --- |
| Already on PostgreSQL, < 5M vectors | **pgvector** — one extension, no new infra |
| Local dev / prototype / notebook | **Chroma** — zero-config, in-process |
| Self-hosted, production, need filtering | **Qdrant** — best-in-class filtering, Rust performance |
| Managed, compliance (HIPAA/SOC2), minimal ops | **Pinecone** — fully managed, BYOC option |
| Hybrid search + knowledge graph + GraphQL | **Weaviate** — native BM25 + vector + modularity |
| Multi-tenant SaaS, per-tenant isolation | **Qdrant** — first-class tenant lifecycle APIs |
| > 100M vectors, managed | **Pinecone** or **Weaviate Cloud** |

### Chroma

Best for: local development, Jupyter notebooks, single-process Python apps.

- Runs embedded in-process; no server required.
- Chroma 2025 Rust rewrite delivers ~4x faster writes/queries vs the original Python implementation.
- Not designed for multi-region, multi-tenant production at scale.
- No compliance certifications, no horizontal sharding.
- Use it to iterate fast. Swap it out before you go to prod with millions of vectors.

### Qdrant

Best for: self-hosted production, cost-sensitive workloads, advanced metadata filtering.

- Written in Rust; compact memory footprint allows edge deployments.
- 1 GB forever-free tier; paid plans from $25/month.
- JSON-based payload filters that combine multiple conditions without degrading recall (custom filtered HNSW — see Filtering section).
- First-class multi-tenancy: named collections/tenants with quota controls and dedicated shard options.
- Native sparse vector support, BM25 scoring, and geo search — Pinecone has none of these.
- Supports multi-vector-per-document (ColBERT-style late interaction).

### Pinecone

Best for: fully managed, compliance-first, teams that want zero infrastructure work.

- Serverless autoscaling; no pods to size.
- Mature compliance certifications (HIPAA, SOC2); BYOC (Bring Your Own Cloud) for hard data isolation.
- Namespace-based logical isolation; up to 100,000 namespaces on standard plans, but only 20 indexes.
- Notable gaps vs Qdrant/Weaviate: no native hybrid search (BM25), no sparse vectors in base product, no geo search, no facets.
- Pinecone Assistant (GA Jan 2025) wraps chunking, embedding, search, reranking, and generation in one endpoint — useful but opinionated.
- Cost model: $0.33/GB storage + per-operation charges. At high volume, self-hosted Qdrant or Weaviate is significantly cheaper.

### Weaviate

Best for: hybrid search, knowledge-graph-style schemas, server-side RAG.

- GraphQL-first API; also exposes REST and gRPC.
- Native hybrid search: vector similarity + BM25 keyword matching in one query, with configurable alpha weighting.
- Generative module: retrieval + LLM call in one server-side round trip — no extra network hop.
- Pluggable embedding models and rerankers via module system.
- Throughput benchmark: ~791 QPS in published comparisons vs Qdrant ~326 QPS, Pinecone ~150 QPS — though numbers vary heavily by hardware and query type.

### pgvector

Best for: teams already on Postgres with < 1–5M vectors and no desire for a separate service.

- Install once: `CREATE EXTENSION vector;`
- Supports HNSW and IVFFlat indexes, cosine/L2/inner-product distance operators, and hybrid search via Postgres full-text search (`ts_vector`/`ts_query`) in the same query.
- Scale vertically the same way you scale Postgres. For > 100M vectors, consider table partitioning or migrating to a purpose-built DB.
- `pgvectorscale` (Timescale) adds DiskANN with Statistical Binary Quantization — better disk efficiency for large datasets.

---

## ANN Algorithm Comparison: HNSW vs IVFFlat

### HNSW (Hierarchical Navigable Small World)

Graph-based index. Vectors are linked to neighbors across multiple layers. Search navigates the hierarchy greedily.

#### Parameters

| Parameter | Default | Effect |
| --- | --- | --- |
| `m` | 16 | Edges per node per layer. Higher = better recall + more memory + slower build. 12–16 is the standard starting point. |
| `ef_construction` | 64 | Candidate pool size during build. Higher = better graph quality + slower build. Minimum: `4 * m`. |
| `ef_search` | 40 | Candidate pool size at query time. Higher = better recall + higher latency. Tune this at runtime. |

#### When HNSW wins

- Query latency is your primary concern.
- Dataset is mostly static (builds are slow; full rebuilds are expensive).
- High-dimensional vectors (HNSW degrades much more gracefully with dimension count than IVFFlat).
- You can afford the RAM — HNSW is an in-memory index; the full graph must fit.

**Build time:** Slow. At large scale, 4–32x slower to build than IVFFlat.

**Memory:** O(n *m* dim). For 10M 1536-dim vectors with m=16, plan for ~30–50 GB RAM for the index alone.

### IVFFlat (Inverted File with Flat Storage)

Clustering-based index. Vectors are assigned to one of `lists` centroids at build time. Search probes `nprobe` clusters.

#### Parameters (IVFFlat (Inverted File with Flat Storage))

| Parameter | Default | Effect |
| --- | --- | --- |
| `lists` | 100 | Number of clusters. Rule of thumb: `rows / 1000` up to 1M rows; `sqrt(rows)` above 1M. |
| `probes` (`nprobe`) | 1 | Clusters searched at query time. Start at `sqrt(lists)`. Higher = better recall + higher latency. |

#### When IVFFlat wins

- Frequent index rebuilds are required (IVFFlat builds 4–32x faster).
- Memory is constrained — IVFFlat is not purely in-memory.
- Dataset is large and growing; incremental updates are common.
- Under low-selectivity filters, recent benchmarks (2024–2025) show IVFFlat can outperform HNSW.

**Recall tradeoff:** IVFFlat generally delivers lower recall-per-latency than HNSW. At the same target recall, HNSW is faster.

**Requires data before building:** IVFFlat must train on representative data to compute centroids. Build the index after loading a substantial fraction of your dataset, not on an empty table.

### pgvector SQL Syntax

```sql
-- HNSW index (cosine similarity)
CREATE INDEX ON items USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- IVFFlat index (L2 distance)
CREATE INDEX ON items USING ivfflat (embedding vector_l2_ops)
WITH (lists = 1000);

-- Distance operators
-- <->  L2 distance
-- <=>  cosine distance
-- <#>  negative inner product (use for normalized vectors)
-- <+>  L1 distance

-- Query — nearest 10 by cosine
SELECT id, content
FROM items
ORDER BY embedding <=> '[0.1, 0.2, ...]'::vector
LIMIT 10;

-- Tune ef_search at session level (HNSW)
SET hnsw.ef_search = 100;

-- Tune probes at session level (IVFFlat)
SET ivfflat.probes = 10;
```

---

## Upsert Patterns

### pgvector (Upsert Patterns)

```sql
INSERT INTO items (id, embedding, metadata)
VALUES (1, '[0.1, 0.2, ...]'::vector, '{"source": "doc-42"}')
ON CONFLICT (id) DO UPDATE
  SET embedding = EXCLUDED.embedding,
      metadata  = EXCLUDED.metadata;
```

Disable indexes during bulk load, then rebuild — index maintenance during `COPY` is expensive:

```sql
DROP INDEX IF EXISTS items_embedding_idx;
COPY items (id, embedding, metadata) FROM '/path/to/data.csv' CSV;
CREATE INDEX ON items USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);
```

### Qdrant (Upsert Patterns)

```python
from qdrant_client import QdrantClient
from qdrant_client.models import PointStruct, VectorParams, Distance

client = QdrantClient(url="http://localhost:6333")

# Create collection
client.recreate_collection(
    collection_name="docs",
    vectors_config=VectorParams(size=1536, distance=Distance.COSINE),
)

# Upsert — idempotent by point id
client.upsert(
    collection_name="docs",
    points=[
        PointStruct(id=1, vector=[0.1, 0.2, ...], payload={"source": "doc-42", "year": 2024}),
        PointStruct(id=2, vector=[0.3, 0.4, ...], payload={"source": "doc-43", "year": 2023}),
    ],
)
```

### Pinecone (Upsert Patterns)

```python
import pinecone

pc = pinecone.Pinecone(api_key="...")
index = pc.Index("my-index")

# Upsert into a namespace
index.upsert(
    vectors=[
        {"id": "doc-1", "values": [0.1, 0.2, ...], "metadata": {"source": "web", "year": 2024}},
        {"id": "doc-2", "values": [0.3, 0.4, ...], "metadata": {"source": "pdf", "year": 2023}},
    ],
    namespace="tenant-abc",
)
```

Pinecone upsert is idempotent — same `id` in the same namespace overwrites the existing vector and metadata.

### Chroma (Upsert Patterns)

```python
import chromadb

client = chromadb.Client()  # in-memory; use chromadb.PersistentClient(path=...) to persist
collection = client.get_or_create_collection("docs")

collection.upsert(
    ids=["doc-1", "doc-2"],
    embeddings=[[0.1, 0.2, ...], [0.3, 0.4, ...]],
    metadatas=[{"source": "web"}, {"source": "pdf"}],
    documents=["text of doc 1", "text of doc 2"],
)
```

---

## Metadata Filtering

The core challenge: applying a filter pre-search risks fragmenting the ANN graph (missing nearby vectors), while post-search filtering wastes recall budget on vectors that will be thrown away. Qdrant and Weaviate solve this with graph-aware custom filtering.

### Qdrant — payload filters

```python
from qdrant_client.models import Filter, FieldCondition, MatchValue, Range

results = client.search(
    collection_name="docs",
    query_vector=[0.1, 0.2, ...],
    query_filter=Filter(
        must=[
            FieldCondition(key="year", range=Range(gte=2023)),
            FieldCondition(key="source", match=MatchValue(value="web")),
        ]
    ),
    limit=10,
)
```

Qdrant evaluates filters against the HNSW graph without rebuilding it — recall degrades gracefully under high selectivity.

### pgvector — SQL WHERE clause

```sql
-- Metadata stored as jsonb column
SELECT id, content
FROM items
WHERE metadata->>'source' = 'web'
  AND (metadata->>'year')::int >= 2023
ORDER BY embedding <=> $1
LIMIT 10;

-- Create B-tree index on frequently filtered columns for speed
CREATE INDEX ON items ((metadata->>'source'));
CREATE INDEX ON items (((metadata->>'year')::int));
```

For very selective filters (< 5% of rows), the planner may choose a sequential scan over the vector index — that is correct behavior. Use `EXPLAIN ANALYZE` to verify the plan.

### Pinecone — metadata filter

```python
index.query(
    vector=[0.1, 0.2, ...],
    filter={"source": {"$eq": "web"}, "year": {"$gte": 2023}},
    top_k=10,
    namespace="tenant-abc",
)
```

Pinecone applies filters post-retrieval against the metadata index. For high-selectivity filters (< 1% match), this can return fewer than `top_k` results.

---

## Namespace and Collection Management

### Pinecone namespaces

A namespace is a logical partition within an index. All vectors, upserts, queries, and deletes are namespace-scoped. Useful for per-user or per-tenant isolation without separate indexes.

```python
# Write to namespace
index.upsert(vectors=[...], namespace="user-123")

# Query within namespace
index.query(vector=[...], top_k=5, namespace="user-123")

# Delete all vectors in a namespace
index.delete(delete_all=True, namespace="user-123")
```

Limit: 100,000 namespaces per index on standard plans. Do not use one namespace per document — that defeats the purpose.

### Qdrant collections and tenants

Each collection has its own HNSW graph, distance metric, and vector size. Collections are isolated; there is no cross-collection search.

For multi-tenant within one collection, use a `tenant_id` payload field and filter on it. Qdrant Cloud also supports named tenants with dedicated shard allocation for hard isolation.

```python
# Create tenant-isolated collection
client.create_collection("docs", vectors_config=VectorParams(size=1536, distance=Distance.COSINE))

# Per-tenant upsert (payload-based tenancy)
client.upsert("docs", points=[
    PointStruct(id=1, vector=[...], payload={"tenant_id": "acme", "doc": "report-q1"}),
])

# Per-tenant query
client.search("docs", query_vector=[...], query_filter=Filter(
    must=[FieldCondition(key="tenant_id", match=MatchValue(value="acme"))]
), limit=10)
```

### Chroma collections

```python
# Collections are the top-level isolation unit
col = client.get_or_create_collection("project-alpha")

# Delete a collection entirely
client.delete_collection("project-alpha")
```

---

## Hybrid Search

Hybrid search combines dense vector similarity with sparse keyword (BM25) matching. This beats pure vector search when queries contain rare or domain-specific terms the embedding model generalizes away.

### Weaviate — native hybrid

```python
import weaviate

client = weaviate.connect_to_local()
collection = client.collections.get("Article")

results = collection.query.hybrid(
    query="transformer architecture attention mechanism",
    alpha=0.5,          # 0 = pure BM25, 1 = pure vector, 0.5 = equal weight
    limit=10,
)
```

### Qdrant — sparse + dense fusion

Qdrant supports sparse vectors (BM25 or SPLADE embeddings) alongside dense vectors. Combine them via Reciprocal Rank Fusion (RRF) or Distribution-Based Score Fusion (DBSF):

```python
from qdrant_client.models import SparseVector, NamedSparseVector, NamedVector, Prefetch, FusionQuery, Fusion

# Assumes collection has both dense and sparse vector spaces
results = client.query_points(
    collection_name="docs",
    prefetch=[
        Prefetch(query=NamedVector(name="dense", vector=[0.1, 0.2, ...]), limit=20),
        Prefetch(query=NamedSparseVector(name="sparse", vector=SparseVector(indices=[10, 42], values=[0.8, 0.3])), limit=20),
    ],
    query=FusionQuery(fusion=Fusion.RRF),
    limit=10,
)
```

### pgvector — manual hybrid

```sql
-- BM25-style keyword rank via ts_rank
WITH semantic AS (
    SELECT id, embedding <=> $1 AS vec_dist
    FROM items
    ORDER BY vec_dist LIMIT 50
),
keyword AS (
    SELECT id, ts_rank(to_tsvector('english', content), plainto_tsquery('english', $2)) AS kw_rank
    FROM items
    WHERE to_tsvector('english', content) @@ plainto_tsquery('english', $2)
    LIMIT 50
)
SELECT
    COALESCE(s.id, k.id) AS id,
    (1.0 / (60 + ROW_NUMBER() OVER (ORDER BY s.vec_dist)))    -- RRF from semantic leg
    + (1.0 / (60 + ROW_NUMBER() OVER (ORDER BY k.kw_rank DESC))) AS rrf_score
FROM semantic s
FULL OUTER JOIN keyword k ON s.id = k.id
ORDER BY rrf_score DESC
LIMIT 10;
```

---

## Index Tuning and Performance

### HNSW tuning workflow

1. Start with `m=16`, `ef_construction=64`, `ef_search=40`.
2. Measure recall against a ground-truth set (exact nearest neighbors via a sequential scan).
3. If recall is too low: raise `ef_search` first (no rebuild needed), then raise `m` and `ef_construction` and rebuild.
4. If latency is too high: lower `ef_search`; accept the recall hit.
5. `ef_construction` must always be `>= 2 * m`.

### IVFFlat tuning workflow

1. Set `lists = sqrt(row_count)` for > 1M rows; `lists = row_count / 1000` for smaller.
2. Start with `probes = sqrt(lists)`.
3. Raise `probes` until recall target is met; each step adds latency linearly.
4. Never build IVFFlat on an empty or near-empty table — centroids need representative data.
5. Schedule periodic index rebuilds if your data distribution shifts significantly.

### pgvector memory settings

```sql
-- Allow more RAM for index build (session-level)
SET maintenance_work_mem = '8GB';

-- pgvector will warn you if the graph no longer fits:
-- "WARNING: hnsw graph build: not enough memory"
-- If you see this, raise maintenance_work_mem or reduce m.
```

### Bulk load pattern (pgvector)

```sql
-- Fastest: load raw data first, then build index
BEGIN;
ALTER TABLE items SET UNLOGGED;       -- optional: skip WAL during load
COPY items (id, embedding, content) FROM STDIN;
ALTER TABLE items SET LOGGED;
COMMIT;

SET maintenance_work_mem = '4GB';
CREATE INDEX ON items USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64);
ANALYZE items;
```

---

## Cold Start Issues

**IVFFlat cold start (pgvector):** The index must be trained on representative data. Querying before the table has enough rows produces poor centroids. Load at least 10x `lists` vectors before building.

**Pinecone index cold start:** A freshly created index in serverless mode may take 10–30 seconds before it can serve queries while the underlying infrastructure provisions. For latency-sensitive paths, pre-warm by sending a dummy query after index creation.

**Qdrant HNSW build latency:** Large collections (> 10M vectors) can take minutes to hours to build the in-memory graph. Qdrant performs background indexing — you can write and query during the build, but recall will be degraded until the graph is complete. Monitor via the collection info endpoint:

```python
info = client.get_collection("docs")
print(info.status)           # "green" once indexing is complete
print(info.indexed_vectors_count)
```

**Chroma:** No meaningful cold start in embedded mode. If using Chroma in client-server mode, the server process itself may take a few seconds to load the collection into memory on first access.

---

## Distance Metrics — Quick Reference

| Metric | Operator (pgvector) | Use when |
| --- | --- | --- |
| Cosine | `<=>` | Text/image embeddings; magnitude doesn't matter |
| L2 (Euclidean) | `<->` | Embeddings already normalized; spatial data |
| Inner product | `<#>` | Pre-normalized vectors; maximizing dot product (bi-encoder models) |
| L1 (Manhattan) | `<+>` | Sparse-ish vectors; robust to outliers |

For OpenAI `text-embedding-3-*` and most sentence-transformer models: use **cosine**. The models are trained with cosine as the objective.

If you normalize all vectors to unit length before storing, cosine and inner product are equivalent — use inner product, which avoids a square root and is faster.

---

## Common Mistakes

- Building an IVFFlat index before the table has data. The centroids will be garbage.
- Using `lists = 100` (default) on a 5M-row table. Use `sqrt(5_000_000) ≈ 2236`.
- Forgetting to set `hnsw.ef_search` before querying. The default of 40 is conservative; for RAG workloads targeting > 95% recall, try 80–100.
- Storing embeddings as `float8[]` arrays instead of the `vector` type. You lose the index and the distance operators.
- Using Pinecone namespaces as a document-level sharding mechanism. They are tenant-level isolation primitives, not per-document partitions.
- Querying Qdrant without a `limit` set. It will return all matching points.
- Running hybrid search in pgvector with the wrong query planner settings — use `EXPLAIN ANALYZE` to confirm the vector index is being used and not bypassed by the filter selectivity estimate.
