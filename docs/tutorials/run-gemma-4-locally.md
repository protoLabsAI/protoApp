# Run Gemma 4 locally

You've [finished Getting started](./getting-started.md) and seen the stub
reply. Now we'll turn on the `llm` feature so the same endpoint serves
real Gemma 4 E2B inference via mistralrs.

## What you'll build

The chat UI unchanged, but replies come from **Gemma 4 E2B** downloaded
from Hugging Face (≈1.5 GB on disk, ~2 GB VRAM at runtime) and run
locally on your GPU (Metal on Apple Silicon, CUDA on NVIDIA).

## 1. Budget your time

| Step | Time |
|---|---|
| Enable Metal/CUDA and rebuild | 10–15 min (first time) |
| First request (downloads weights) | 2–5 min on a fast connection |
| Subsequent cold starts | ~5 s |
| Per-token latency on M3 Max | ~12 ms |

If you don't have the patience for the first build, stay on the stub
for now. There's no regret — the interface is identical.

## 2. Build with your GPU backend

**Apple Silicon**

```sh
pnpm tauri dev -- --features llm,metal
```

**NVIDIA (Linux / Windows with CUDA toolkit)**

```sh
pnpm tauri dev -- --features llm,cuda
```

**CPU-only** (don't — it's unusable for 4 B params, but it compiles)

```sh
pnpm tauri dev -- --features llm
```

The first compile is long because mistralrs pulls in a large dependency
tree (candle, hf-hub, mcp, the compression stack). Subsequent rebuilds
are fast.

## 3. First request

Send any message. The Rust side logs:

```
INFO Loading Gemma 4 E2B (unsloth GGUF Q4_K_M) — first run downloads ~1.5 GB
INFO Gemma 4 E2B ready
```

On Linux/macOS, the weights land in `~/.cache/huggingface/hub/` —
shared with every other HF-ecosystem tool on the box.

## 4. Notice what changes

- Replies are real now, not echoes.
- The first request is slow (model load), every subsequent one is fast.

> Function calling: the model supports it, but the `/v1/chat/completions`
> streaming handler in `engines/llm.rs` currently forwards only
> `delta.content`, not `tool_calls` deltas. Tracked as a follow-up.

## Troubleshooting

**"mistralrs failed to compile"** → make sure you have enough RAM and disk; link-time is where it fails usually.

**"Download stalls"** → `huggingface-cli download unsloth/gemma-4-E2B-it-GGUF gemma-4-E2B-it-Q4_K_M.gguf` in advance; the Tauri app will pick it up from the shared cache.

**Out of GPU memory** → E2B uses ~2 GB VRAM at Q4_K_M. If you have less, stay on CPU+stub for now or swap to a smaller GGUF via the steps in [Swap the default LLM](../how-to/swap-llm-model.md).

## Next

- [Swap the default LLM](../how-to/swap-llm-model.md) — point at a different GGUF repo.
- [Why Gemma 4 E2B](../explanation/engine-choices.md#llm-gemma-4-e2b) — the model-selection rationale.
