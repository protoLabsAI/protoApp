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
