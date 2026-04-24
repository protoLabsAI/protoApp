use std::collections::HashSet;
use std::sync::{Arc, LazyLock};

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use super::state::AppState;

/// `GET /v1/models` — OpenAI-compatible model listing.
///
/// Returns the union of models this server knows how to serve. With no engine
/// features enabled we still advertise the defaults so the UI can populate
/// its model picker while the user is deciding whether to build with
/// `--features llm`, `--features stt`, etc.
pub async fn list(State(_): State<Arc<AppState>>) -> Json<ModelList> {
    let data = default_models()
        .into_iter()
        .map(|m| ModelEntry {
            id: m.id.to_string(),
            object: "model",
            created: 0,
            owned_by: m.owner.to_string(),
        })
        .collect();

    Json(ModelList {
        object: "list",
        data,
    })
}

#[derive(Serialize)]
pub struct ModelList {
    pub object: &'static str,
    pub data: Vec<ModelEntry>,
}

#[derive(Serialize)]
pub struct ModelEntry {
    pub id: String,
    pub object: &'static str,
    pub created: u64,
    pub owned_by: String,
}

pub struct LocalModel {
    pub id: &'static str,
    pub owner: &'static str,
    pub kind: ModelKind,
}

#[derive(Clone, Copy)]
pub enum ModelKind {
    Chat,
    Transcription,
    Speech,
}

pub fn default_models() -> Vec<LocalModel> {
    vec![
        LocalModel {
            id: "qwen3-4b-instruct-2507",
            owner: "qwen",
            kind: ModelKind::Chat,
        },
        LocalModel {
            id: "whisper-large-v3-turbo",
            owner: "openai",
            kind: ModelKind::Transcription,
        },
        LocalModel {
            id: "kokoro-82m",
            owner: "hexgrad",
            kind: ModelKind::Speech,
        },
    ]
}

// O(1) lookup tables built once from the single `default_models()` source of
// truth, so adding a model only touches that one function.
static CHAT_MODELS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ids_for_kind(ModelKind::Chat));
static SPEECH_MODELS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ids_for_kind(ModelKind::Speech));

fn ids_for_kind(kind: ModelKind) -> HashSet<&'static str> {
    default_models()
        .into_iter()
        .filter(|m| std::mem::discriminant(&m.kind) == std::mem::discriminant(&kind))
        .map(|m| m.id)
        .collect()
}

/// Lookup: does the catalog contain a chat-capable model with this id?
pub fn is_chat_model(id: &str) -> bool {
    CHAT_MODELS.contains(id)
}

/// Lookup: does the catalog contain a speech-capable (TTS) model with this id?
pub fn is_speech_model(id: &str) -> bool {
    SPEECH_MODELS.contains(id)
}

static TRANSCRIPTION_MODELS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ids_for_kind(ModelKind::Transcription));

/// Lookup: does the catalog contain a transcription (STT) model with this id?
pub fn is_transcription_model(id: &str) -> bool {
    TRANSCRIPTION_MODELS.contains(id)
}
