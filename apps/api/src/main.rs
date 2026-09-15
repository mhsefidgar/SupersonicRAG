use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use tokio::{fs, io::AsyncWriteExt};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

const MAX_REQUEST_BYTES: usize = 50 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;
const MAX_FILES_PER_REQUEST: usize = 20;
const ALLOWED_EXTENSIONS: &[&str] = &["pdf", "docx", "txt", "md", "csv", "json", "html", "htm"];

#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<RagConfig>>,
    storage_root: PathBuf,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    service: &'static str,
}

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

fn defaults() -> RagConfig {
    RagConfig {
        dense_top_k: 50,
        bm25_top_k: 50,
        sparse_top_k: 50,
        rrf_k: 60,
        reranker_top_k: 8,
        score_threshold: 0.0,
        context_max_tokens: 6000,
        chunk_size: 600,
        chunk_overlap: 100,
    }
}

#[derive(Debug, Serialize)]
struct DocumentRecord {
    id: String,
    name: String,
    size_bytes: usize,
    extension: String,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct UploadResponse {
    documents: Vec<DocumentRecord>,
}

#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        service: "supersonicrag-api",
    })
}

async fn config(State(state): State<AppState>) -> Json<RagConfig> {
    Json(state.config.read().expect("config lock poisoned").clone())
}

async fn update_config(
    State(state): State<AppState>,
    Json(next): Json<RagConfig>,
) -> Result<Json<RagConfig>, (StatusCode, Json<ApiError>)> {
    validate_config(&next)?;
    let mut cfg = state.config.write().expect("config lock poisoned");
    *cfg = next;
    Ok(Json(cfg.clone()))
}

async fn list_documents(
    State(state): State<AppState>,
) -> Result<Json<Vec<DocumentRecord>>, (StatusCode, Json<ApiError>)> {
    let mut entries = Vec::new();
    let mut dir = fs::read_dir(&state.storage_root)
        .await
        .map_err(internal_error)?;

    while let Some(entry) = dir.next_entry().await.map_err(internal_error)? {
        let path = entry.path();
        if !path.is_file() || path.file_name().and_then(|x| x.to_str()) == Some("manifest.jsonl") {
            continue;
        }
        let metadata = entry.metadata().await.map_err(internal_error)?;
        let name = path
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or("document")
            .to_owned();
        let id = name.split('-').next().unwrap_or_default().to_owned();
        let extension = extension_for(&name).unwrap_or("unknown").to_owned();
        entries.push(DocumentRecord {
            id,
            name,
            size_bytes: metadata.len() as usize,
            extension,
            status: "queued",
        });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(entries))
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, Json<ApiError>)> {
    fs::create_dir_all(&state.storage_root)
        .await
        .map_err(internal_error)?;
    let mut documents = Vec::new();
    let mut count = 0usize;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|err| bad_request(err.to_string()))?
    {
        if field.name() != Some("files") {
            continue;
        }
        count += 1;
        if count > MAX_FILES_PER_REQUEST {
            return Err(bad_request("too many files; maximum is 20"));
        }

        let original_name = field
            .file_name()
            .ok_or_else(|| bad_request("each file must have a filename"))?
            .to_owned();
        let safe_name = sanitize_filename(&original_name)?;
        let extension =
            extension_for(&safe_name).ok_or_else(|| bad_request("unsupported file type"))?;
        let mut hasher = Sha256::new();
        let mut bytes_written = 0usize;
        let temp_path = state
            .storage_root
            .join(format!(".upload-{}-{}", std::process::id(), count));
        let mut file = fs::File::create(&temp_path).await.map_err(internal_error)?;

        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|err| bad_request(err.to_string()))?
        {
            bytes_written = bytes_written.saturating_add(chunk.len());
            if bytes_written > MAX_FILE_BYTES {
                let _ = fs::remove_file(&temp_path).await;
                return Err(bad_request("file exceeds the 25 MB limit"));
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(internal_error)?;
        }
        file.flush().await.map_err(internal_error)?;
        drop(file);

        let id = hex::encode(hasher.finalize());
        let final_name = format!("{}-{}", id, safe_name);
        let final_path = state.storage_root.join(&final_name);
        if fs::try_exists(&final_path).await.map_err(internal_error)? {
            fs::remove_file(&temp_path).await.map_err(internal_error)?;
            info!(document_id = %id, filename = %safe_name, "duplicate document upload ignored");
        } else {
            fs::rename(&temp_path, &final_path)
                .await
                .map_err(internal_error)?;
            info!(
                document_id = %id,
                filename = %safe_name,
                bytes = bytes_written,
                "document stored for indexing"
            );
        }

        documents.push(DocumentRecord {
            id,
            name: safe_name,
            size_bytes: bytes_written,
            extension: extension.to_owned(),
            status: "queued",
        });
    }

    if documents.is_empty() {
        warn!("upload request contained no files field");
        return Err(bad_request("no files were uploaded"));
    }

    Ok(Json(UploadResponse { documents }))
}

fn validate_config(cfg: &RagConfig) -> Result<(), (StatusCode, Json<ApiError>)> {
    let valid = cfg.dense_top_k > 0
        && cfg.dense_top_k <= 200
        && cfg.bm25_top_k > 0
        && cfg.bm25_top_k <= 200
        && cfg.sparse_top_k > 0
        && cfg.sparse_top_k <= 200
        && cfg.rrf_k > 0
        && cfg.rrf_k <= 200
        && cfg.reranker_top_k > 0
        && cfg.reranker_top_k <= 50
        && (0.0..=1.0).contains(&cfg.score_threshold)
        && cfg.context_max_tokens >= 500
        && cfg.context_max_tokens <= 32_000
        && cfg.chunk_size >= 100
        && cfg.chunk_size <= 4_000
        && cfg.chunk_overlap <= 1_000
        && cfg.chunk_overlap < cfg.chunk_size;

    if valid {
        Ok(())
    } else {
        Err(bad_request("invalid RAG configuration values"))
    }
}

fn sanitize_filename(input: &str) -> Result<String, (StatusCode, Json<ApiError>)> {
    let name = Path::new(input)
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .trim();
    if name.is_empty() || name == "." || name == ".." || name.len() > 180 {
        return Err(bad_request("invalid filename"));
    }
    if name.chars().any(|c| c.is_control()) {
        return Err(bad_request("filename contains invalid control characters"));
    }
    Ok(name.replace('/', "_").replace('\\', "_"))
}

fn extension_for(name: &str) -> Option<&str> {
    let ext = Path::new(name).extension()?.to_str()?.to_ascii_lowercase();
    ALLOWED_EXTENSIONS.iter().find(|x| **x == ext).copied()
}

fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            error: message.into(),
        }),
    )
}

fn internal_error<E: std::fmt::Display>(error: E) -> (StatusCode, Json<ApiError>) {
    tracing::error!(error = %error, "local storage operation failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            error: "local storage operation failed".into(),
        }),
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let storage_root = std::env::var("SUPERSONICRAG_STORAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/data/knowledge"));
    fs::create_dir_all(&storage_root)
        .await
        .expect("create storage directory");

    let state = AppState {
        config: Arc::new(RwLock::new(defaults())),
        storage_root,
    };
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/config", get(config).post(update_config))
        .route("/v1/documents", get(list_documents))
        .route("/v1/documents/upload", post(upload))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("bind API port");
    info!("SupersonicRAG API listening on 0.0.0.0:8080");
    axum::serve(listener, app).await.expect("API server failed");
}
