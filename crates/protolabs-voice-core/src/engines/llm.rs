//! Qwen3-4B-Instruct-2507 via [`llama-cpp-2`][0] — Rust bindings to llama.cpp.
//!
//! Gemma 4 is the model we want (function calling, vision-capable), but
//! llama.cpp's Fused Gated Delta Network path asserts on its tensor
//! naming (see `Cargo.toml`). Qwen3-Instruct-2507 is a classic
//! transformer that never exercises the FGDN path, so it loads and
//! streams cleanly on our pinned `llama-cpp-sys-2 = 0.1.143`. Swap back
//! to Gemma 4 once upstream llama.cpp accepts Gemma 4's tensor names.
//!
//! Layering:
//!   * [`backend()`] holds a process-global `LlamaBackend` in a `OnceLock`.
//!     llama.cpp requires exactly one to exist at a time.
//!   * [`load_default`] downloads the GGUF (if missing) to
//!     `~/.cache/protoapp/llm/` and loads it on a `spawn_blocking` worker.
//!     Returns `Arc<LlamaModel>` — the model is read-only and cheap to
//!     share across request handlers.
//!   * [`stream`] runs one generation: build the chat prompt via the
//!     model's embedded template, tokenize, and drive the decode loop
//!     inside `spawn_blocking` while pushing detokenized pieces to an
//!     mpsc back to the caller (which is already listening from axum's
//!     SSE writer).
//!
//! [0]: https://crates.io/crates/llama-cpp-2

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use tokio::sync::mpsc;

use crate::api::chat::ChatMessage;
use crate::engines::events::{Engine, StatusEmitter, emit_error, emit_loading, emit_ready};

// -- Model choice ------------------------------------------------------------

// Qwen3-4B-Instruct-2507 is the current default because Gemma 4 and
// Qwen3.5 both trip llama.cpp's FGDN tensor-name assert on 0.1.143 and
// 0.1.145 alike (see `Cargo.toml`). Qwen3-Instruct-2507 is a classic
// attention transformer with no gated-delta tensors, so it sidesteps
// the FGDN path entirely. Apache-2.0, ~2.5 GB at Q4_K_M, confirmed
// streaming end-to-end on 2026-04-23.
const DEFAULT_MODEL_REPO: &str = "unsloth/Qwen3-4B-Instruct-2507-GGUF";
const DEFAULT_MODEL_FILE: &str = "Qwen3-4B-Instruct-2507-Q4_K_M.gguf";
const DEFAULT_CONTEXT_TOKENS: u32 = 8192;
const DEFAULT_MAX_NEW_TOKENS: u32 = 1024;
const DEFAULT_TEMPERATURE: f32 = 0.7;

// -- Timeouts / env knobs ----------------------------------------------------

/// Hard cap on the first-run download + model load (the warm path is seconds).
/// Tunable via `PROTOAPP_LLM_LOAD_TIMEOUT_SECS`.
fn load_timeout() -> Duration {
    const DEFAULT_SECS: u64 = 15 * 60;
    std::env::var("PROTOAPP_LLM_LOAD_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(DEFAULT_SECS))
}

fn download_timeout() -> Duration {
    std::env::var("PROTOAPP_LLM_DOWNLOAD_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(15 * 60))
}

// -- Process-global llama.cpp backend ---------------------------------------

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
// Guards the actual init so two racing threads don't both call
// `LlamaBackend::init` and then drop one (its Drop runs llama_backend_free,
// which would pull the rug out from under the other).
static BACKEND_INIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn backend() -> Result<&'static LlamaBackend> {
    if let Some(b) = BACKEND.get() {
        return Ok(b);
    }
    let _guard = BACKEND_INIT_LOCK
        .lock()
        .map_err(|_| anyhow!("BACKEND_INIT_LOCK poisoned"))?;
    if let Some(b) = BACKEND.get() {
        return Ok(b);
    }
    let backend = LlamaBackend::init().context("LlamaBackend::init")?;
    BACKEND
        .set(backend)
        .map_err(|_| anyhow!("BACKEND already initialized (unreachable under lock)"))?;
    Ok(BACKEND.get().expect("BACKEND set above"))
}

// -- Model download + load ---------------------------------------------------

fn cache_dir() -> Result<PathBuf> {
    let root = dirs::cache_dir()
        .ok_or_else(|| anyhow!("no cache dir available on this platform"))?;
    let dir = root.join("protoapp").join("llm");
    std::fs::create_dir_all(&dir).context("create llm cache dir")?;
    Ok(dir)
}

fn cached_model_path() -> Result<PathBuf> {
    if let Ok(override_path) = std::env::var("PROTOAPP_LLM_MODEL_PATH") {
        return Ok(PathBuf::from(override_path));
    }
    Ok(cache_dir()?.join(DEFAULT_MODEL_FILE))
}

async fn ensure_model(emitter: &Arc<dyn StatusEmitter>) -> Result<PathBuf> {
    let path = cached_model_path()?;
    if path.exists() {
        return Ok(path);
    }

    let url = format!(
        "https://huggingface.co/{repo}/resolve/main/{file}",
        repo = DEFAULT_MODEL_REPO,
        file = DEFAULT_MODEL_FILE,
    );
    tracing::info!(%url, to = %path.display(), "downloading LLM (first run only)");

    // Partial-then-rename, per-process temp suffix so concurrent downloads
    // don't stomp each other.
    let tmp = path.with_extension(format!(
        "gguf.partial.{}.{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    download_streaming(&url, &tmp, emitter).await?;
    if path.exists() {
        let _ = tokio::fs::remove_file(&tmp).await;
    } else {
        tokio::fs::rename(&tmp, &path)
            .await
            .context("rename LLM model into place")?;
    }
    Ok(path)
}

async fn download_streaming(
    url: &str,
    dst: &Path,
    emitter: &Arc<dyn StatusEmitter>,
) -> Result<()> {
    use crate::engines::events::emit_downloading;
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    let client = reqwest::Client::builder()
        .timeout(download_timeout())
        .build()
        .context("build reqwest client")?;
    let resp = client.get(url).send().await.context("GET model")?;
    if !resp.status().is_success() {
        bail!("LLM model download failed: HTTP {}", resp.status());
    }
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(dst).await?;
    let mut stream = resp.bytes_stream();
    let mut bytes_written: u64 = 0;
    let mut last_tick = std::time::Instant::now();
    emit_downloading(emitter, Engine::Llm, 0, total);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("read chunk")?;
        file.write_all(&chunk).await?;
        bytes_written += chunk.len() as u64;
        if last_tick.elapsed() > Duration::from_millis(500) {
            emit_downloading(emitter, Engine::Llm, bytes_written, total);
            if let Some(total) = total {
                tracing::info!(
                    percent = (bytes_written * 100 / total.max(1)),
                    "LLM download progress"
                );
            }
            last_tick = std::time::Instant::now();
        }
    }
    file.flush().await?;
    Ok(())
}

/// Build the default local LLM. Downloads on first call, caches on disk.
pub async fn load_default(emitter: &Arc<dyn StatusEmitter>) -> Result<Arc<LlamaModel>> {
    let timeout = load_timeout();
    tracing::info!(
        ?timeout,
        repo = DEFAULT_MODEL_REPO,
        file = DEFAULT_MODEL_FILE,
        "Loading Qwen3-4B-Instruct-2507 (GGUF Q4_K_M via llama.cpp)"
    );
    emit_loading(emitter, Engine::Llm);

    let fut = async {
        let path = ensure_model(emitter).await?;
        // `LlamaBackend` must exist before any model/context. Initialize
        // once on any thread — the whole crate shares one.
        let backend = backend()?;
        // Loading a GGUF is CPU-bound (mmap + metadata parse + tensor
        // validation); run off the async runtime so we don't block the
        // executor for seconds.
        let model = tokio::task::spawn_blocking(move || {
            let params = LlamaModelParams::default();
            LlamaModel::load_from_file(backend, &path, &params)
        })
        .await
        .context("spawn_blocking load_from_file")??;
        Ok::<_, anyhow::Error>(Arc::new(model))
    };

    let outcome = tokio::time::timeout(timeout, fut).await;
    let model = match outcome {
        Err(_) => {
            let msg = format!(
                "LLM load timed out after {:?}; set PROTOAPP_LLM_LOAD_TIMEOUT_SECS to extend",
                timeout
            );
            emit_error(emitter, Engine::Llm, &msg);
            return Err(anyhow!(msg));
        }
        Ok(Err(e)) => {
            let msg = format!("{e:#}");
            emit_error(emitter, Engine::Llm, &msg);
            return Err(e);
        }
        Ok(Ok(m)) => m,
    };

    tracing::info!("Qwen3-4B-Instruct-2507 ready");
    emit_ready(emitter, Engine::Llm);
    Ok(model)
}

// -- Streaming generation ----------------------------------------------------

/// Run one chat turn: build the prompt from `history`, tokenize, decode, and
/// stream detokenized pieces to `tx` as they come out.
///
/// Errors come back as a plain `String` so `api::chat::drive_backend` can map
/// them to the outgoing OpenAI-compatible error shape without pulling llama
/// types into the HTTP layer.
pub async fn stream(
    model: &Arc<LlamaModel>,
    history: Vec<ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    tx: mpsc::Sender<String>,
) -> std::result::Result<(), String> {
    let chat_messages: Vec<LlamaChatMessage> = history
        .iter()
        .filter_map(|m| {
            let role = match m.role.as_str() {
                "system" | "user" | "assistant" => m.role.clone(),
                other => {
                    tracing::warn!(
                        role = %other,
                        "unrecognized chat role; defaulting to user — check the client"
                    );
                    "user".to_string()
                }
            };
            LlamaChatMessage::new(role, m.content.clone()).ok()
        })
        .collect();
    if chat_messages.is_empty() {
        return Err("no chat messages to render".into());
    }

    let model = model.clone();
    let temperature = temperature.unwrap_or(DEFAULT_TEMPERATURE);
    let max_tokens = max_tokens.unwrap_or(DEFAULT_MAX_NEW_TOKENS);

    // llama.cpp holds non-thread-safe C++ state on the decode context —
    // keep everything below on one blocking worker and send tokens back
    // through the mpsc using `blocking_send`.
    let res = tokio::task::spawn_blocking(move || run_chat(&model, chat_messages, temperature, max_tokens, &tx))
        .await
        .map_err(|e| format!("spawn_blocking llm worker join: {e}"))?;
    res
}

fn run_chat(
    model: &LlamaModel,
    messages: Vec<LlamaChatMessage>,
    temperature: f32,
    max_new_tokens: u32,
    tx: &mpsc::Sender<String>,
) -> std::result::Result<(), String> {
    let backend = backend().map_err(|e| format!("{e:#}"))?;
    let template = model
        .chat_template(None)
        .unwrap_or_else(|_| LlamaChatTemplate::new("chatml").expect("chatml fallback template"));
    // Use the `_with_tools_oaicompat` variant even without tools — it routes
    // through llama.cpp's Jinja engine, which handles Gemma 4's embedded
    // template. The plain `apply_chat_template` uses the old legacy path
    // that returns ffi error -1 on Gemma-style templates.
    let rendered = model
        .apply_chat_template_with_tools_oaicompat(
            &template,
            &messages,
            None, // no tool definitions
            None, // no JSON schema constraint
            true, // add assistant turn marker
        )
        .map_err(|e| format!("apply_chat_template_with_tools_oaicompat: {e}"))?;
    let prompt = rendered.prompt;

    let tokens = model
        .str_to_token(&prompt, AddBos::Always)
        .map_err(|e| format!("str_to_token: {e}"))?;

    let prompt_len = tokens.len() as u32;
    let ctx_size = (prompt_len + max_new_tokens + 32)
        .max(DEFAULT_CONTEXT_TOKENS)
        .min(32_768);

    // Disable Flash Attention on context creation: with it on (the default
    // "auto" policy), llama.cpp then resolves Fused Gated Delta Net support
    // (PR ggml-org/llama.cpp#17869) and trips an internal GGML_ASSERT on
    // Gemma 4's tensor naming scheme. Gemma 4 has no FGDN layers, so the
    // assert is spurious but fatal. FA's main win is long-context speed;
    // on an 8 k context with our short turns the loss is imperceptible.
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(ctx_size))
        .with_flash_attention_policy(llama_cpp_sys_2::LLAMA_FLASH_ATTN_TYPE_DISABLED);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("new_context: {e}"))?;

    // Feed the whole prompt, ask llama.cpp to produce logits for the final
    // token only (that's what we'll sample from).
    let mut batch = LlamaBatch::new(prompt_len as usize, 1);
    let last_index = prompt_len.saturating_sub(1) as i32;
    for (i, token) in tokens.iter().enumerate() {
        batch
            .add(*token, i as i32, &[0], i as i32 == last_index)
            .map_err(|e| format!("batch.add prompt token: {e}"))?;
    }
    ctx.decode(&mut batch)
        .map_err(|e| format!("ctx.decode prompt: {e}"))?;

    // Sampler: a tiny chain that does temperature + top-p + final sampling.
    // Seed is fixed for reproducibility during dev; worth threading through
    // req.seed once we expose it.
    let seed = 0xA5_A5_A5_A5_u32;
    let top_p = 0.95;
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::temp(temperature),
        LlamaSampler::top_p(top_p, /* min_keep */ 1),
        LlamaSampler::dist(seed),
    ]);

    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut produced: u32 = 0;
    let mut n_cur = batch.n_tokens();

    while produced < max_new_tokens {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        let piece = model
            .token_to_piece(token, &mut decoder, true, None)
            .map_err(|e| format!("token_to_piece: {e}"))?;
        if tx.blocking_send(piece).is_err() {
            // Client went away; no point continuing.
            return Ok(());
        }

        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| format!("batch.add next token: {e}"))?;
        ctx.decode(&mut batch)
            .map_err(|e| format!("ctx.decode next token: {e}"))?;
        n_cur += 1;
        produced += 1;
    }

    Ok(())
}
