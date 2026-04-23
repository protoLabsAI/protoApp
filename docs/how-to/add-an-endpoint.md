# Add an OpenAI-compatible endpoint

Say you want to add `/v1/embeddings`. The pattern is the same for any
new endpoint.

## 1. Create the handler module

`crates/protolabs-voice-core/src/api/embeddings.rs`:

```rust
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use super::state::AppState;

#[derive(Deserialize)]
pub struct EmbeddingsRequest {
    pub model: String,
    pub input: EmbeddingInput,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Single(String),
    Batch(Vec<String>),
}

#[derive(Serialize)]
pub struct EmbeddingsResponse {
    pub object: &'static str,
    pub data: Vec<EmbeddingObject>,
    pub model: String,
}

#[derive(Serialize)]
pub struct EmbeddingObject {
    pub object: &'static str,
    pub embedding: Vec<f32>,
    pub index: u32,
}

pub async fn create(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<EmbeddingsRequest>,
) -> Json<EmbeddingsResponse> {
    // TODO: real implementation behind `#[cfg(feature = "llm")]`
    Json(EmbeddingsResponse {
        object: "list",
        data: vec![],
        model: req.model,
    })
}
```

## 2. Wire it into the router

In `crates/protolabs-voice-core/src/api/mod.rs`:

```rust
pub mod embeddings; // 1. declare the module

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/models", get(models::list))
        .route("/v1/chat/completions", post(chat::completions))
        .route("/v1/audio/transcriptions", post(transcriptions::create))
        .route("/v1/audio/speech", post(speech::create))
        .route("/v1/embeddings", post(embeddings::create)) // 2. register the route
        // ...
}
```

## 3. Add an integration test

`crates/protolabs-voice-core/tests/api_smoke.rs`:

```rust
#[tokio::test]
async fn embeddings_roundtrip() {
    let (addr, fut) = api::bind().await.unwrap();
    tokio::spawn(fut);

    let body: serde_json::Value = reqwest::Client::new()
        .post(format!("http://{addr}/v1/embeddings"))
        .json(&json!({ "model": "text-embedding-3-small", "input": "hello" }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(body["object"], "list");
}
```

## 4. Run

```sh
cargo test -p protolabs-voice-core
```

## Conventions

- Follow OpenAI's schema exactly — matching request/response shapes lets any OpenAI SDK (JS, Python, LangChain, Vercel AI SDK) hit our server with only a `baseURL` change.
- Feature-gate real engine work behind `#[cfg(feature = "llm")]` / `"stt"` / `"tts"` so the default build stays fast and installable without heavy native deps.
- Return a stub that exercises the transport when the engine feature is off — see `speech.rs` for the pattern (inline valid silent WAV).
- Update [reference/openai-api.md](../reference/openai-api.md) with the new endpoint's schema.
