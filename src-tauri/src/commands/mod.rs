use serde::{Deserialize, Serialize};
use specta::Type;

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
