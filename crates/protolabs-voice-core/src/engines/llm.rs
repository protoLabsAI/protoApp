//! Gemma 4 E2B via mistralrs GGUF loader.
//!
//! First call to [`load_default`] downloads the weights from Hugging Face
//! and builds the model — that's a ~1.5 GB download and several minutes on a
//! cold machine. Subsequent calls reuse the cached binary.

use anyhow::Result;
use mistralrs::{
    ChatCompletionChunkResponse, ChunkChoice, Delta, GgufModelBuilder, Model, RequestBuilder,
    Response, TextMessageRole,
};
use tokio::sync::mpsc;

use crate::api::chat::ChatMessage;

/// Build the default local Gemma 4 E2B instance.
///
/// We intentionally accept the HF defaults for cache directory; mistralrs
/// reuses `~/.cache/huggingface/hub` just like the Python ecosystem, so
/// weights are shared with other tools on the box.
pub async fn load_default() -> Result<Model> {
    tracing::info!("Loading Gemma 4 E2B (unsloth GGUF Q4_K_M) — first run downloads ~1.5 GB");

    let model = GgufModelBuilder::new(
        "unsloth/gemma-4-E2B-it-GGUF",
        vec!["gemma-4-E2B-it-Q4_K_M.gguf"],
    )
    .with_logging()
    .build()
    .await?;

    tracing::info!("Gemma 4 E2B ready");
    Ok(model)
}

/// Kick off a streaming chat completion. Deltas land in `tx` as plain strings;
/// the channel closes when the model emits its final chunk. Any error is
/// logged and propagated by dropping the sender.
pub async fn stream(model: &Model, history: Vec<ChatMessage>, tx: mpsc::Sender<String>) {
    let mut request = RequestBuilder::new();
    for msg in &history {
        let role = match msg.role.as_str() {
            "system" => TextMessageRole::System,
            "assistant" => TextMessageRole::Assistant,
            _ => TextMessageRole::User,
        };
        request = request.add_message(role, msg.content.clone());
    }

    let mut stream = match model.stream_chat_request(request).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(?e, "stream_chat_request failed");
            return;
        }
    };

    while let Some(chunk) = stream.next().await {
        match chunk {
            Response::Chunk(ChatCompletionChunkResponse { choices, .. }) => {
                if let Some(ChunkChoice {
                    delta: Delta {
                        content: Some(content),
                        ..
                    },
                    ..
                }) = choices.first()
                {
                    if tx.send(content.clone()).await.is_err() {
                        return; // client disconnected
                    }
                }
            }
            Response::Done(_) | Response::CompletionDone(_) => break,
            Response::InternalError(e) | Response::ValidationError(e) => {
                tracing::error!(?e, "model returned error");
                break;
            }
            _ => {} // ModelError, CompletionChunk, etc. — ignore for now
        }
    }
}
