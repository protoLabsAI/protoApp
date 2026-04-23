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
pub mod state;

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
        .route("/healthz", get(healthz))
        .with_state(state)
        .layer(cors)
}

async fn healthz() -> &'static str {
    "ok"
}

/// Bind the router to 127.0.0.1:0 (ephemeral port) and return the socket
/// address plus a future that runs the server until the app exits.
pub async fn bind() -> std::io::Result<(SocketAddr, impl std::future::Future<Output = std::io::Result<()>>)> {
    let state = Arc::new(AppState::new());
    let app = router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let fut = async move { axum::serve(listener, app).await };
    Ok((addr, fut))
}
