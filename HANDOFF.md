# protoApp — handoff

_Last updated: 2026-04-23._

If you just inherited this repo, this file tells you the three things
that aren't obvious from the code alone.

## 1. Why the default LLM is Qwen3-4B-Instruct-2507, not Gemma 4

The app was designed around Gemma 4 (function calling + vision). We
can't ship it today because of an upstream bug in llama.cpp's FGDN
(Fused Gated Delta Network) path — it asserts on Gemma 4's tensor
naming and crashes the process on first inference. We confirmed this
on both `llama-cpp-sys-2 0.1.143` and `0.1.145`. Full reproduction and
the attempts we made are in [STATUS.md](./STATUS.md#gemma-4-blocked-by-llamacpp-fgdn-tensor-name-assert).

`Qwen3-4B-Instruct-2507` is a classic attention transformer — no
gated-delta tensors, never touches the FGDN path, streams tokens
reliably. When upstream llama.cpp accepts Gemma 4's tensor names,
swapping back is a one-liner (see the comment in
[`crates/protolabs-voice-core/src/engines/llm.rs`](./crates/protolabs-voice-core/src/engines/llm.rs)
at `DEFAULT_MODEL_REPO`).

## 2. The `=0.1.143` pin on `llama-cpp-2` is load-bearing

`Cargo.toml` pins `llama-cpp-2 = "=0.1.143"` and `llama-cpp-sys-2 = "=0.1.143"`.

Don't cargo-update through these. The comment in `Cargo.toml` explains
the history. The short version: 0.1.143 is the version where we
verified end-to-end streaming with Qwen3-4B-Instruct-2507 on
2026-04-23. 0.1.145 likely works too (Qwen3-Instruct doesn't use
FGDN), but we haven't retested, and the minor FGDN-code churn between
0.1.143 and 0.1.145 shouldn't slip into someone else's runtime without
the corresponding manual test. If you want to bump, run a full chat
request end-to-end first.

## 3. Three engines, three feature flags, independent GPU backend

```sh
pnpm tauri dev                                  # stub everything (fast compile)
pnpm tauri dev -- --features llm,metal          # real chat on Apple Silicon
pnpm tauri dev -- --features engines,metal      # LLM + STT + TTS
cargo build -p protoapp --features engines,cuda --release  # NVIDIA, release
```

The `llm`, `stt`, and `tts` features are independent. `metal` and
`cuda` are composition-only — they enable GPU kernels *on whichever
engines are already feature-enabled*. `--features metal` alone is a
no-op, by design.

Detail: [docs/reference/cargo-features.md](./docs/reference/cargo-features.md).

## Everything else

The code is commented where the *why* isn't obvious. Start at:

- [README.md](./README.md) — project overview, quick start.
- [docs/README.md](./docs/README.md) — Diátaxis-organized docs index.
- [docs/tutorials/getting-started.md](./docs/tutorials/getting-started.md) — full walkthrough with the stub build.
- [docs/tutorials/run-local-llm.md](./docs/tutorials/run-local-llm.md) — enabling real inference.
- [docs/explanation/engine-choices.md](./docs/explanation/engine-choices.md) — why these specific engines.

Two open follow-ups in the in-repo task tracker:

- **#12** — wire ORBIS Python into `orbis-sidecar`.
- **#21** — preload engines on user demand so the first chat doesn't block on a 2.5 GB download.

Both are green-field, not blocking what's here.
