use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use super::state::AppState;

/// `GET /v1/models` — OpenAI-compatible model listing.
///
/// Returns the union of models this server knows how to serve. When the
/// `engines` feature is off, we still advertise the defaults so the UI can
/// populate its model picker while the user is deciding whether to enable
/// real inference.
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
            id: "gemma-4-e2b",
            owner: "google",
            kind: ModelKind::Chat,
        },
        LocalModel {
            id: "gemma-4-e4b",
            owner: "google",
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
