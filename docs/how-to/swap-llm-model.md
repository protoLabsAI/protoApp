# Swap the default LLM model

The default is Qwen3-4B-Instruct-2507. Here's how to point the engine
at a different GGUF without changing any code paths.

## 1. Pick a repo + file

`llama-cpp-2` accepts any GGUF on the Hugging Face hub that llama.cpp
understands. Models known to work with our current pin
(`llama-cpp-sys-2 = "=0.1.143"`):

| Use case | Repo | File |
|---|---|---|
| **Default** | `unsloth/Qwen3-4B-Instruct-2507-GGUF` | `Qwen3-4B-Instruct-2507-Q4_K_M.gguf` |
| Smaller / faster | `unsloth/SmolLM2-1.7B-Instruct-GGUF` | `SmolLM2-1.7B-Instruct-Q4_K_M.gguf` |
| Larger / stronger | `unsloth/Qwen2.5-7B-Instruct-GGUF` | `Qwen2.5-7B-Instruct-Q4_K_M.gguf` |

**Do NOT switch to a gated-delta-network model (Gemma 4, Qwen3.5)**
until upstream llama.cpp fixes the FGDN tensor-name assert — the
process will abort on first inference. See
[STATUS.md](../../STATUS.md#gemma-4-blocked-by-llamacpp-fgdn-tensor-name-assert).

## 2. Edit the loader

`crates/protolabs-voice-core/src/engines/llm.rs`:

```rust
const DEFAULT_MODEL_REPO: &str = "unsloth/Qwen3-4B-Instruct-2507-GGUF";
const DEFAULT_MODEL_FILE: &str = "Qwen3-4B-Instruct-2507-Q4_K_M.gguf";
```

Change the repo and filename. That's it.

## 3. Update the advertised model id

`crates/protolabs-voice-core/src/api/models.rs`:

```rust
pub fn default_models() -> Vec<LocalModel> {
    vec![
        LocalModel { id: "qwen3-4b-instruct-2507", owner: "qwen", kind: ModelKind::Chat },
        // ...
    ]
}
```

This is what `GET /v1/models` returns. Make the id match what the
frontend sends in `ChatRequest.model` (`src/hooks/use-chat.ts`) so the
picker UI doesn't desync.

## 4. Rebuild

Pick the right features for your platform:

```sh
# Apple Silicon (Metal)
cargo build -p protoapp --features llm,metal --release

# NVIDIA
cargo build -p protoapp --features llm,cuda --release

# CPU-only (slow, but no GPU toolchain needed)
cargo build -p protoapp --features llm --release
```

Our GGUF cache lives at `~/.cache/protoapp/llm/` — swapping downloads
the new model but keeps the old one, so you can A/B by flipping the
constant and rebuilding.

## Caveats

- **License**: check the model card. Qwen3 is Apache-2.0 (redistributable). Gemma models are under Google's Gemma Terms of Use.
- **VRAM / RAM**: at Q4_K_M, 4B params is ~3 GB, 7B is ~5 GB. Check before switching on low-end machines.
- **Tokenizer / chat template**: llama.cpp parses the embedded chat template out of the GGUF. Unsloth's repos have historically been the cleanest; if a model outputs garbled prompts, try a different quantizer.
