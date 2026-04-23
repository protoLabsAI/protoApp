//! OpenAI-compatible local HTTP server backed by in-process Rust inference.
//!
//! The router exposes `/v1/*` endpoints that mirror OpenAI's schema so any
//! OpenAI SDK can point `baseURL` at this server and work unchanged.
//!
//! In the default build, handlers return stub/streaming responses that prove
//! the plumbing is correct. Build with `--features llm` (and optionally
//! `metal`/`cuda`) to swap the stubs for real mistralrs inference; see the
//! `stt` / `tts` features for the other engines.

pub mod chat;
pub mod models;
pub mod speech;
pub mod state;
pub mod transcriptions;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use axum::routing::{get, post};
use tower_http::cors::{AllowOrigin, CorsLayer};

use state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    // Binding to 127.0.0.1 keeps the socket unreachable from the network,
    // but **not** from other pages in the user's browser: any site they
    // visit can issue `fetch("http://127.0.0.1:<port>/v1/...")`. CORS is
    // the wall that keeps those cross-origin reads from succeeding.
    //
    // We only allow requests whose `Origin` resolves to loopback (or is
    // absent — which is the normal case for the OpenAI SDK running in the
    // Tauri webview, or for `curl`). Server apps that want to open this
    // up should layer their own CorsLayer on top via [`router`] rather
    // than editing this default.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
            origin
                .to_str()
                .ok()
                .map(is_loopback_origin)
                .unwrap_or(false)
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
        ]);

    Router::new()
        .route("/v1/models", get(models::list))
        .route("/v1/chat/completions", post(chat::completions))
        .route("/v1/audio/transcriptions", post(transcriptions::create))
        .route("/v1/audio/speech", post(speech::create))
        .route("/healthz", get(healthz))
        .with_state(state)
        .layer(cors)
}

/// True if the origin's host is a loopback address or a Tauri webview scheme.
/// Missing `Origin` headers never reach this function — they're handled by
/// the predicate returning `false` from the `.to_str()` miss path.
fn is_loopback_origin(origin: &str) -> bool {
    // Tauri's webview uses distinct schemes per platform; accept them as
    // "same app" even though they're nominally cross-origin.
    const TAURI_SCHEMES: &[&str] = &[
        "tauri://",
        "https://tauri.localhost", // Windows
        "http://tauri.localhost",
    ];
    if TAURI_SCHEMES.iter().any(|p| origin.starts_with(p)) {
        return true;
    }
    // Anything else: parse and require a loopback host.
    if let Ok(url) = url::Url::parse(origin) {
        if let Some(host) = url.host_str() {
            return matches!(host, "localhost" | "127.0.0.1" | "::1")
                || host.starts_with("127.");
        }
    }
    false
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
