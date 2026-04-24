# Run a local LLM

You've [finished Getting started](./getting-started.md) and seen the
stub reply. Now we'll turn on the `llm` feature so the same endpoint
serves real inference — by default **Qwen3-4B-Instruct-2507** via
`llama-cpp-2` (Rust bindings to llama.cpp).

> **Why not Gemma 4?** Gemma 4 is what we originally targeted (function
> calling, vision-capable), but llama.cpp's FGDN tensor-name check
> asserts on Gemma 4 and crashes the process on first inference. Full
> story in [STATUS.md](../../STATUS.md#gemma-4-blocked-by-llamacpp-fgdn-tensor-name-assert).
> Qwen3-4B-Instruct-2507 is a classic attention transformer with no
> gated-delta tensors, so it sidesteps the bug entirely and streams
> reliably.

## What you'll build

The chat UI unchanged, but replies come from
**Qwen3-4B-Instruct-2507** (Apache-2.0, ~2.5 GB at Q4_K_M, downloaded
from [unsloth/Qwen3-4B-Instruct-2507-GGUF](https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF))
running locally on your GPU (Metal on Apple Silicon, CUDA on NVIDIA).

## 1. Budget your time

| Step | Time |
|---|---|
| Enable Metal/CUDA and rebuild | 5–10 min (first time, llama.cpp is a small dep tree) |
| First request (downloads weights) | 2–5 min on a fast connection |
| Subsequent cold starts | ~3 s |
| Per-token latency on M3 Max | ~12 ms (Metal), ~200 ms (CPU) |

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

**CPU-only** (slow but works without a GPU toolchain)

```sh
pnpm tauri dev -- --features llm
```

The build is fast because `llama-cpp-2` pulls in a thin C++ dep tree
compared to the mistralrs stack we used to depend on.

## 3. First request

Send any message. The Rust side logs:

```
INFO Loading Qwen3-4B-Instruct-2507 (GGUF Q4_K_M via llama.cpp) — first run downloads ~2.5 GB
INFO Qwen3-4B-Instruct-2507 ready
```

The weights land in `~/.cache/protoapp/llm/` — separate from other
HF-ecosystem caches because we stream the download ourselves (so the
engine-status banner in the UI gets live byte-progress events).

## 4. Notice what changes

- Replies are real, not echoes.
- The first request is slow (model load). Every subsequent one is fast.
- The engine banner at the top of the Chat tab shows **loading**,
  **downloading** (with a progress bar), **ready**, or **error**
  states, driven by a Tauri event bus defined in
  `crates/protolabs-voice-core/src/engines/events.rs`.

> Function calling: Qwen3-4B-Instruct-2507 supports tool use, but the
> streaming handler in `engines/llm.rs` currently forwards only
> `delta.content`, not `tool_calls` deltas. Tracked as a follow-up.

## Troubleshooting

**"llama-cpp-sys-2 failed to build"** → ensure you have a working C++
compiler. On macOS, Command Line Tools is enough (`xcode-select --install`).
On Linux, `build-essential` + `cmake`.

**"Download stalls"** → Hugging Face can rate-limit anonymous IPs.
`huggingface-cli download unsloth/Qwen3-4B-Instruct-2507-GGUF Qwen3-4B-Instruct-2507-Q4_K_M.gguf --local-dir ~/.cache/protoapp/llm/`
in advance and the Tauri app will use the cached file.

**"Out of GPU memory"** → Qwen3-4B at Q4_K_M uses ~3 GB VRAM on Metal.
If you have less, drop to CPU (`--features llm` without `,metal`) or
swap to a smaller GGUF via [Swap the default LLM](../how-to/swap-llm-model.md).

**`GGML_ASSERT` on `LLAMA_TENSOR_NAME_FGDN_AR`** → you've pointed at a
model that uses gated delta networks (Gemma 4, Qwen3.5). See
[STATUS.md](../../STATUS.md#gemma-4-blocked-by-llamacpp-fgdn-tensor-name-assert).
Revert to Qwen3-4B-Instruct-2507 or another classic-attention model.

## Next

- [Swap the default LLM](../how-to/swap-llm-model.md) — point at a different GGUF repo.
- [Why Qwen3-4B-Instruct-2507](../explanation/engine-choices.md#llm-qwen3-4b-instruct-2507-via-llama-cpp-2) — the model-selection rationale.
