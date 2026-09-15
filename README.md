# SupersonicRAG

**A portable, high-performance RAG runtime for local, cloud, private-cloud, and air-gapped deployments.**

SupersonicRAG uses the same retrieval semantics across Windows, macOS, Linux, Docker, VPS, AWS, GCP, Kubernetes, and a separately hosted frontend.

## Local-first testing

Docker Compose is the supported developer path. It avoids OS-specific native dependencies.

Linux/macOS:

```bash
./scripts/start-local.sh
```

Windows PowerShell:

```powershell
./scripts/start-local.ps1
```

Or directly:

```bash
docker compose up --build
```

Open:

- Admin console: `http://localhost:3000`
- API health: `http://localhost:8080/health`
- Qdrant dashboard: `http://localhost:6333/dashboard`

Stop the stack with `docker compose down`.

## Knowledge Base

The admin console includes a **Knowledge Base** uploader. Users can drag and drop or browse for PDF, DOCX, TXT, Markdown, CSV, JSON, and HTML files. Selected files remain in the browser upload queue until the user clicks **Upload & index**.

The intended ingestion path is:

```text
Upload -> validate -> hash/deduplicate -> object storage -> parse/OCR
       -> chunk -> metadata/ACL -> embeddings + BM25/sparse index -> ready
```

Production ingestion must associate every upload with the authenticated tenant/project, enforce ACLs, limit file size/type, scan untrusted content, and make indexing idempotent.

## Primary architecture

```text
Clients / SDK / CLI / MCP
          |
          v
     Rust API Gateway
          |
    +-----+----------------+
    |                      |
RAG Query Engine       Ingestion API
    |                      |
    |                 Queue / Workers
    |                      |
    |       parse/OCR -> chunk -> embed -> index
    |
    +-> auth + tenant/ACL
    +-> query rewrite
    +-> dense + BM25/sparse in parallel
    +-> RRF -> dedup -> parent expansion
    +-> reranker -> context packing
    +-> grounded generation -> citations

PostgreSQL: control plane + RLS
Qdrant: dense/sparse search + tenant payload filters
S3/GCS/MinIO: raw/versioned documents
Valkey/Redis: optional cache
Evaluation: MRR, MAP, NDCG, Precision, Recall, F1, Accuracy, Faithfulness, citations, latency, cost
```

## Admin controls

Safe runtime parameters include dense/BM25/sparse top-K, RRF K, reranker settings, score thresholds, chunk size/overlap, parent-child retrieval, context budget, query rewriting, caching, evaluation K values, and model/provider selection.

Security-critical controls such as tenant isolation, ACL enforcement, authentication requirements, and audit logging must be role-protected. Secrets must never be editable as plaintext.

## Evaluation

SupersonicRAG separates retrieval metrics from answer-generation metrics. Retrieval datasets use explicit relevance judgments for MRR, MAP@K, NDCG@K, Precision@K, Recall@K, F1 and Hit Rate. Answer datasets separately measure Accuracy, Faithfulness/Groundedness, Answer Relevance, Citation Accuracy and Citation Completeness. Record p50/p95/p99 latency, throughput, token usage and estimated cost with each experiment.

## Cross-platform design

- Windows 10/11 + Docker Desktop
- macOS Intel/Apple Silicon + Docker Desktop
- Linux x86_64/ARM64 + Docker Engine/Compose
- VPS providers with Docker
- Kubernetes

Native Rust development is optional. No cloud account is required for the local baseline.

## Repository layout

```text
SupersonicRAG/
├── apps/api/          # Rust HTTP/data-plane API
├── apps/worker/       # asynchronous ingestion/evaluation worker
├── apps/admin/        # React/Vite admin + Knowledge Base UI
├── crates/            # reusable Rust domains
├── config/            # runtime defaults
├── deploy/            # Docker/cloud deployment definitions
├── scripts/            # cross-platform local helpers
├── evals/              # evaluation datasets
├── migrations/         # PostgreSQL schema/RLS
└── docker-compose.yml  # local integration environment
```

## Performance principles

Use Rust for orchestration, parallelize independent retrieval stages, batch embeddings/reranking, apply bounded concurrency/backpressure, stream responses, use content hashes for incremental indexing, and profile before introducing C/C++.

## License

Apache-2.0.
