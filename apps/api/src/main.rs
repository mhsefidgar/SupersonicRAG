use std::{
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use sha2::{Digest, Sha256};
use tokio::fs;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::info;
use uuid::Uuid;

mod config;
mod models;
mod sanitize;

use config::AppConfig;
use models::{HealthResponse, UploadResponse};
use sanitize::{extension_for, sanitize_filename};

static UPLOAD_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const MAX_FILES_PER_REQUEST: usize = 20;
const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    storage_root: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::from_env()?;
    fs::create_dir_all(&config.storage_root).await?;

    let state = AppState {
        storage_root: config.storage_root,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/upload", post(upload))
        .layer(DefaultBodyLimit::max(MAX_FILES_PER_REQUEST * MAX_FILE_BYTES))
        .layer(SetRequestIdLayer::new(
            axum::http::HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(PropagateRequestIdLayer::new(
            axum::http::HeaderName::from_static("x-request-id"),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = config.bind_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "API listening");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_owned(),
    })
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut count = 0usize;
    let mut uploaded = Vec::new();

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|err| bad_request(err.to_string()))?
    {
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
        let temp_path = state
            .storage_root
            .join(format!(".upload-{}-{}", std::process::id(), sequence));
        let mut file = fs::File::create(&temp_path)
            .await
            .map_err(internal_error)?;
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
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .map_err(internal_error)?;
        }
        tokio::io::AsyncWriteExt::flush(&mut file)
            .await
            .map_err(internal_error)?;
        drop(file);

        let digest = format!("{:x}", hasher.finalize());
        let final_name = format!("{}-{}.{}", Uuid::new_v4(), &digest[..16], extension);
        let final_path = state.storage_root.join(&final_name);
        fs::rename(&temp_path, &final_path)
            .await
            .map_err(internal_error)?;

        uploaded.push(UploadResponse {
            filename: original_name,
            stored_as: final_name,
            bytes: bytes_written,
            sha256: digest,
        });
    }

    Ok(Json(uploaded))
}

fn bad_request(message: String) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, message)
}

fn internal_error<E: std::fmt::Display>(error: E) -> (StatusCode, String) {
    tracing::error!(%error, "internal API error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal server error".to_owned(),
    )
}

#[allow(dead_code)]
fn _keep_imports_used() -> HashMap<String, String> {
    HashMap::new()
}

#[allow(dead_code)]
fn _infallible(_: Infallible) {}

#[allow(dead_code)]
fn _path(_: &Path) {}
