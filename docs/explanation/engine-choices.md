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

**Shipped** behind `--features stt`:

- Frontend records via `MediaRecorder`, then `AudioContext.decodeAudioData` + our `encodeMono16kWav` helper (`src/lib/wav.ts`) emit 16 kHz mono PCM16 WAV. The server never touches an audio codec.
- Server: `hound` parses the WAV, `whisper-rs` 0.16 runs it against the cached `ggml-base.en-q5_1.bin` model (auto-downloaded into `~/.cache/protoapp/whisper/`).
- Build prerequisite: `brew install cmake` (whisper.cpp vendors C++ that needs a cmake configure step).
- Override `PROTOAPP_WHISPER_MODEL_PATH` to point at a different GGML model.

## TTS: Kokoro-82M (pending)

**Picked (aspirationally)**:
[Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M) — 82 M-param
neural TTS, Apache-2.0, 50+ voices across 9 languages, ~10 s of audio
synthesized in ~1 s on WebGPU and similar on CPU via ONNX Runtime.

**Shipped** behind `--features tts`:

- Implementation: [`lucasjinreal/Kokoros`](https://github.com/lucasjinreal/Kokoros) via a pinned git dep. Exports `TTSKoko::tts_raw_audio(txt, lan, voice, speed, ...)` returning `Vec<f32>` at 24 kHz, which we wrap in a PCM16 WAV with `hound` before handing to the client.
- Engine is lazily initialized once per process; Kokoros handles the model + voice-pack downloads on first use into `~/.cache/protoapp/kokoro/`. Override paths with `PROTOAPP_KOKORO_{MODEL,VOICES}_PATH`.
- Build prerequisites: `brew install cmake` (pulled in transitively by `audiopus_sys`), plus the workspace-level `CMAKE_POLICY_VERSION_MINIMUM=3.5` in `.cargo/config.toml` so CMake 4 doesn't refuse the vendored C projects that still target CMake ≤ 3.4.

Why not the crates.io `tts-rs` crate we originally wanted: `tts-rs 2026.2.x` pins an older `ort` rc, but cargo resolves a newer one in workspace context and that tripped a generic-parameter mismatch on `ort::Error<SessionBuilder>`. Tracked upstream at
[rishiskhare/tts-rs#1](https://github.com/rishiskhare/tts-rs/issues/1).
Kokoros handles the phonemizer + ONNX stack directly, so it sidesteps the issue.

**Future improvements worth considering**

1. Hand-roll ORT + Kokoro ONNX inside voice-core for tighter control over streaming chunk boundaries.
2. Swap back to a published `tts-rs` once the upstream `ort` generic issue is resolved — the git dep is simpler than keeping our own wrapper.
3. Add mp3 transcoding (see also the advisory `x-protoapp-note` header on the `/v1/audio/speech` response when `response_format=mp3`).

Without the `tts` feature the endpoint still returns a valid (silent, PCM16)
WAV so any client relying on that contract keeps working.

**Alternative we rejected for v1**: [Piper](https://github.com/rhasspy/piper)
via `piper-rs`. Faster on low-end CPUs, many languages, but quality
gap vs Kokoro is audible and we'd rather default to the better voice
and let it lag on ancient machines.
