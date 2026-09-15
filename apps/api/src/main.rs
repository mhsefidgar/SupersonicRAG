use axum::{extract::State, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::{Arc, RwLock}};

#[derive(Serialize)]
struct Health { status: &'static str, service: &'static str }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RagConfig {
    dense_top_k: usize,
    bm25_top_k: usize,
    sparse_top_k: usize,
    rrf_k: usize,
    reranker_top_k: usize,
    score_threshold: f64,
    context_max_tokens: usize,
    chunk_size: usize,
    chunk_overlap: usize,
}

fn defaults() -> RagConfig { RagConfig { dense_top_k:50, bm25_top_k:50, sparse_top_k:50, rrf_k:60, reranker_top_k:8, score_threshold:0.0, context_max_tokens:6000, chunk_size:600, chunk_overlap:100 } }

async fn health() -> Json<Health> { Json(Health { status:"ok", service:"swiftrag-api" }) }
async fn config(State(state): State<Arc<RwLock<RagConfig>>>) -> Json<RagConfig> { Json(state.read().unwrap().clone()) }
async fn update_config(State(state): State<Arc<RwLock<RagConfig>>>, Json(next): Json<RagConfig>) -> Json<RagConfig> {
    // Production middleware must authenticate the caller and scope this write to tenant/project context.
    let mut cfg = state.write().unwrap();
    *cfg = next;
    Json(cfg.clone())
}

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(defaults()));
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/config", get(config).post(update_config))
        .with_state(state);
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
