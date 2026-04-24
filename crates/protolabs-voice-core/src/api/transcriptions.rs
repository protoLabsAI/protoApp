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
    State(state): State<Arc<AppState>>,
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

    // Reject unknown transcription model ids fail-fast (mirrors the chat and
    // speech endpoints) so a typo or a hallucinated id doesn't get a real
    // Whisper result returned under a bogus label.
    if !super::models::is_transcription_model(&model) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": {
                    "message": format!("transcription model `{model}` not found"),
                    "type": "invalid_request_error",
                    "param": "model",
                    "code": "model_not_found",
                }
            })),
        )
            .into_response();
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

    match transcribe(&audio_bytes, &model, &state).await {
        Ok(text) => {
            if fmt_response {
                text.into_response()
            } else {
                Json(TranscriptionResponse { text }).into_response()
            }
        }
        Err(Failure::BadAudio(msg)) => {
            // Client-side problem — log at debug because this isn't a bug
            // in our side, and return 400 with an invalid_request_error so
            // the OpenAI SDK raises a sensible exception class.
            tracing::debug!(%msg, "rejecting malformed audio");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("could not decode audio: {msg}"),
                        "type": "invalid_request_error",
                        "param": "file",
                        "code": "bad_audio",
                    }
                })),
            )
                .into_response()
        }
        Err(Failure::Engine(msg)) => {
            tracing::error!(%msg, "transcription engine failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("transcription failed: {msg}"),
                        "type": "server_error",
                        "code": "transcription_failure",
                    }
                })),
            )
                .into_response()
        }
    }
}

/// Always-compiled shape of what the inner transcribe helper returns, so
/// the handler's `match` above doesn't need its own cfg arms. Under `stt`
/// we map `engines::stt::TranscriptionFailure` into this; under the stub
/// only `Ok` is ever produced — so the variants look dead when the `stt`
/// feature is off, but the handler still needs the match arms compiled.
#[derive(Debug)]
#[cfg_attr(not(feature = "stt"), allow(dead_code))]
enum Failure {
    BadAudio(String),
    Engine(String),
}

#[cfg(not(feature = "stt"))]
async fn transcribe(bytes: &[u8], model: &str, _state: &AppState) -> Result<String, Failure> {
    Ok(format!(
        "[stub transcription — build with `--features stt` to enable whisper-rs; \
         needs cmake on the build host] received {} bytes for model {}",
        bytes.len(),
        model
    ))
}

#[cfg(feature = "stt")]
async fn transcribe(bytes: &[u8], _model: &str, state: &AppState) -> Result<String, Failure> {
    use crate::engines::stt::TranscriptionFailure;
    // Model-id validation happens in the outer handler, not here — by this
    // point we've already confirmed the caller asked for an id we serve,
    // which today maps to the single cached whisper model.
    crate::engines::stt::transcribe(bytes, &state.emitter)
        .await
        .map_err(|e| match e {
            TranscriptionFailure::BadAudio(s) => Failure::BadAudio(s),
            TranscriptionFailure::Engine(s) => Failure::Engine(s),
        })
}
