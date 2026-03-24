---
name: rag-knowledge-systems
description: RAG system design covering chunking strategies (recursive, semantic, late chunking, ColPali), vector database patterns, ingestion/retrieval pipelines, evaluation (recall@k, MRR), and the RECALL-first pipeline. Use when building or optimizing any retrieval-augmented generation system.
domain: ai-engineering
tags: [rag, chunking, embeddings, vector-search, retrieval, ingestion, evaluation, recall, precision]
triggers: RAG, chunking, vector search, retrieval augmented generation, embeddings, recall@k, ingestion, semantic search, RECALL pipeline, knowledge retrieval
---

# RAG Knowledge Systems

## 1. Chunking Strategies

### Recursive Character Chunking

Industry baseline. Splits text using separator hierarchy (`["\n\n", "\n", " ", ""]`). Use `chunk_size` of 512-1024 tokens with 10-20% overlap for continuity.

### Semantic Chunking

Breaks text by meaning. Split into sentences, embed each, create new chunk when cosine similarity between consecutive sentences drops below threshold (topic break). Better for heterogeneous documents.

### Late Chunking (Contextual Embeddings)

Solves "lost context" problem. Embed entire document using long-context model, then split resulting token embeddings into chunks and pool. Chunk for "its population" knows "its" refers to "Berlin."

### ColPali (Vision-Based RAG)

Bypasses OCR entirely. Uses vision-language model to embed page images directly. Best for PDFs with complex layouts, tables, diagrams. No text extraction pipeline needed.

### Selection Guide

| Strategy | Best For | Tradeoff |
|----------|----------|----------|
| Recursive | General text, simple docs | May split mid-concept |
| Semantic | Mixed-topic documents | Higher compute cost |
| Late chunking | Context-dependent text | Requires long-context model |
| ColPali | PDFs with layouts/tables | GPU-intensive, newer approach |

## 2. Ingestion Pipeline

### Steps

1. **Document loading**: Parse source format (PDF, HTML, MD, DOCX). Extract text and metadata.
2. **Preprocessing**: Clean whitespace, normalize Unicode, remove boilerplate headers/footers.
3. **Chunking**: Apply chosen strategy. Add metadata (source, page, section heading).
4. **Embedding**: Generate vector embeddings. Batch for throughput. Cache embeddings for unchanged documents.
5. **Indexing**: Upsert to vector store with metadata. Use namespaces or metadata filters for multi-tenancy.

### RECALL-First Pipeline

Every turn: call `query_knowledge` with 2-5 keywords first. If "No relevant information found," proceed with broader search. Before building multi-tool plans, call `get_relevant_tools`. For documentation, use `get_doc_outline` then `get_section` for targeted retrieval.

### Token Conservation

- Avoid redundant `read_file` when RAG chunk is sufficient.
- Truncate broad search with strict limits.
- Use `project_packer` for high-level structure instead of multiple `ls` calls.
- Fetch complete semantic units (functions, classes) over fixed-size blocks.

## 3. Retrieval Patterns

### Vector Search

- Cosine similarity or dot product for normalized embeddings. Top-K retrieval with score threshold filtering.
- Tune `top_k` (start with 5-10) and score threshold (0.3 broad, 0.5 balanced, 0.7 high precision).

### Hybrid Search

- Combine dense (vector) and sparse (BM25/keyword) retrieval. Dense captures semantic meaning; sparse captures exact terms, acronyms, rare words.
- Use reciprocal rank fusion (RRF) or learned weights to merge results.

### Reranking

- Two-stage retrieval: broad recall (top-50 from vector search), then rerank with cross-encoder model (top-5 final).
- Cross-encoders (BAAI/bge-reranker, Cohere rerank) are more accurate but slower than bi-encoders.
- Add ~100-300ms latency per reranking step. Use for high-stakes queries.

### Multi-Query Retrieval

- Generate multiple reformulations of the query. Retrieve for each, deduplicate and merge results. Improves recall for ambiguous queries.

### Contextual Compression

- After retrieval, extract only the relevant sentences from each chunk. Reduces noise in LLM context. Use an LLM or extractive model for compression.

## 4. Evaluation

### Metrics

| Metric | What It Measures | Target |
|--------|-----------------|--------|
| **Recall@K** | % of relevant docs in top-K results | >0.8 |
| **Precision@K** | % of top-K results that are relevant | >0.6 |
| **MRR** | Reciprocal rank of first relevant result | >0.7 |
| **NDCG** | Quality of ranking considering position | >0.7 |
| **Faithfulness** | Does answer use retrieved context? | >0.9 |
| **Relevance** | Does answer address the question? | >0.9 |

### Building a Golden Set

- Curate 50-100 question-answer-context triples from real usage. Include edge cases: ambiguous queries, multi-hop reasoning, out-of-scope questions.
- Annotate which chunks should be retrieved for each question.
- Run evaluation after every chunking, embedding, or retrieval change.

### RAG Triad (RAGAS)

1. **Context relevance**: Are retrieved chunks relevant to the question?
2. **Faithfulness**: Is the answer grounded in retrieved context?
3. **Answer relevance**: Does the answer address the question?

### Common Failure Modes

- **Low recall**: Chunks too small, embeddings miss semantic meaning, query-document vocabulary mismatch.
- **Low precision**: Chunks too large (contain noise), metadata filters too broad.
- **Hallucination**: LLM ignores retrieved context, context is insufficient, no explicit grounding instruction.

## 5. Zero-Waste Context Philosophy

### Context Stuffing vs Precision Retrieval

- **Context stuffing**: Load every relevant file. High token usage, slower reasoning, lower quality ("lost in the noise").
- **Precision retrieval**: Fetch only exact bytes needed. 99% token reduction. Risk: "Context Blindness" (missing peripheral dependencies).

### When to Fetch More Context

- Highly coupled code where dependencies span multiple files.
- Bug investigation where root cause may be in caller, not callee.
- Architecture questions where broad understanding is needed.

### When to Minimize

- Focused implementation tasks with clear scope.
- Configuration changes with well-defined parameters.
- Code reviews of isolated functions.

## Checklist

- [ ] Chunking strategy chosen and tested with representative documents
- [ ] Overlap configured to prevent context loss at boundaries
- [ ] Embedding model selected with appropriate dimension/quality tradeoff
- [ ] Top-K and score threshold tuned via evaluation
- [ ] Hybrid search evaluated (dense + sparse)
- [ ] Reranking evaluated for high-stakes use cases
- [ ] Golden set curated with 50+ annotated queries
- [ ] Recall@K and precision@K measured and tracked
- [ ] Faithfulness and answer relevance evaluated
- [ ] RECALL-first pipeline enforced in agent workflows
