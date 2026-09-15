use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Serialize)]
struct Health { status: &'static str, service: &'static str }

async fn health() -> Json<Health> {
    Json(Health { status: "ok", service: "swiftrag-api" })
}

async fn config() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "retrieval": {
            "dense_enabled": true,
            "dense_top_k": 50,
            "bm25_enabled": true,
            "bm25_top_k": 50,
            "sparse_enabled": false,
            "sparse_top_k": 50,
            "rrf_enabled": true,
            "rrf_k": 60,
            "reranker_enabled": true,
            "reranker_top_k": 8,
            "score_threshold": 0.0,
            "parent_child_enabled": true,
            "query_rewrite_enabled": true,
            "semantic_cache_enabled": true,
            "context_max_tokens": 6000
        },
        "evaluation": { "k_values": [1,3,5,10] }
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(health)).route("/v1/config", get(config));
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
