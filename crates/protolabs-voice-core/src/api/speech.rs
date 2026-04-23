//! `POST /v1/audio/speech` — OpenAI-compatible TTS endpoint.
//!
//! JSON body: `{ "model": "kokoro-82m", "input": "...", "voice": "af_heart",
//! "response_format": "wav" | "mp3" }`. Returns raw audio bytes with the
//! matching `Content-Type`.
//!
//! Without the `engines` feature we return a minimal valid WAV containing
//! silence so the client can still play back something and we can verify the
//! transport end-to-end.

use std::sync::Arc;

use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use super::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SpeechRequest {
    #[serde(default)]
    pub model: Option<String>,
    pub input: String,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub response_format: Option<String>,
    #[serde(default)]
    pub speed: Option<f32>,
}

pub async fn create(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SpeechRequest>,
) -> Response {
    // --- Validate everything before we do any work -----------------------

    if req.input.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "`input` must not be empty").into_response();
    }

    // Model: if the caller pins one, require it to be a speech model we serve.
    // Passing `None` or omitting the field falls through to our default.
    if let Some(m) = req.model.as_deref() {
        if !super::models::is_speech_model(m) {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("speech model `{m}` not found"),
                        "type": "invalid_request_error",
                        "param": "model",
                        "code": "model_not_found",
                    }
                })),
            )
                .into_response();
        }
    }

    let fmt = req.response_format.as_deref().unwrap_or("wav");
    if !matches!(fmt, "wav" | "mp3") {
        return (
            StatusCode::BAD_REQUEST,
            format!("unsupported response_format `{fmt}`; use `wav` or `mp3`"),
        )
            .into_response();
    }

    // --- Work ------------------------------------------------------------

    let voice = req.voice.as_deref().unwrap_or("af_heart");
    let audio = synthesize(&req.input, voice).await;

    let mut headers = HeaderMap::new();
    // Always WAV on the wire today; mp3 transcoding is a TODO. We surface
    // that to clients that asked for mp3 via a custom header so they can
    // warn users instead of silently failing to play the audio.
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("audio/wav"));
    if fmt == "mp3" {
        headers.insert(
            "x-protoapp-note",
            HeaderValue::from_static("mp3 encoding not yet implemented; returning wav"),
        );
    }
    (headers, Body::from(audio)).into_response()
}

#[cfg(not(feature = "tts"))]
async fn synthesize(_input: &str, _voice: &str) -> Vec<u8> {
    // 1 second of silence at 24 kHz mono f32 — enough for the UI to detect
    // "something came back" without a real TTS engine loaded.
    silence_wav_24khz(1.0)
}

#[cfg(feature = "tts")]
async fn synthesize(_input: &str, _voice: &str) -> Vec<u8> {
    // TODO(step-1f): wire tts-rs (Kokoro) here once model-download helper lands.
    silence_wav_24khz(1.0)
}

/// Build an in-memory WAV of the given duration (silent f32 at 24 kHz mono).
/// Small enough to inline; avoids the `hound` dep on the default path.
fn silence_wav_24khz(seconds: f32) -> Vec<u8> {
    let sample_rate: u32 = 24_000;
    let num_samples = (sample_rate as f32 * seconds) as u32;
    let byte_rate = sample_rate * 4; // 1 ch * 4 bytes (f32)
    let data_size = num_samples * 4;
    let chunk_size = 36 + data_size;

    let mut w = Vec::with_capacity(44 + data_size as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&chunk_size.to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes()); // subchunk1 size
    w.extend_from_slice(&3u16.to_le_bytes()); // audio format = IEEE float
    w.extend_from_slice(&1u16.to_le_bytes()); // channels
    w.extend_from_slice(&sample_rate.to_le_bytes());
    w.extend_from_slice(&byte_rate.to_le_bytes());
    w.extend_from_slice(&4u16.to_le_bytes()); // block align
    w.extend_from_slice(&32u16.to_le_bytes()); // bits per sample
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_size.to_le_bytes());
    for _ in 0..num_samples {
        w.extend_from_slice(&0f32.to_le_bytes());
    }
    w
}
