# Engine choices

The LLM/STT/TTS landscape in April 2026 is crowded. Here's the
reasoning behind each pick and the serious alternatives we evaluated.

## LLM: Gemma 4 E2B via mistralrs

**Picked**: Google's [Gemma 4 E2B](https://huggingface.co/google/gemma-4-E2B-it),
quantized GGUF Q4_K_M from
[unsloth/gemma-4-E2B-it-GGUF](https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF),
run via [mistralrs](https://github.com/EricLBuehler/mistral.rs) 0.8.

**Why this model**

- Released 2026-04-02; 3 weeks old at time of writing.
- The model is natively trained for function calling. **Wiring status**: our streaming handler in `crates/protolabs-voice-core/src/engines/llm.rs` currently forwards only `delta.content`; `tool_calls` deltas are **not yet** surfaced on `/v1/chat/completions`. The capability is there at the model level, but our pipeline is still pending — tracked in the README roadmap.
- Vision + audio input (we don't use audio input in v1 — whisper-rs is faster and more predictable).
- MatFormer architecture: "E2B" means ~2 B **effective** params from a deeper base, so the runtime footprint is ~2 GB at Q4_K_M — installable on a user's laptop.

**Why mistralrs specifically**

- Ships an OpenAI-compatible API surface (`/v1/chat/completions`, `/v1/audio/transcriptions`, `/v1/audio/speech`, `/v1/embeddings`, `/v1/images/generations`) as both a standalone binary *and* an embeddable Rust library.
- Metal + CUDA + CPU with FlashAttention 2/3 and PagedAttention.
- Model format-agnostic: HuggingFace safetensors, GGUF, UQFF.
- MCP client built in (useful when we layer agent capabilities on top).
- Active, solo-maintained by Eric Buehler; release cadence has been weekly.

**Alternatives we evaluated**

| Alternative | Why we didn't pick it |
|---|---|
| `llama-cpp-2` (utilityai/llama_cpp-rs) | Fastest build times, smallest dep surface, but no built-in OpenAI shape — we'd be hand-rolling an Axum layer that mistralrs gives us for free. |
| `candle` | Great Rust ML primitives but LLM-level conveniences (tokenizers, chat templates, server) are not first-class. |
| Pure in-browser (WebLLM) | See [architecture](./architecture.md). Rejected on perf + UX grounds. |
| Moshi (kyutai-labs) | Different paradigm — full-duplex speech-to-speech foundation model. Great for a "casual mode" demo. Doesn't support function calling; not a drop-in. Reserved for a future parallel path. |

## STT: whisper-rs

**Picked**: [whisper-rs](https://github.com/tazz4843/whisper-rs) 0.16
against GGML weights.

**Why**

- Rust bindings to whisper.cpp — the battle-tested C++ implementation.
- Metal / CUDA / Vulkan / OpenBLAS backends. Apple Silicon acceleration is first-class.
- Default model `whisper-large-v3-turbo` Q5_0 is ~800 MB on disk, real-time factor ~0.05 on M-series.

**Alternatives**

- `Xenova/moonshine-base` — optimized for streaming / low latency. Compelling for real-time voice, but English-only and the quality gap vs Whisper turbo is real enough that we punt the trade-off until we're doing streaming pipelines in earnest.
- `sherpa-onnx` — single ONNX runtime for STT + TTS + VAD. Would let us drop one dep; costs us Metal on Whisper.

**Current status**: endpoint scaffolded with a stub. Real wiring needs
`brew install cmake` on the build host, plus a model-download helper.

## TTS: Kokoro-82M (pending)

**Picked (aspirationally)**:
[Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M) — 82 M-param
neural TTS, Apache-2.0, 50+ voices across 9 languages, ~10 s of audio
synthesized in ~1 s on WebGPU and similar on CPU via ONNX Runtime.

**Current blocker**: the crates.io crate
[`tts-rs`](https://crates.io/crates/tts-rs) fails to compile against
the `ort` release cargo resolves for it. We tested `2026.2.1`,
`2026.2.2`, and `2026.2.3`; all three fail the same way. The compile
error from rustc, abbreviated:

```
error[E0277]: `?` couldn't convert the error to `KokoroError`:
  the trait `From<ort::Error<SessionBuilder>>` is not implemented
  for `KokoroError`
  --> tts-rs/src/engines/kokoro/model.rs:292:44
```

ort moved `SessionBuilder` out of (or changed its position in) the
`Error<T>` generic in a post-rc.10 release; tts-rs's `?` calls against
the old signature break. Track the upstream issue at
[rishiskhare/tts-rs#1](https://github.com/rishiskhare/tts-rs/issues/1).

**Options we'll use when unblocked**

1. Switch to [`lucasjinreal/Kokoros`](https://github.com/lucasjinreal/Kokoros) as a git dep — mature implementation, not published to crates.io but that's a one-line `Cargo.toml` tweak.
2. Hand-roll ORT + Kokoro ONNX integration inside voice-core. Most work; also gives us the most control over streaming.
3. Wait for tts-rs upstream fix.

Today the endpoint returns a valid silent WAV so the frontend contract
holds while we wait.

**Alternative we rejected for v1**: [Piper](https://github.com/rhasspy/piper)
via `piper-rs`. Faster on low-end CPUs, many languages, but quality
gap vs Kokoro is audible and we'd rather default to the better voice
and let it lag on ancient machines.
