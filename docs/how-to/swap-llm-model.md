# Swap the default LLM model

The default is Gemma 4 E2B. Here's how to point the engine at a
different GGUF without changing any code paths.

## 1. Pick a repo + file

mistralrs accepts any GGUF on the Hugging Face hub. Confirmed working:

| Use case | Repo | File |
|---|---|---|
| Smaller / faster | `unsloth/SmolLM2-1.7B-Instruct-GGUF` | `SmolLM2-1.7B-Instruct-Q4_K_M.gguf` |
| Gemma 4 E4B (bigger, smarter) | `unsloth/gemma-4-E4B-it-GGUF` | `gemma-4-E4B-it-Q4_K_M.gguf` |
| Qwen 3 4B | `unsloth/Qwen3-4B-Instruct-2507-GGUF` | `Qwen3-4B-Instruct-2507-Q4_K_M.gguf` |

## 2. Edit the loader

`crates/protolabs-voice-core/src/engines/llm.rs`:

```rust
pub async fn load_default() -> Result<Model> {
    let model = GgufModelBuilder::new(
        "unsloth/gemma-4-E2B-it-GGUF",                 // ← repo
        vec!["gemma-4-E2B-it-Q4_K_M.gguf"],            // ← file
    )
    .with_logging()
    .build()
    .await?;
    Ok(model)
}
```

Change the repo and filename. That's it.

## 3. Update the advertised model id

`crates/protolabs-voice-core/src/api/models.rs`:

```rust
pub fn default_models() -> Vec<LocalModel> {
    vec![
        LocalModel { id: "gemma-4-e2b", owner: "google", kind: ModelKind::Chat },
        // ...
    ]
}
```

This is what `GET /v1/models` returns. Make the id match what the
frontend sends in `ChatRequest.model` so the picker UI doesn't desync.

## 4. Rebuild

```sh
cargo build -p protoapp --features llm,metal --release
```

mistralrs caches weights per-repo, so swapping downloads the new GGUF
but keeps the old one — safe to bounce back and forth for A/B.

## Caveats

- **License**: Gemma's weights are under Google's Gemma Terms of Use (not Apache). If you redistribute a bundled binary, download on first run rather than shipping the weights in the installer.
- **VRAM**: E4B at Q4_K_M needs ~4 GB; check before switching on low-end machines.
- **Tokenizer**: for non-unsloth GGUFs, some have a buggy chat template. Unsloth's repos have historically been the fastest to ship a clean template.
