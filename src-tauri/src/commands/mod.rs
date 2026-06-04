use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::ApiServer;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct GreetResponse {
    pub message: String,
    pub version: String,
}

#[tauri::command]
#[specta::specta]
pub fn greet(name: String) -> GreetResponse {
    GreetResponse {
        message: format!("Hello, {name}! Greeted from Rust."),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Base URL for the in-process OpenAI-compatible server,
/// e.g. `http://127.0.0.1:53217`. The frontend passes this to the OpenAI SDK.
#[tauri::command]
#[specta::specta]
pub fn get_api_base_url(server: State<'_, ApiServer>) -> String {
    format!("http://{}", server.addr)
}

/// Default model the local voice-core server serves — used when the frontend
/// doesn't override it. MUST match an id in voice-core's catalog
/// (`api::models::default_models`), or `/v1/chat/completions` returns 404
/// `model_not_found`. The catalog id is lowercase.
const DEFAULT_AGENT_MODEL: &str = "qwen3-4b-instruct-2507";

/// Drive one in-process agent turn (zeroclaw) and return the reply.
///
/// `base_url` is voice-core's server root (the frontend already resolves it via
/// [`get_api_base_url`]); we take it as an argument rather than `State` so the
/// command can be `async` without holding the state borrow across `.await`.
/// The agent's LLM provider is wired to `{base_url}/v1`, so inference runs on
/// the same local model the Chat tab uses — no network, no Python sidecar.
#[tauri::command]
#[specta::specta]
pub async fn agent_ask(
    base_url: String,
    model: Option<String>,
    message: String,
    session_id: Option<String>,
) -> Result<String, String> {
    let model = model.unwrap_or_else(|| DEFAULT_AGENT_MODEL.to_string());
    protoapp_agent::ask(&base_url, &model, &message, session_id.as_deref())
        .await
        .map_err(|e| format!("{e:#}"))
}
