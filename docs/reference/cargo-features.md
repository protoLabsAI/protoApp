# Cargo features

All defined in `crates/protolabs-voice-core/Cargo.toml` and mirrored at
the `protoapp` level so `cargo tauri build --features …` propagates
through the workspace.

## Engine features

| Feature | Pulls in | Status |
|---|---|---|
| `llm` | `mistralrs = "0.8"` | Real — Gemma 4 E2B GGUF via `GgufModelBuilder`. |
| `stt` | `whisper-rs = "0.16"` | Endpoint scaffolded; real model wiring pending. Requires `cmake` on the build host. |
| `tts` | *none currently* | Placeholder — stub emits silent WAV. `tts-rs` has an upstream `ort` compile break; we'll wire in kokoros or direct ort when fixed. |
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
