//! Gemma 4 E2B via mistralrs GGUF loader.
//!
//! First call to [`load_default`] downloads the weights from Hugging Face
//! and builds the model — that's a ~1.5 GB download and several minutes on a
//! cold machine. Subsequent calls reuse the cached binary.

use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use mistralrs::{
    ChatCompletionChunkResponse, ChunkChoice, Delta, GgufModelBuilder, Model, RequestBuilder,
    Response, TextMessageRole,
};
use tokio::sync::mpsc;

use crate::api::chat::ChatMessage;

/// Hard cap on initial model load (download + warmup).
/// Tunable via `PROTOAPP_LLM_LOAD_TIMEOUT_SECS`.
fn load_timeout() -> Duration {
    const DEFAULT_SECS: u64 = 15 * 60;
    std::env::var("PROTOAPP_LLM_LOAD_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(DEFAULT_SECS))
}

/// Build the default local Gemma 4 E2B instance.
///
/// We intentionally accept the HF defaults for cache directory; mistralrs
/// reuses `~/.cache/huggingface/hub` just like the Python ecosystem, so
/// weights are shared with other tools on the box.
pub async fn load_default() -> Result<Model> {
    let timeout = load_timeout();
    tracing::info!(
        ?timeout,
        "Loading Gemma 4 E2B (unsloth GGUF Q4_K_M) — first run downloads ~1.5 GB"
    );

    let build_fut = GgufModelBuilder::new(
        "unsloth/gemma-4-E2B-it-GGUF",
        vec!["gemma-4-E2B-it-Q4_K_M.gguf"],
    )
    .with_logging()
    .build();

    let model = tokio::time::timeout(timeout, build_fut)
        .await
        .map_err(|_| {
            tracing::error!(?timeout, "LLM load timed out");
            anyhow!(
                "LLM load timed out after {:?}; set PROTOAPP_LLM_LOAD_TIMEOUT_SECS to extend",
                timeout
            )
        })?
        .context("GgufModelBuilder::build failed")?;

    tracing::info!("Gemma 4 E2B ready");
    Ok(model)
}

/// Kick off a streaming chat completion. Deltas land in `tx` as plain strings;
/// the channel closes when the model emits its final chunk.
///
/// Returns `Ok(())` on a clean finish or a client-disconnect, `Err(msg)` if
/// the backend signals a failure mid-stream (ModelError / InternalError /
/// ValidationError / the initial stream_chat_request itself failing). The
/// caller is expected to forward the error into the HTTP response.
pub async fn stream(
    model: &Model,
    history: Vec<ChatMessage>,
    tx: mpsc::Sender<String>,
) -> std::result::Result<(), String> {
    let mut request = RequestBuilder::new();
    for msg in &history {
        let role = match msg.role.as_str() {
            "system" => TextMessageRole::System,
            "assistant" => TextMessageRole::Assistant,
            "user" => TextMessageRole::User,
            other => {
                tracing::warn!(
                    role = %other,
                    "unrecognized chat role; defaulting to user — check the client"
                );
                TextMessageRole::User
            }
        };
        request = request.add_message(role, msg.content.clone());
    }

    let mut stream = model
        .stream_chat_request(request)
        .await
        .map_err(|e| {
            let msg = format!("stream_chat_request failed: {e}");
            tracing::error!(%msg);
            msg
        })?;

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
                        // Client went away — not an error on our side.
                        return Ok(());
                    }
                }
            }
            Response::Done(_) | Response::CompletionDone(_) => return Ok(()),
            Response::ModelError(msg, _) => {
                tracing::error!(%msg, "model error during generation");
                return Err(format!("model error: {msg}"));
            }
            Response::InternalError(e) => {
                tracing::error!(?e, "internal error during generation");
                return Err(format!("internal error: {e}"));
            }
            Response::ValidationError(e) => {
                tracing::error!(?e, "validation error during generation");
                return Err(format!("validation error: {e}"));
            }
            _ => {} // CompletionChunk / CompletionModelError / etc.
        }
    }
    Ok(())
}
