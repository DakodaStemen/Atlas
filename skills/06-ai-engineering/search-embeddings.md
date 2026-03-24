---
name: search-embeddings
description: Search and embedding patterns covering embedding strategies (model selection, dimensionality, fine-tuning, quantization, multi-vector), hybrid search (dense+sparse fusion, BM25, reciprocal rank fusion), and reranking (cross-encoders, ColBERT, score calibration). Use when building or optimizing search and retrieval systems.
domain: ai-engineering
tags: [embeddings, hybrid-search, reranking, cross-encoder, bm25, dense-retrieval, sparse-retrieval, vector-search]
triggers: embeddings, embedding model, hybrid search, reranking, cross-encoder, BM25, dense retrieval, sparse retrieval, reciprocal rank fusion
---


## Purpose

Production guidance for building, selecting, and operating text embedding systems. Covers the full stack: picking a model, indexing, retrieval, hybrid search, and throughput optimization.


## 2. Dimensionality Tradeoffs

Higher dimensions encode more semantic nuance but cost more in storage, memory, and ANN index construction time.

Practical tiers:

- **128–256 dims**: Real-time, high-throughput systems (e.g., typeahead, recommendation at scale). Use only with MRL-trained models that gracefully degrade.
- **512 dims**: Solid production default for most RAG pipelines. Hits the quality/cost sweet spot.
- **768–1024 dims**: General-purpose retrieval with moderate scale (<100M vectors).
- **1536–3072 dims**: Long-context, multilingual, or multimodal tasks where maximum recall matters. Index build time and memory grow roughly linearly.

For Matryoshka-trained models, do not just truncate: normalize after truncation (L2 norm to unit length) so cosine similarity remains valid.


## 4. Distance Metrics: Cosine vs Dot Product vs Euclidean

| Metric | Formula | When to use |
| --- | --- | --- |
| Cosine similarity | `dot(a,b) / ( | a | * | b | )` | When vectors are NOT normalized; captures directional similarity independent of magnitude |
| Dot product | `dot(a,b)` | When vectors ARE normalized (equivalent to cosine); faster to compute, preferred in ANN indexes |
| Euclidean (L2) | `sqrt(sum((a-b)^2))` | Fine for normalized vectors but less common; don't mix with unnormalized |

In practice: normalize your vectors and use dot product (inner product) as the similarity function. This is what all major vector databases (Pinecone, Weaviate, Qdrant, pgvector) optimize for internally.

Never mix metrics between the query encoder and the document encoder. If your model was trained with cosine similarity, using dot product on unnormalized vectors will return garbage rankings.


## 6. Late Chunking

Traditional RAG chunks documents before embedding, losing cross-chunk context. Late chunking reverses this: embed the **entire document** through the transformer to get token-level vectors, then apply mean pooling per chunk boundary. Each chunk embedding is conditioned on its full document context.

### How it works

1. Pass the full document through the encoder (requires a long-context model: jina-embeddings-v3, BGE-M3, or similar with 8K+ token window).
2. Collect token-level hidden states.
3. Apply mean pooling within each chunk's token span — not across the entire document.
4. Index the resulting chunk vectors.

**When to use**: documents where later passages reference earlier ones (e.g., legal contracts, technical specs, narratives). Benchmark shows 5–15% retrieval improvement on multi-hop queries over naive chunking.

**Limitations**: requires a long-context embedding model; can't be done with API models that return only a single pooled vector.

```python
# Pseudocode for late chunking with token-level access
token_embeddings = model.encode_tokens(full_document)  # shape: [n_tokens, dim]
chunk_embeddings = []
for start, end in chunk_spans:
    chunk_vec = token_embeddings[start:end].mean(axis=0)
    chunk_embeddings.append(normalize(chunk_vec))
```


## 8. Reranking

Rerankers (cross-encoders) read the query and document together, producing a relevance score without precomputed embeddings. They're slower but more accurate than bi-encoders.

Typical pipeline:

1. Hybrid retrieval → top-100 candidates (fast)
2. Cross-encoder reranker → top-10 (slow, accurate)

Good open-weight rerankers: `cross-encoder/ms-marco-MiniLM-L-6-v2` (fast), `BAAI/bge-reranker-v2-m3` (multilingual, best quality).

Cohere Rerank API and Voyage Rerank are strong managed options.

Injecting BM25 scores as features into neural rerankers improves MRR@10 by ~7% (per published benchmarks) and adds explainability in regulated domains.


## 10. Caching

Embedding calls are expensive and deterministic — the same string always produces the same vector. Cache aggressively.

### Query-level caching

```python
import hashlib, json
from functools import lru_cache

def cache_key(text: str, model: str) -> str:
    return hashlib.sha256(f"{model}:{text}".encode()).hexdigest()

# Redis example
def get_or_embed(text: str, model_fn, redis_client, model_name: str) -> list[float]:
    key = cache_key(text, model_name)
    cached = redis_client.get(key)
    if cached:
        return json.loads(cached)
    vec = model_fn(text)
    redis_client.setex(key, 86400, json.dumps(vec))  # TTL: 1 day
    return vec
```

### Document-level caching

For offline indexing, skip re-embedding documents that haven't changed. Hash the document content; if the hash matches a stored record, reuse the stored vector. Mandatory during model migrations to bound re-embedding cost.

### Caching boundaries

Do not cache mid-transformation (e.g., after chunking but before embedding). Cache the final normalized vector that goes into the index, not intermediate representations.


## 12. ANN Index Selection

| Index | Best for | Notes |
| --- | --- | --- |
| HNSW | 1M–100M vectors, high recall | Default for most databases; tune `ef_construction` and `m` at build; `ef_search` at query |
| IVF+PQ | >100M vectors, memory-constrained | Lower recall than HNSW; use `nprobe` to trade recall for speed |
| Flat (exact) | <500K vectors or evaluation | Exact neighbors; no recall/speed tradeoff but O(n) |
| ScaNN | Google's ANN library; production at massive scale | Best throughput/recall curve at billion scale |

For most RAG pipelines (1M–10M documents): HNSW with `m=16`, `ef_construction=200`, `ef_search=100`. These defaults give >0.95 recall@10 with sub-10ms latency on commodity GPU.


## 14. Cost Estimation Reference

For 1M documents at ~300 tokens average:

| Option | Embedding cost | Storage (1024 dims, float32) | Monthly GPU |
| --- | --- | --- | --- |
| OpenAI text-embedding-3-small | ~$6 | ~4GB | $0 (API) |
| OpenAI text-embedding-3-large | ~$39 | ~12GB | $0 (API) |
| Cohere Embed v4 | ~$30 | ~4GB | $0 (API) |
| Self-hosted Qwen3-8B | $0 | ~4GB | ~$100 |
| Self-hosted all-MiniLM-L6-v2 | $0 | ~1.5GB | ~$20 (CPU) |

Re-embedding cost is a one-time hit; weight it against ongoing query cost when choosing between API and self-hosted.

---


# Hybrid Search & Reranking for RAG

## When to Use

**Pure vector search wins** when:

- Queries are paraphrases or conceptually similar to indexed content (e.g., "car" vs "automobile")
- The corpus is in-domain and the embedding model was trained on similar data
- Users ask natural language questions with no exact-match expectation

**Keyword-only (BM25) wins** when:

- Queries contain product codes, serial numbers, ticker symbols, or rare proper nouns
- The domain is highly specialized (legal citations, medical codes, internal jargon) and embeddings are undertrained
- Retrieval must be reproducible and explainable (BM25 is deterministic)

**Hybrid search wins** (most production RAG cases) when:

- Queries mix semantic intent with specific terms (e.g., "how does GAN training work in PyTorch 2.0")
- Your embedding model is pretrained on general data and your corpus is domain-specific — BM25 adapts immediately while dense retrieval generalizes
- You see the classic RAG failure modes: missing abbreviations, failing on named entities, or drifting on long-tail queries

### Diagnosis checklist for retrieval quality issues

- Exact-term misses → add BM25 / sparse retrieval
- Semantic drift on paraphrases → strengthen dense embeddings or add reranker
- Both failing → switch to three-stage: BM25 + dense + rerank
- Consistently wrong top-1 but correct in top-10 → the retriever has recall; add a reranker


## Semantic Search

Dense retrieval encodes queries and documents into continuous vector spaces where geometric proximity implies semantic similarity.

### Embedding pipeline

```python
from sentence_transformers import SentenceTransformer
import numpy as np

model = SentenceTransformer("BAAI/bge-large-en-v1.5")  # strong general-purpose

doc_embeddings = model.encode(
    [doc.page_content for doc in documents],
    batch_size=64,
    normalize_embeddings=True,  # enables dot product == cosine similarity
    show_progress_bar=True
)

query_embedding = model.encode("your query here", normalize_embeddings=True)
scores = np.dot(doc_embeddings, query_embedding)
```

#### Vector index trade-offs

| Index Type | Build Time | Query Speed | Accuracy | Memory |
| ------------ | ----------- | ------------- | ---------- | -------- |
| Flat (exact) | Fast | Slow at scale | 100% recall | High |
| HNSW | Slow | Fast | 95–99% recall | High |
| IVF-PQ | Medium | Fast | 90–97% recall | Low |
| Binary | Fast | Very fast | 85–95% recall | Very low |

HNSW is the standard for production RAG. For ColBERT-style multi-vector docs, disable HNSW graph creation (`m=0` in Qdrant) and use brute-force MaxSim instead — the candidate set is small enough that exact search dominates.

#### Model selection heuristics

- General RAG: `BAAI/bge-large-en-v1.5` or `text-embedding-3-large`
- Multilingual: `intfloat/multilingual-e5-large`
- Code: `Salesforce/SFR-Embedding-Code`
- If you must fine-tune: use contrastive loss with hard negatives mined from BM25


## Reranking Architecture

The two-stage pipeline is the production standard:

```text
Stage 1 — Recall Phase (fast, cheap):
  BM25 retrieval (top-25) + Dense retrieval (top-25)
         → RRF fusion → top-50 candidates

Stage 2 — Precision Phase (slow, expensive):
  Cross-encoder or ColBERT reranker over top-50 candidates
         → final top-5 to top-10 passed to LLM context
```

The recall phase uses bi-encoders (query and document encoded independently), which allows pre-computing all document embeddings offline. The precision phase uses cross-encoders (query and document encoded together), which is far more accurate but cannot scale to the full corpus.

### Latency budget rule of thumb

- Bi-encoder retrieval: 10–50 ms for 1M docs with HNSW
- Cross-encoder reranking: 50–200 ms for 50 candidates on GPU, 200–500 ms on CPU
- ColBERT reranking: 20–80 ms for 50 candidates (more efficient than cross-encoder at same quality tier)
- Total P95 budget for production: keep retrieval + reranking under 500 ms; if over, reduce candidate set or use a smaller reranker model


## Late Interaction: ColBERT

ColBERT (Contextualized Late Interaction over BERT) encodes query and document independently into sequences of token-level embeddings, then scores relevance via the **MaxSim** operator:

```sql
score(Q, D) = Σ  max  sim(qᵢ, dⱼ)
              i   j∈D
```

For each query token embedding `qᵢ`, find its maximum cosine similarity to any document token embedding `dⱼ`. Sum over all query tokens. This captures fine-grained token-level interactions without the quadratic cross-attention cost of cross-encoders.

### Why it matters for RAG

- Precompute all document token embeddings offline (like bi-encoders)
- At query time, only the query tokens are encoded, then MaxSim runs over precomputed doc embeddings
- 100× more efficient than cross-encoders at comparable quality tiers (per Infinity benchmarks on MLDR)
- Works well on long documents: split into paragraphs, encode each, take max paragraph score

#### RAGatouille (simplest ColBERT integration)

```python
from ragatouille import RAGPretrainedModel

RAG = RAGPretrainedModel.from_pretrained("colbert-ir/colbertv2.0")

# Index documents (precomputes token embeddings)
RAG.index(
    collection=[doc.page_content for doc in documents],
    document_ids=[doc.metadata["id"] for doc in documents],
    index_name="my_rag_index",
    max_document_length=256,
    split_documents=True
)

# Search
results = RAG.search(query="your question here", k=5)
```

#### Qdrant multi-vector ColBERT (production)

```python
from qdrant_client.models import VectorParams, Distance, HnswConfigDiff

# Disable HNSW for multi-vector collection (brute-force MaxSim)
client.create_collection(
    collection_name="colbert_docs",
    vectors_config={
        "colbert": VectorParams(
            size=128,  # ColBERT token embedding dim
            distance=Distance.COSINE,
            multivector_config=models.MultiVectorConfig(
                comparator=models.MultiVectorComparator.MAX_SIM
            ),
            hnsw_config=HnswConfigDiff(m=0),  # no graph — exact search
        )
    }
)
```


## Implementation: Elasticsearch / OpenSearch

Elasticsearch 8.x+ supports `knn` combined with BM25 in a single query via the `bool` + `knn` pattern:

```json
POST /my-index/_search
{
  "knn": {
    "field": "embedding",
    "query_vector": [0.1, 0.2, ...],
    "k": 50,
    "num_candidates": 100,
    "boost": 0.5
  },
  "query": {
    "bool": {
      "should": [
        {
          "multi_match": {
            "query": "transformer attention mechanism",
            "fields": ["title^2", "content"],
            "type": "best_fields",
            "boost": 0.5
          }
        }
      ]
    }
  },
  "size": 20
}
```

### Python (elasticsearch-py)

```python
from elasticsearch import Elasticsearch
from sentence_transformers import SentenceTransformer

es = Elasticsearch("http://localhost:9200")
model = SentenceTransformer("BAAI/bge-large-en-v1.5")

query_vec = model.encode(query, normalize_embeddings=True).tolist()

resp = es.search(
    index="my-index",
    knn={
        "field": "embedding",
        "query_vector": query_vec,
        "k": 50,
        "num_candidates": 100,
        "boost": 0.5,
    },
    query={
        "multi_match": {
            "query": query,
            "fields": ["title^2", "content"],
            "boost": 0.5,
        }
    },
    size=20,
)

hits = resp["hits"]["hits"]
```

**OpenSearch** uses the same pattern with `"knn": true` field mapping and the `knn_vector` type. The hybrid scoring is handled by summing BM25 and kNN scores via boosting — not RRF natively (use a custom post-processing step or the OpenSearch Neural Plugin for RRF).


## Evaluation Metrics

Always measure retrieval independently from generation. A good generator can't fix bad retrieval.

### NDCG@K (Normalized Discounted Cumulative Gain)

Accounts for both relevance and rank position. Graded relevance (0, 1, 2, 3) preferred over binary.

```text
DCG@K  = Σ (rel_i) / log₂(i + 1)    for i in 1..K
NDCG@K = DCG@K / IDCG@K             (IDCG = ideal DCG)
```

**MRR (Mean Reciprocal Rank):** Average of `1/rank` of the first relevant document. Use this when you care primarily about getting one correct result in the top position.

**Recall@K:** Fraction of all relevant documents found in the top-K. Use this to measure how well your retriever covers the answer space before reranking.

**Precision@K:** Fraction of top-K results that are relevant. Use this to measure reranker quality.

```python
from sklearn.metrics import ndcg_score
import numpy as np

def recall_at_k(relevant_ids, retrieved_ids, k):
    retrieved_k = set(retrieved_ids[:k])
    return len(set(relevant_ids) & retrieved_k) / len(relevant_ids)

def mrr(relevant_ids, retrieved_ids):
    for rank, doc_id in enumerate(retrieved_ids, start=1):
        if doc_id in relevant_ids:
            return 1.0 / rank
    return 0.0

# NDCG with sklearn
y_true = np.array([[3, 2, 1, 0, 0]])  # graded relevance
y_score = np.array([[0.9, 0.8, 0.3, 0.2, 0.1]])  # model scores
ndcg = ndcg_score(y_true, y_score, k=5)
```

**Benchmark datasets:** MS MARCO, BEIR (zero-shot generalization), MLDR (multilingual), and your own annotated sample (minimum 50–100 query-answer pairs from real user queries).


## References

- Cormack, G. et al. (2009). "Reciprocal Rank Fusion outperforms Condorcet and individual Rank Learning Methods." *SIGIR 2009*. — original RRF paper
- Khattab, O. & Zaharia, M. (2020). "ColBERT: Efficient and Effective Passage Search via Contextualized Late Interaction over BERT." *SIGIR 2020*.
- Robertson, S. & Zaragoza, H. (2009). "The Probabilistic Relevance Framework: BM25 and Beyond." *Foundations and Trends in IR*.
- Pinecone. "Hybrid Search Intro." <https://www.pinecone.io/learn/hybrid-search-intro/>
- Qdrant. "Hybrid Search Revamped — Building with Qdrant's Query API." <https://qdrant.tech/articles/hybrid-search/>
- Superlinked. "Optimizing RAG with Hybrid Search & Reranking." <https://superlinked.com/vectorhub/articles/optimizing-rag-with-hybrid-search-reranking>
- Infiniflow. "Dense vector + Sparse vector + Full text search + Tensor reranker = Best retrieval for RAG?" <https://infiniflow.org/blog/best-hybrid-search-solution>
- RAGatouille. <https://github.com/bclavie/RAGatouille> — ColBERT wrapper for RAG
- BEIR Benchmark. <https://github.com/beir-cellar/beir> — heterogeneous retrieval evaluation
