# SupersonicRAG

**A portable, high-performance RAG runtime for local, cloud, private-cloud, and air-gapped deployments.**

SupersonicRAG is designed around one principle: the same retrieval engine should run on Windows, macOS, Linux, Docker, VPS, AWS, GCP, Kubernetes, and a Vercel-hosted frontend without changing the RAG semantics.

## Primary architecture

```text
                         ┌──────────────────────────────┐
                         │ Web / SDK / CLI / MCP clients│
                         └──────────────┬───────────────┘
                                        │ HTTPS / SSE
                         ┌──────────────▼───────────────┐
                         │        Rust API Gateway       │
                         │ auth • tenant • RBAC • rate  │
                         │ limits • request validation   │
                         └───────┬───────────────┬──────┘
                                 │               │
                   ┌─────────────▼───┐   ┌──────▼────────────┐
                   │ RAG Query Engine │   │ Ingestion / Jobs  │
                   ├─────────────────┤   ├───────────────────┤
                   │ query rewrite   │   │ connectors        │
                   │ ACL filtering   │   │ parsing / OCR     │
                   │ dense retrieval │   │ chunking          │
                   │ BM25 / sparse   │   │ embeddings        │
                   │ RRF fusion      │   │ indexing          │
                   │ reranking       │   └───────────────────┘
                   │ context packing │              │
                   │ citations       │              ▼
                   │ grounded answer │        Queue / Workers
                   └───────┬─────────┘
                           │
          ┌────────────────┼─────────────────┐
          ▼                ▼                 ▼
     PostgreSQL          Qdrant       Object Storage
     control plane    dense+sparse     S3/GCS/MinIO
     + RLS            + payload ACL    raw/versioned data
          │                │                 │
          └────────────────┼─────────────────┘
                           ▼
                    Evaluation Engine
          MRR • MAP • NDCG • Precision • Recall
          F1 • Accuracy • Hit Rate • Faithfulness
          Citation Accuracy • latency • cost
```

## Repository structure

```text
SupersonicRAG/
├── apps/
│   ├── api/                 # Rust/Axum API
│   ├── worker/              # idempotent background jobs
│   └── admin/               # React/Vite control plane UI
├── crates/
│   ├── rag-core/            # end-to-end orchestration
│   ├── retrieval/           # BM25, dense, sparse, RRF, reranking
│   ├── evaluation/          # reproducible retrieval/answer metrics
│   ├── tenancy/             # tenant/project isolation
│   ├── security/            # auth, ACL, audit, policy contracts
│   └── ingestion/           # connector/parser/chunker contracts
├── config/                  # environment-safe defaults
├── deploy/                  # Docker, AWS, GCP, Kubernetes, Vercel
├── docs/                    # architecture and operational docs
├── evals/                   # datasets and relevance judgments
├── migrations/              # PostgreSQL schema/RLS migrations
└── docker-compose.yml       # portable local stack
```

## Retrieval pipeline

```text
query
 → authentication + tenant resolution
 → authorization / document ACL filter
 → optional query rewrite
 → dense + BM25/sparse retrieval in parallel
 → Reciprocal Rank Fusion (RRF)
 → deduplication + parent expansion
 → optional cross-encoder reranking
 → context compression / token budget
 → grounded generation
 → citation validation
 → streamed answer
```

RRF combines ranked lists rather than assuming BM25 and vector scores are directly comparable. The RRF constant is runtime-configurable and should be tuned against an evaluation set rather than hard-coded as a universal optimum.

## Evaluation suite

SupersonicRAG separates **retrieval metrics** from **answer-generation metrics** and computes metrics from explicit labeled datasets.

| Metric | Purpose |
|---|---|
| MRR | Position of the first relevant result |
| MAP@K | Ranking quality across multiple relevant results |
| NDCG@K | Graded relevance and ranking quality |
| Precision@K | Relevant results among retrieved results |
| Recall@K | Relevant results successfully retrieved |
| F1@K | Harmonic mean of precision and recall |
| Hit Rate@K | Whether at least one relevant result was retrieved |
| Accuracy | Labeled answer/classification correctness |
| Faithfulness | Whether generated claims are supported by evidence |
| Answer Relevance | Whether the answer addresses the question |
| Citation Accuracy | Whether citations support the claims they reference |
| Citation Completeness | Whether important claims have supporting citations |
| Latency | p50/p95/p99 pipeline timing |
| Throughput | Requests/jobs processed per unit time |
| Cost | Model/token/infrastructure cost where available |

### Metric definitions

For binary relevance at cutoff K:

```text
Precision@K = relevant_retrieved / K
Recall@K    = relevant_retrieved / total_relevant
F1@K        = 2 * Precision@K * Recall@K / (Precision@K + Recall@K)
MRR         = mean(1 / rank_of_first_relevant)
```

NDCG and MAP must use the complete relevance judgments available to the evaluator. Do not compare scores across datasets with materially different relevance definitions without documenting the difference.

## Admin control plane

The admin dashboard is intended to expose **safe, versioned runtime configuration**, not secrets. Controls include:

- dense top-K
- BM25 top-K
- sparse top-K
- RRF K
- reranker enable/disable and top-K
- minimum score thresholds
- chunk size and overlap
- parent/child retrieval
- context token budget
- query rewriting
- semantic/retrieval caching
- citation and grounding enforcement
- evaluation cutoff K values
- embedding / reranker / LLM provider and model
- retrieval strategy selection

Every configuration change should be versioned, scoped to tenant/project, validated against safe bounds, and audited. API keys, OAuth client secrets, database passwords and provider credentials must never be rendered as editable plaintext dashboard fields.

## Multitenancy and security

Tenant context is derived from authenticated credentials and propagated through every data-plane call. A client must never be allowed to select an arbitrary tenant merely by sending `tenant_id` in a request body.

Isolation ladder:

```text
shared Postgres + RLS
        ↓
shared Qdrant collection + mandatory tenant payload filter
        ↓
dedicated Qdrant shard for large tenants
        ↓
dedicated database / cluster
        ↓
dedicated VPC / deployment
```

Document ACLs are applied **before retrieval**, not after generation. Production deployments should use TLS, least-privilege identities, secret managers, audit logs, rate limits, retention/deletion policies, encrypted backups, and prompt-injection/tool-permission boundaries.

## Local-first and cross-platform

The supported baseline is containerized so native host differences do not change the architecture:

```bash
docker compose up --build
```

Target hosts:

- Windows 10/11 + Docker Desktop
- macOS Intel + Apple Silicon + Docker Desktop
- Linux x86_64/ARM64 + Docker Engine/Compose
- VPS providers with Docker
- Kubernetes

Native Rust development is optional. No cloud account is required for the local stack.

## Deployment model

- **Local/Lite:** Rust API + worker + PostgreSQL + Qdrant + MinIO + optional Valkey + local models.
- **AWS:** ECS/Fargate or EKS + RDS + S3 + SQS + Qdrant.
- **GCP:** Cloud Run + Cloud SQL + Cloud Storage + Cloud Tasks/Pub/Sub + Qdrant.
- **Vercel:** Next/React admin frontend and optional BFF/streaming layer; heavy ingestion/vector workloads remain in the portable backend.
- **Private/air-gapped:** OCI images + local object storage + local models + Kubernetes/Compose.

The application should use provider interfaces so AWS SQS, GCP Tasks, and a local queue do not leak into retrieval/business logic.

## Performance principles

1. Rust for request orchestration and concurrency.
2. Parallel lexical and vector retrieval.
3. Batch embedding and reranking.
4. Bounded worker concurrency and backpressure.
5. Streaming responses with SSE.
6. Content-addressed ingestion and incremental indexing.
7. Cache embeddings and retrieval results where safe and tenant-scoped.
8. Avoid unnecessary copies and serialization in hot paths.
9. Profile before introducing C/C++; use optimized native libraries/ONNX Runtime where inference is the bottleneck.
10. Measure p50/p95/p99 latency for every retrieval stage.

## Versioning and reproducibility

Every answer/evaluation should be traceable to:

```text
answer_id
 → tenant/project
 → configuration version
 → query/retrieval configuration
 → document versions/chunk IDs
 → embedding model/version
 → reranker version
 → prompt version
 → LLM model/version
```

This makes regression testing and A/B comparison possible.

## Development priorities

1. Make the Rust workspace compile on Windows/macOS/Linux.
2. Implement configuration + validation + persistence.
3. Implement PostgreSQL schema and RLS.
4. Implement Qdrant dense/sparse retrieval and mandatory tenant filters.
5. Implement BM25 + dense parallel retrieval and RRF.
6. Implement ingestion with content hashes and resumable jobs.
7. Implement evaluation datasets and all ranking metrics.
8. Implement reranking, citations, grounding, and retrieval tracing.
9. Complete admin controls and tenant-scoped audit history.
10. Add Docker, AWS, GCP, Kubernetes and Vercel deployment automation.

## License

Apache-2.0.
