use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use sha2::{Digest, Sha256};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{atomic::{AtomicU64, Ordering}, Arc},
};
use tokio::fs;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

// Existing application code continues below.
