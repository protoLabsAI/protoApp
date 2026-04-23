# Cargo features

All defined in `crates/protolabs-voice-core/Cargo.toml` and mirrored at
the `protoapp` level so `cargo tauri build --features …` propagates
through the workspace.

## Engine features

| Feature | Pulls in | Status |
|---|---|---|
| `llm` | `mistralrs = "0.8"` | Real — Gemma 4 E2B GGUF via `GgufModelBuilder`. |
| `stt` | `whisper-rs = "0.16"` + `hound` | Real — Whisper via whisper.cpp. Requires `cmake` on the build host (`brew install cmake` on macOS). Auto-downloads `ggml-base.en-q5_1` (~60 MB) into `~/.cache/protoapp/whisper/` on first use. |
| `tts` | `kokoros` (git dep) + ORT runtime | Real — Kokoro-82M via ONNX. Requires `cmake` plus the [workspace `.cargo/config.toml`](../../.cargo/config.toml) env var `CMAKE_POLICY_VERSION_MINIMUM=3.5` (already set). Auto-downloads `kokoro-v1.0.onnx` + voices bin (~340 MB) into `~/.cache/protoapp/kokoro/` on first use. |
| `engines` | `llm` + `stt` + `tts` | Umbrella for "everything". |

## GPU backends

Composition-only features: they enable GPU support on whichever
engines are *also* pulled in via the feature flags above. Without an
engine feature, they're a no-op.

| Feature | What it enables |
|---|---|
| `metal` | `mistralrs?/metal`, `mistralrs?/accelerate`, `whisper-rs?/metal` |
| `cuda` | `mistralrs?/cuda`, `whisper-rs?/cuda` |

The `?` syntax means "only if the optional dep is already enabled
elsewhere." That's why `cargo build --features metal` alone compiles
cleanly with no GPU code — because no engine is enabled.

## Recommended combos

| Goal | Command |
|---|---|
| Fast dev loop, stubs only | `cargo build` |
| Real chat on Apple Silicon | `cargo build -p protoapp --features llm,metal --release` |
| Real chat on NVIDIA | `cargo build -p protoapp --features llm,cuda --release` |
| Chat + STT on Apple Silicon (after `brew install cmake`) | `cargo build -p protoapp --features llm,stt,metal --release` |
| Everything on NVIDIA | `cargo build -p protoapp --features engines,cuda --release` (FlashAttention 2 kernels are auto-selected by mistralrs when the GPU supports them; no dedicated feature flag) |

## Adding a new feature

Add both in `crates/protolabs-voice-core/Cargo.toml` and in
`src-tauri/Cargo.toml` (the Tauri-level definition proxies to the
voice-core feature so users don't need to know about the workspace
structure).
