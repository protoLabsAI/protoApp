//! OpenAI-compatible local HTTP server backed by in-process Rust inference.
//!
//! The router exposes `/v1/*` endpoints that mirror OpenAI's schema so any
//! OpenAI SDK can point `baseURL` at this server and work unchanged.
//!
//! In the default build, handlers return stub/streaming responses that prove
//! the plumbing is correct. Enable the `engines` cargo feature to swap the
//! stubs for real mistralrs / whisper-rs / kokoros backends.

pub mod chat;
pub mod models;
pub mod speech;
pub mod state;
pub mod transcriptions;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::{Any, CorsLayer};

use state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/v1/models", get(models::list))
        .route("/v1/chat/completions", post(chat::completions))
        .route("/v1/audio/transcriptions", post(transcriptions::create))
        .route("/v1/audio/speech", post(speech::create))
        .route("/healthz", get(healthz))
        .with_state(state)
        .layer(cors)
}

async fn healthz() -> &'static str {
    "ok"
}

/// Bind the router to 127.0.0.1:0 (ephemeral port) and return the socket
/// address plus a future that runs the server until the app exits. A fresh
/// [`AppState`] is created internally — use [`bind_with_state`] if you need
/// to share state with Tauri commands or other callers.
pub async fn bind() -> std::io::Result<(SocketAddr, impl std::future::Future<Output = std::io::Result<()>>)> {
    bind_with_state(Arc::new(AppState::new())).await
}

/// Same as [`bind`], but uses a caller-supplied [`AppState`]. Useful when
/// a host app wants to reuse the same engine handles from both the HTTP
/// surface and its own in-process code (e.g. a Tauri command that preloads
/// the LLM or a voice pipeline that calls the engines directly).
pub async fn bind_with_state(
    state: Arc<AppState>,
) -> std::io::Result<(SocketAddr, impl std::future::Future<Output = std::io::Result<()>>)> {
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let fut = async move { axum::serve(listener, app).await };
    Ok((addr, fut))
}
