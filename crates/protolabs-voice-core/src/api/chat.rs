//! `POST /v1/chat/completions` — OpenAI-compatible chat endpoint.
//!
//! Two response shapes:
//!   * `stream: false` → single JSON body
//!   * `stream: true`  → Server-Sent Events with `data: {...}\n\n` chunks
//!     terminated by `data: [DONE]\n\n`
//!
//! Without the `llm` feature (or when it's on but model load fails), we emit
//! a placeholder echo so the frontend plumbing can be exercised end-to-end.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_stream::stream;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatCompletion {
    pub id: String,
    pub object: &'static str,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChoiceFull>,
}

#[derive(Debug, Serialize)]
pub struct ChoiceFull {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ChatChunk {
    pub id: String,
    pub object: &'static str,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChoiceDelta>,
}

#[derive(Debug, Serialize)]
pub struct ChoiceDelta {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<&'static str>,
}

#[derive(Debug, Serialize, Default)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

pub async fn completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Response {
    // Reject requests for models we don't serve. We still advertise a model id
    // even when the real engine isn't compiled in, so the check is against the
    // catalog — not against whether the engine is actually loaded.
    if !super::models::is_chat_model(&req.model) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "message": format!("model `{}` not found", req.model),
                    "type": "invalid_request_error",
                    "param": "model",
                    "code": "model_not_found",
                }
            })),
        )
            .into_response();
    }

    if req.stream {
        stream_response(state, req).await.into_response()
    } else {
        json_response(state, req).await.into_response()
    }
}

/// Terminal outcome of `drive_backend`. The mpsc sender streams tokens; this
/// channel carries the final success/failure signal so we can distinguish a
/// clean finish from a silent drop (e.g. the backend panicked or aborted).
type BackendOutcome = std::result::Result<(), BackendError>;

#[derive(Debug, Clone)]
struct BackendError {
    message: String,
}

async fn json_response(state: Arc<AppState>, req: ChatRequest) -> Response {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);
    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<BackendOutcome>();
    let req_clone = req.clone_messages();
    tokio::spawn(async move {
        drive_backend(state, req_clone, tx, done_tx).await;
    });

    let mut content = String::new();
    while let Some(piece) = rx.recv().await {
        content.push_str(&piece);
    }
    let outcome = done_rx.await.unwrap_or_else(|_| {
        Err(BackendError {
            message: "backend dropped without signalling".into(),
        })
    });

    match outcome {
        Ok(()) => {
            let body = ChatCompletion {
                id: format!("chatcmpl-{}", Uuid::new_v4().simple()),
                object: "chat.completion",
                created: now_secs(),
                model: req.model,
                choices: vec![ChoiceFull {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".into(),
                        content,
                    },
                    finish_reason: "stop",
                }],
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": {
                    "message": e.message,
                    "type": "server_error",
                    "code": "backend_failure",
                }
            })),
        )
            .into_response(),
    }
}

async fn stream_response(
    state: Arc<AppState>,
    req: ChatRequest,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let id = format!("chatcmpl-{}", Uuid::new_v4().simple());
    let created = now_secs();
    let model = req.model.clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);
    let (done_tx, mut done_rx) = tokio::sync::oneshot::channel::<BackendOutcome>();
    let req_clone = req.clone_messages();
    tokio::spawn(async move {
        drive_backend(state, req_clone, tx, done_tx).await;
    });

    let s = stream! {
        // Opening chunk — role only.
        let first = ChatChunk {
            id: id.clone(),
            object: "chat.completion.chunk",
            created,
            model: model.clone(),
            choices: vec![ChoiceDelta {
                index: 0,
                delta: Delta { role: Some("assistant".into()), content: None },
                finish_reason: None,
            }],
        };
        yield Ok(Event::default().data(serde_json::to_string(&first).unwrap()));

        while let Some(piece) = rx.recv().await {
            let chunk = ChatChunk {
                id: id.clone(),
                object: "chat.completion.chunk",
                created,
                model: model.clone(),
                choices: vec![ChoiceDelta {
                    index: 0,
                    delta: Delta { role: None, content: Some(piece) },
                    finish_reason: None,
                }],
            };
            yield Ok(Event::default().data(serde_json::to_string(&chunk).unwrap()));
        }

        let outcome = (&mut done_rx).await.unwrap_or_else(|_| {
            Err(BackendError { message: "backend dropped without signalling".into() })
        });
        let finish_reason = if outcome.is_ok() { "stop" } else { "error" };

        if let Err(e) = &outcome {
            // Emit a non-standard error frame before the terminator so clients
            // that check for it can surface it; the OpenAI SDK simply ignores
            // unknown keys.
            let err_frame = serde_json::json!({
                "id": id.clone(),
                "object": "chat.completion.chunk",
                "created": created,
                "model": model.clone(),
                "error": { "message": e.message.clone(), "type": "server_error" },
            });
            yield Ok(Event::default().data(err_frame.to_string()));
        }

        // Closing chunk — finish_reason.
        let last = ChatChunk {
            id: id.clone(),
            object: "chat.completion.chunk",
            created,
            model: model.clone(),
            choices: vec![ChoiceDelta {
                index: 0,
                delta: Delta::default(),
                finish_reason: Some(finish_reason),
            }],
        };
        yield Ok(Event::default().data(serde_json::to_string(&last).unwrap()));

        // OpenAI clients expect a literal [DONE] sentinel.
        yield Ok(Event::default().data("[DONE]"));
    };

    Sse::new(s).keep_alive(KeepAlive::default())
}

/// Dispatch to the real engine when it's compiled in; otherwise emit a stub
/// stream so the frontend and transport can still be exercised end-to-end.
///
/// Always sends a terminal [`BackendOutcome`] on `done`, even if we took the
/// stub fallback path (in which case it's `Ok(())`). That's the signal the
/// response handlers use to distinguish "clean finish" from "silent drop".
async fn drive_backend(
    _state: Arc<AppState>,
    req: ChatRequest,
    tx: tokio::sync::mpsc::Sender<String>,
    done: tokio::sync::oneshot::Sender<BackendOutcome>,
) {
    #[cfg(feature = "llm")]
    {
        match _state.llm_or_load().await {
            Ok(model) => {
                let outcome = crate::engines::llm::stream(
                    model,
                    req.messages,
                    req.temperature,
                    req.max_tokens,
                    tx,
                )
                .await
                .map_err(|message| BackendError { message });
                let _ = done.send(outcome);
                return;
            }
            Err(e) => {
                tracing::error!(?e, "failed to load LLM engine; falling back to stub");
                // Fall through to stub so the client still sees something.
            }
        }
    }

    emit_stub(&req, &tx).await;
    let _ = done.send(Ok(()));
}

async fn emit_stub(req: &ChatRequest, tx: &tokio::sync::mpsc::Sender<String>) {
    let last_user = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.as_str())
        .unwrap_or("(no user message)");

    let reply = format!(
        "[stub reply — build with `--features llm` (optionally with `metal` on macOS or `cuda` on NVIDIA, e.g. `--features \"llm metal\"`) for real Gemma 4 inference] \
         You said: {last_user}"
    );

    for word in reply.split_inclusive(' ') {
        if tx.send(word.to_string()).await.is_err() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    }
}

impl ChatRequest {
    fn clone_messages(&self) -> ChatRequest {
        ChatRequest {
            model: self.model.clone(),
            messages: self.messages.clone(),
            stream: self.stream,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
