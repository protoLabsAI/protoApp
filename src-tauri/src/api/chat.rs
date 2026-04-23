//! `POST /v1/chat/completions` — OpenAI-compatible chat endpoint.
//!
//! Two response shapes:
//!   * `stream: false` → single JSON body
//!   * `stream: true`  → Server-Sent Events with `data: {...}\n\n` chunks
//!     terminated by `data: [DONE]\n\n`
//!
//! Without the `engines` feature, we emit a placeholder echo so the frontend
//! plumbing can be exercised end-to-end before pulling in mistralrs.

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
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Response {
    if req.stream {
        stream_response(req).into_response()
    } else {
        json_response(req).into_response()
    }
}

fn json_response(req: ChatRequest) -> (StatusCode, Json<ChatCompletion>) {
    let reply = stub_reply(&req);
    let body = ChatCompletion {
        id: format!("chatcmpl-{}", Uuid::new_v4().simple()),
        object: "chat.completion",
        created: now_secs(),
        model: req.model,
        choices: vec![ChoiceFull {
            index: 0,
            message: ChatMessage {
                role: "assistant".into(),
                content: reply,
            },
            finish_reason: "stop",
        }],
    };
    (StatusCode::OK, Json(body))
}

fn stream_response(req: ChatRequest) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let id = format!("chatcmpl-{}", Uuid::new_v4().simple());
    let created = now_secs();
    let model = req.model.clone();
    let reply = stub_reply(&req);

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

        // Token chunks — one word per chunk, 40ms cadence, so the UI animates.
        for word in reply.split_inclusive(' ') {
            let chunk = ChatChunk {
                id: id.clone(),
                object: "chat.completion.chunk",
                created,
                model: model.clone(),
                choices: vec![ChoiceDelta {
                    index: 0,
                    delta: Delta { role: None, content: Some(word.to_string()) },
                    finish_reason: None,
                }],
            };
            yield Ok(Event::default().data(serde_json::to_string(&chunk).unwrap()));
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
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
                finish_reason: Some("stop"),
            }],
        };
        yield Ok(Event::default().data(serde_json::to_string(&last).unwrap()));

        // OpenAI clients expect a literal [DONE] sentinel.
        yield Ok(Event::default().data("[DONE]"));
    };

    Sse::new(s).keep_alive(KeepAlive::default())
}

fn stub_reply(req: &ChatRequest) -> String {
    let last_user = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.as_str())
        .unwrap_or("(no user message)");

    format!(
        "[stub reply — enable the `engines` feature for real Gemma 4 inference] \
         You said: {last_user}"
    )
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
