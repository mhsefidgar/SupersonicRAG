use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::{Arc, RwLock}};

#[derive(Serialize)]
struct Health { status: &'static str, service: &'static str }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RagConfig { dense_top_k: usize, bm25_top_k: usize, sparse_top_k: usize, rrf_k: usize, reranker_top_k: usize, score_threshold: f64, context_max_tokens: usize, chunk_size: usize, chunk_overlap: usize }
fn defaults() -> RagConfig { RagConfig { dense_top_k:50,bm25_top_k:50,sparse_top_k:50,rrf_k:60,reranker_top_k:8,score_threshold:0.0,context_max_tokens:6000,chunk_size:600,chunk_overlap:100 } }
async fn health() -> Json<Health> { Json(Health { status:"ok", service:"supersonicrag-api" }) }
async fn config(State(state): State<Arc<RwLock<RagConfig>>>) -> Json<RagConfig> { Json(state.read().expect("config lock poisoned").clone()) }
async fn update_config(State(state): State<Arc<RwLock<RagConfig>>>, Json(next): Json<RagConfig>) -> Json<RagConfig> { let mut cfg=state.write().expect("config lock poisoned"); *cfg=next; Json(cfg.clone()) }
async fn upload_placeholder() -> (StatusCode, Json<serde_json::Value>) { (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({"error":"document ingestion is not enabled in this development build","next":"start the worker and configure object storage"}))) }
#[tokio::main]
async fn main() {
 let state=Arc::new(RwLock::new(defaults()));
 let app=Router::new().route("/health",get(health)).route("/v1/config",get(config).post(update_config)).route("/v1/documents/upload",post(upload_placeholder)).with_state(state);
 let addr:SocketAddr="0.0.0.0:8080".parse().expect("valid listen address");
 let listener=tokio::net::TcpListener::bind(addr).await.expect("bind API port");
 axum::serve(listener,app).await.expect("API server failed");
}
