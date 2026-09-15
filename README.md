# SwiftRAG

Ultra-light, portable, multi-tenant RAG runtime with hybrid retrieval, RRF, reranking, evaluation, and cloud/local deployment.

## Architecture

```text
Clients / SDKs
      |
      v
API Gateway (Rust/Axum)
      |
      +--> Auth / RBAC / tenant resolution / rate limits
      |
      +--> RAG Query Engine
      |      +--> query rewrite
      |      +--> dense retrieval
      |      +--> BM25 / sparse retrieval
      |      +--> RRF fusion
      |      +--> reranking
      |      +--> context compression
      |      +--> grounded generation + citations
      |
      +--> Ingestion API -> queue -> workers -> parser/OCR -> chunker -> embeddings -> indexes
      |
      +--> Evaluation Engine
             +--> MRR, NDCG@K, MAP@K, Precision@K, Recall@K, Hit Rate, F1,
                 Accuracy, Faithfulness, Answer Relevance, Citation Accuracy,
                 latency and cost

Control plane: PostgreSQL
Search plane: Qdrant (dense + sparse + payload filters)
Object plane: S3/GCS-compatible object storage
Queue: NATS JetStream / SQS / Cloud Tasks
Cache: Valkey/Redis (optional)
Frontend: React + Vite admin console

Deployment targets: Docker/Compose, Windows, macOS, Linux, AWS, GCP, Kubernetes, VPS,
Vercel frontend + external RAG API.
```

## Repository layout

```text
SwiftRAG/
├── apps/
│   ├── api/              # Rust HTTP API
│   ├── worker/           # async ingestion/evaluation workers
│   └── admin/            # React/Vite admin console
├── crates/
│   ├── rag-core/         # query orchestration
│   ├── retrieval/        # dense/BM25/sparse/RRF/reranking
│   ├── evaluation/       # IR + answer quality metrics
│   ├── tenancy/          # tenant isolation + policy evaluation
│   ├── security/         # auth, ACL, secrets, audit primitives
│   └── ingestion/        # parsing/chunking/indexing contracts
├── config/
├── deploy/
│   ├── docker/
│   ├── aws/
│   ├── gcp/
│   ├── kubernetes/
│   └── vercel/
├── docs/
├── evals/
├── migrations/
└── docker-compose.yml
```

## Local quick start

Requirements: Docker Desktop or Docker Engine + Compose. This avoids OS-specific native dependencies and runs on Windows, macOS and Linux.

```bash
docker compose up --build
```

Then open:

- Admin: http://localhost:3000
- API: http://localhost:8080/health
- Qdrant: http://localhost:6333/dashboard

For native Rust development:

```bash
cargo run -p swiftrag-api
```

## Configuration

All retrieval and generation parameters are runtime configuration, not compile-time constants. The admin console exposes safe controls for:

- chunk size / overlap
- dense top-K
- BM25 top-K
- sparse top-K
- RRF K
- reranker enablement and top-K
- score thresholds
- parent-child expansion
- context budget
- query rewrite
- semantic cache
- citation/grounding enforcement
- evaluation K values
- model/provider selection

Changes are versioned per tenant/project and audited. Secrets are never editable as plain values in the dashboard.

## Evaluation metrics

SwiftRAG implements a dedicated evaluation contract for:

- **MRR** — Mean Reciprocal Rank
- **NDCG@K** — Normalized Discounted Cumulative Gain
- **MAP@K** — Mean Average Precision
- **Precision@K**
- **Recall@K**
- **F1@K**
- **Hit Rate@K / Success@K**
- **Accuracy** for labeled answer tasks
- **Faithfulness / groundedness**
- **Answer relevance**
- **Citation accuracy / completeness**
- latency, throughput and estimated cost

IR metrics operate on explicit relevance judgments; answer metrics use separate labeled/evaluator datasets. This prevents conflating retrieval correctness with generation correctness.

## Security model

Every data-plane operation is scoped by authenticated tenant/project context. Production deployments should use Postgres RLS plus vector payload filters, document ACL filtering before retrieval, least-privilege service accounts, TLS, KMS/Secret Manager, audit logs, rate limits, prompt-injection defenses, and explicit data-retention/deletion workflows.

## Design principles

1. Rust for latency-sensitive orchestration; Python only for optional ML/OCR adapters.
2. Provider-neutral interfaces for LLMs, embeddings, rerankers, storage and queues.
3. Parallel dense + lexical retrieval followed by RRF and optional reranking.
4. Tenant isolation is a first-class invariant.
5. Everything important is observable and versioned.
6. Local mode works without a cloud account.
7. Cloud deployment reuses the same OCI images.
8. Expensive stages are optional and can be enabled from the admin UI.
