use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, State},
    http::{HeaderValue, Method, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::{fs, io::AsyncWriteExt};
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

const DEFAULT_PORT: u16 = 8080;
const MAX_REQUEST_BYTES: usize = 50 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;
const MAX_FILES_PER_REQUEST: usize = 20;
static UPLOAD_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct AppState {
    storage_root: PathBuf,
    config: Arc<tokio::sync::RwLock<RagConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RagConfig {
    dense_top_k: usize,
    bm25_top_k: usize,
    sparse_top_k: usize,
    rrf_k: usize,
    reranker_top_n: usize,
    score_threshold: f32,
    context_budget_tokens: usize,
    chunk_size: usize,
    chunk_overlap: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            dense_top_k: 20,
            bm25_top_k: 20,
            sparse_top_k: 20,
            rrf_k: 60,
            reranker_top_n: 8,
            score_threshold: 0.0,
            context_budget_tokens: 6000,
            chunk_size: 900,
            chunk_overlap: 120,
        }
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

#[derive(Debug, Deserialize)]
struct ConfigUpdate {
    dense_top_k: Option<usize>,
    bm25_top_k: Option<usize>,
    sparse_top_k: Option<usize>,
    rrf_k: Option<usize>,
    reranker_top_n: Option<usize>,
    score_threshold: Option<f32>,
    context_budget_tokens: Option<usize>,
    chunk_size: Option<usize>,
    chunk_overlap: Option<usize>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError {
        status: StatusCode::BAD_REQUEST,
        message: message.into(),
    }
}

fn internal_error(error: impl std::fmt::Display) -> ApiError {
    warn!(%error, "internal API error");
    ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: "internal server error".to_owned(),
    }
}

fn extension_for(name: &str) -> Option<&'static str> {
    match Path::new(name)
        .extension()?
        .to_str()?
        .to_ascii_lowercase()
        .as_str()
    {
        "pdf" => Some("pdf"),
        "docx" => Some("docx"),
        "txt" => Some("txt"),
        "md" => Some("md"),
        "csv" => Some("csv"),
        "json" => Some("json"),
        "html" => Some("html"),
        "htm" => Some("htm"),
        _ => None,
    }
}

fn sanitize_filename(name: &str) -> Result<String, ApiError> {
    let path = Path::new(name);
    let file_name = path
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or_default();
    let cleaned = file_name.trim().replace(['/', '\\', '\0'], "_");
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        return Err(bad_request("invalid filename"));
    }
    Ok(cleaned)
}

fn validate_config(config: &RagConfig) -> Result<(), ApiError> {
    if config.dense_top_k == 0 || config.dense_top_k > 200 {
        return Err(bad_request("dense_top_k must be between 1 and 200"));
    }
    if config.bm25_top_k == 0 || config.bm25_top_k > 200 {
        return Err(bad_request("bm25_top_k must be between 1 and 200"));
    }
    if config.sparse_top_k == 0 || config.sparse_top_k > 200 {
        return Err(bad_request("sparse_top_k must be between 1 and 200"));
    }
    if config.rrf_k == 0 || config.rrf_k > 200 {
        return Err(bad_request("rrf_k must be between 1 and 200"));
    }
    if config.reranker_top_n == 0 || config.reranker_top_n > 50 {
        return Err(bad_request("reranker_top_n must be between 1 and 50"));
    }
    if !(0.0..=1.0).contains(&config.score_threshold) {
        return Err(bad_request("score_threshold must be between 0 and 1"));
    }
    if !(500..=32000).contains(&config.context_budget_tokens) {
        return Err(bad_request(
            "context_budget_tokens must be between 500 and 32000",
        ));
    }
    if !(100..=4000).contains(&config.chunk_size) {
        return Err(bad_request("chunk_size must be between 100 and 4000"));
    }
    if config.chunk_overlap > 1000 || config.chunk_overlap >= config.chunk_size {
        return Err(bad_request(
            "chunk_overlap must be <= 1000 and less than chunk_size",
        ));
    }
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "supersonicrag-api",
    })
}

async fn get_config(State(state): State<AppState>) -> Json<RagConfig> {
    Json(state.config.read().await.clone())
}

async fn update_config(
    State(state): State<AppState>,
    Json(update): Json<ConfigUpdate>,
) -> Result<Json<RagConfig>, ApiError> {
    let mut config = state.config.write().await;
    if let Some(value) = update.dense_top_k {
        config.dense_top_k = value;
    }
    if let Some(value) = update.bm25_top_k {
        config.bm25_top_k = value;
    }
    if let Some(value) = update.sparse_top_k {
        config.sparse_top_k = value;
    }
    if let Some(value) = update.rrf_k {
        config.rrf_k = value;
    }
    if let Some(value) = update.reranker_top_n {
        config.reranker_top_n = value;
    }
    if let Some(value) = update.score_threshold {
        config.score_threshold = value;
    }
    if let Some(value) = update.context_budget_tokens {
        config.context_budget_tokens = value;
    }
    if let Some(value) = update.chunk_size {
        config.chunk_size = value;
    }
    if let Some(value) = update.chunk_overlap {
        config.chunk_overlap = value;
    }
    validate_config(&config)?;
    Ok(Json(config.clone()))
}

async fn list_documents(
    State(state): State<AppState>,
) -> Result<Json<Vec<DocumentRecord>>, ApiError> {
    fs::create_dir_all(&state.storage_root)
        .await
        .map_err(internal_error)?;
    let mut entries = fs::read_dir(&state.storage_root)
        .await
        .map_err(internal_error)?;
    let mut documents = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(internal_error)? {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|x| x.to_str()) else {
            continue;
        };
        if name == "manifest.jsonl" || name.starts_with('.') {
            continue;
        }
        let metadata = entry.metadata().await.map_err(internal_error)?;
        if !metadata.is_file() {
            continue;
        }
        let Some(extension) = extension_for(name) else {
            continue;
        };
        documents.push(DocumentRecord {
            id: name.split('-').next().unwrap_or(name).to_owned(),
            name: name.to_owned(),
            size_bytes: metadata.len() as usize,
            extension: extension.to_owned(),
            status: "stored",
        });
    }
    documents.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(documents))
}

async fn upload_documents(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Vec<DocumentRecord>>, ApiError> {
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
        let extension = extension_for(&safe_name)
            .ok_or_else(|| bad_request("unsupported file type"))?
            .to_owned();
        let mut hasher = Sha256::new();
        let mut bytes_written = 0usize;
        let sequence = UPLOAD_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_path =
            state
                .storage_root
                .join(format!(".upload-{}-{}", std::process::id(), sequence));
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
            extension,
            status: "queued",
        });
    }

    Ok(Json(documents))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let storage_root = std::env::var("SUPERSONICRAG_STORAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data/knowledge"));
    let config = RagConfig::default();
    validate_config(&config).expect("default config must be valid");
    let state = AppState {
        storage_root,
        config: Arc::new(tokio::sync::RwLock::new(config)),
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE])
        .allow_origin("*".parse::<HeaderValue>().expect("valid CORS origin"));
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/config", get(get_config).post(update_config))
        .route("/v1/documents", get(list_documents).post(upload_documents))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .layer(cors)
        .with_state(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("failed to bind API listener");
    info!(%address, "SupersonicRAG API listening");
    axum::serve(listener, app).await.expect("API server failed");
}
