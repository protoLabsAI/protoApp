# protoApp

A Tauri 2 desktop app that runs an **OpenAI-compatible local HTTP server
inside its own Rust process**. The React frontend talks to it with the
standard `openai` npm package — same SDK, different `baseURL`.

Backed by in-process Rust inference (llama-cpp-2 + Qwen3-4B-Instruct-2507,
whisper-rs, kokoros). Ships as a Cargo workspace so a second app,
`orbis-tauri`, can vendor the same engine substrate and add a Python
agent sidecar.

> Gemma 4 was the original target but is currently blocked by an
> upstream llama.cpp FGDN bug — see [STATUS.md](./STATUS.md) and
> [HANDOFF.md](./HANDOFF.md) for the full story and the one-line swap
> back.

## Status

| Area | State |
|---|---|
| OpenAI `/v1/*` surface (models, chat, transcriptions, speech, healthz) | shipped, 11/11 tests green |
| Streaming SSE chat UI | shipped |
| Real chat via Qwen3-4B-Instruct-2507 behind `--features llm` | shipped |
| Metal + CUDA feature flags | shipped |
| Real whisper-rs STT | shipped behind `--features stt` (requires `brew install cmake` once) |
| Real Kokoro TTS | shipped behind `--features tts` (via `kokoros` git dep) |
| ORBIS Python sidecar plumbing | shipped as `orbis-sidecar`; awaits ORBIS WS entry point ([#12](./docs/how-to/integrate-orbis-sidecar.md)) |
| React voice panels (mic / TTS playback) | shipped as Transcribe + Speak tabs; work against stubs today, pick up real engines automatically |
| Gemma 4 as default | ⏳ blocked on upstream llama.cpp ([STATUS.md](./STATUS.md)) |

## Quick start

```sh
pnpm install
pnpm tauri dev
```

Starts with stub engines for fast iteration. Add `--features llm,metal`
(or `,cuda`) to bring up the real LLM — see [Run a local LLM](./docs/tutorials/run-local-llm.md).
With `--features engines,metal` you get the full stack: Qwen3-4B-Instruct-2507,
Whisper, Kokoro TTS. First chat/transcribe/speak each trigger a
one-time model download (~2.5 GB / ~60 MB / ~340 MB) cached under
`~/.cache/protoapp/`.

Without the `metal` feature, LLM inference runs on CPU at ~2–5 tok/s —
noticeably slow. Build with GPU acceleration to get ~60 tok/s on
M-series:

```sh
pnpm tauri dev -- --features llm,metal
```

(Apple Silicon. Use `llm,cuda` on NVIDIA.)

See [docs/tutorials/getting-started.md](./docs/tutorials/getting-started.md)
for the full walkthrough.

## Architecture

```
┌─────────── Tauri process (Rust) ───────────┐
│                                             │
│  React UI  ←→  @tauri-apps/api              │
│       ↓                                     │
│  openai JS SDK (baseURL = localhost:<port>) │
│       ↓                                     │
│  axum server ←— protolabs-voice-core        │
│    │  /v1/models                            │
│    │  /v1/chat/completions  ← llama-cpp-2   │
│    │  /v1/audio/transcriptions ← whisper-rs │
│    │  /v1/audio/speech ← kokoros            │
│    │                                        │
│    └── orbis-sidecar (spawn + WebSocket)    │
└────────────────────┬────────────────────────┘
                     │
              ┌──────▼────────────────┐
              │ ORBIS Python sidecar  │
              │ (agent, a2a, memory)  │
              └───────────────────────┘
```

Workspace members:

- [`src-tauri/`](./src-tauri/) — Tauri shell (commands, lifecycle, window)
- [`crates/protolabs-voice-core/`](./crates/protolabs-voice-core/) — OpenAI-compatible router + engine wrappers
- [`crates/orbis-sidecar/`](./crates/orbis-sidecar/) — Python sidecar spawner + WebSocket client

Details in [docs/reference/workspace-crates.md](./docs/reference/workspace-crates.md)
and [docs/explanation/architecture.md](./docs/explanation/architecture.md).

## Tech stack

Frontend: React 19 · Vite 7 · shadcn/ui · Zustand · TanStack Query · Biome

Backend: Tauri 2 · tokio · axum 0.7 · llama-cpp-2 0.1.143 · whisper-rs 0.16 · kokoros · tokio-tungstenite 0.24

Tooling: Cargo workspace · pnpm · tauri-specta (typed Tauri commands)

## Documentation

`./docs/` follows the [Diátaxis](https://diataxis.fr) framework. Start at
[docs/README.md](./docs/README.md).

| Path | Purpose |
|---|---|
| [`docs/tutorials/`](./docs/tutorials/) | learn by doing |
| [`docs/how-to/`](./docs/how-to/) | solve a specific task |
| [`docs/reference/`](./docs/reference/) | look up a fact |
| [`docs/explanation/`](./docs/explanation/) | understand the why |

## Commands

```sh
pnpm tauri dev                                    # run in dev mode (stub)
pnpm tauri dev -- --features llm,metal            # ...with real Qwen3-4B on Apple Silicon
pnpm tauri build -- --features engines,metal      # release build, all engines
cargo test --workspace                            # 11 tests across voice-core + sidecar
pnpm typecheck                                    # TS
pnpm lint                                         # Biome
```

## Further reading

- [STATUS.md](./STATUS.md) — what's shipped, what's blocked, and why.
- [HANDOFF.md](./HANDOFF.md) — the three non-obvious things a new contributor needs to know.

## License

MIT or Apache-2.0, at your option. Model weights carry their own
licenses (Qwen3 under Apache-2.0, Whisper under MIT, Kokoro under
Apache-2.0) — see the model cards.
