//! `POST /v1/audio/transcriptions` — OpenAI-compatible STT endpoint.
//!
//! Multipart form: `file`, `model`, `response_format?`.
//! Returns `{"text": "..."}` on `json` (default) or the raw text when the
//! client asks for `response_format=text`.
//!
//! With the `stt` feature we hand the file bytes to whisper-rs (see
//! `engines::stt`). Without it we return a stub acknowledging the file size
//! so the frontend plumbing can be exercised end-to-end.
//!
//! The frontend's `useTranscription` hook sends 16 kHz mono PCM16 WAV so the
//! server never touches an audio codec; clients that upload a different WAV
//! sample-rate get naively resampled on the server.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use super::state::AppState;

#[derive(Debug, Serialize)]
pub struct TranscriptionResponse {
    pub text: String,
}

pub async fn create(
    State(_state): State<Arc<AppState>>,
    mut form: Multipart,
) -> Response {
    let mut audio_bytes: Vec<u8> = Vec::new();
    let mut model = "whisper-large-v3-turbo".to_string();
    let mut response_format = "json".to_string();

    loop {
        let field = match form.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break, // end of form
            Err(e) => {
                // A malformed multipart body used to be silently truncated —
                // callers now get a 400 so partial accepts can't masquerade
                // as successful requests.
                return (
                    StatusCode::BAD_REQUEST,
                    format!("invalid multipart body: {e}"),
                )
                    .into_response();
            }
        };

        match field.name().unwrap_or("").to_string().as_str() {
            "file" => {
                audio_bytes = match field.bytes().await {
                    Ok(b) => b.to_vec(),
                    Err(e) => {
                        return (StatusCode::BAD_REQUEST, format!("read file failed: {e}"))
                            .into_response();
                    }
                };
            }
            "model" => {
                // Only overwrite the default on a successful read — a failed
                // multipart parse should leave the "whisper-large-v3-turbo"
                // default intact, not blank it.
                if let Ok(v) = field.text().await {
                    if !v.is_empty() {
                        model = v;
                    }
                }
            }
            "response_format" => {
                // Mirror the `model` handling: only overwrite the default
                // when the parsed text is non-empty, so an empty or failed
                // read leaves `"json"` in place instead of producing an
                // empty string that immediately fails validation below.
                if let Ok(v) = field.text().await {
                    if !v.is_empty() {
                        response_format = v;
                    }
                }
            }
            _ => {
                let _ = field.bytes().await; // drain unknown fields
            }
        }
    }

    if audio_bytes.is_empty() {
        return (StatusCode::BAD_REQUEST, "missing `file` field").into_response();
    }

    // Validate response_format before doing any work so invalid requests
    // fail fast rather than getting a JSON body they didn't ask for.
    let fmt_response = match response_format.as_str() {
        "json" => false,
        "text" => true,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                format!("unsupported `response_format`: `{other}`; use `json` or `text`"),
            )
                .into_response();
        }
    };

    let text = transcribe(&audio_bytes, &model).await;

    if fmt_response {
        text.into_response()
    } else {
        Json(TranscriptionResponse { text }).into_response()
    }
}

#[cfg(not(feature = "stt"))]
async fn transcribe(bytes: &[u8], model: &str) -> String {
    format!(
        "[stub transcription — build with `--features stt` to enable whisper-rs; \
         needs cmake on the build host] received {} bytes for model {}",
        bytes.len(),
        model
    )
}

#[cfg(feature = "stt")]
async fn transcribe(bytes: &[u8], _model: &str) -> String {
    match crate::engines::stt::transcribe(bytes).await {
        Ok(text) => text,
        Err(e) => {
            tracing::error!(?e, "whisper transcription failed");
            format!("[transcription error: {e}]")
        }
    }
}
