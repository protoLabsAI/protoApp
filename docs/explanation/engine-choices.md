# Engine choices

The LLM/STT/TTS landscape in April 2026 is crowded. Here's the
reasoning behind each pick and the serious alternatives we evaluated.

## LLM: Qwen3-4B-Instruct-2507 via llama-cpp-2

**Picked**: [Qwen3-4B-Instruct-2507](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507),
quantized GGUF Q4_K_M from
[unsloth/Qwen3-4B-Instruct-2507-GGUF](https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF),
run via [`llama-cpp-2`](https://crates.io/crates/llama-cpp-2) 0.1.143
(Rust bindings to llama.cpp).

**Why this model**

- Apache-2.0 license; redistributable without special terms.
- Classic attention transformer — no gated-delta tensors, so it doesn't trip llama.cpp's FGDN assertion (which blocks Gemma 4 and Qwen3.5, see below).
- ~2.5 GB at Q4_K_M, ~3 GB VRAM at runtime; comfortable on a modern laptop.
- Tool-use trained; the streaming handler in `engines/llm.rs` currently forwards only `delta.content` (tracked as a follow-up to surface `tool_calls` deltas).

**Why llama-cpp-2 specifically**

- Thin Rust binding around llama.cpp's mature C++ core — fast CPU inference via tight SIMD/BLAS, first-class Metal on Apple Silicon, CUDA on NVIDIA.
- Small dep tree compared to other Rust LLM stacks — cold compile is a few minutes instead of 10–15.
- We own the HTTP surface ourselves (axum in `protolabs-voice-core`), which is fine: the OpenAI-compat endpoints are a few hundred lines of handlers.

### Why not Gemma 4 (what we originally wanted)

Gemma 4 E2B/E4B was the plan: function calling, vision input,
MatFormer's small runtime footprint. Two successive blockers killed it
for v1:

**mistralrs path (tried first)** — three separate Gemma 4 bugs:
1. Missing GGUF arch enum for `gemma4` — upstream [#2098](https://github.com/EricLBuehler/mistral.rs/issues/2098).
2. `Gemma4ForConditionalGeneration` rejected by `TextModelBuilder` because it's not a CausalLM class.
3. serde duplicate-field error on `expert_intermediate_size` via `ModelBuilder` — upstream [#2119](https://github.com/EricLBuehler/mistral.rs/issues/2119), filed by us. No fix on master either.

**llama-cpp-2 path (pivoted to)** — llama.cpp's Fused Gated Delta
Network code asserts on Gemma 4's tensor naming:
```
GGML_ASSERT(strncmp(n->name, LLAMA_TENSOR_NAME_FGDN_AR "-", prefix_len) == 0) failed
```
(src/llama-context.cpp:485 on 0.1.143, line 487 on 0.1.145.) Weights
load successfully — first inference aborts. Qwen3.5-4B trips the same
assert because it also uses gated delta networks.

The full repro log, attempts, and unblock criteria live in
[STATUS.md](../../STATUS.md#gemma-4-blocked-by-llamacpp-fgdn-tensor-name-assert).
When upstream llama.cpp accepts Gemma 4's tensor names, the swap back
is a one-liner in `engines/llm.rs`.

**Alternatives we evaluated**

| Alternative | Why we didn't pick it |
|---|---|
| `mistralrs` 0.8 | Had three Gemma 4 bugs (see above); fine for other models but we wanted Gemma 4 as the default. |
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
