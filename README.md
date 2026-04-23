# protoApp

A Tauri 2 desktop app that runs an **OpenAI-compatible local HTTP server
inside its own Rust process**. The React frontend talks to it with the
standard `openai` npm package — same SDK, different `baseURL`.

Backed by in-process Rust inference (mistralrs + Gemma 4 E2B, whisper-rs,
kokoros). Ships as a Cargo workspace so a second app, `orbis-tauri`, can
vendor the same engine substrate and add a Python agent sidecar.

## Status

| Area | State |
|---|---|
| OpenAI `/v1/*` surface (models, chat, transcriptions, speech, healthz) | shipped, 11/11 tests green |
| Streaming SSE chat UI | shipped |
| Real Gemma 4 E2B chat behind `--features llm` | shipped |
| Metal + CUDA feature flags | shipped |
| Real whisper-rs STT | shipped behind `--features stt` (requires `brew install cmake` once) |
| Real Kokoro TTS | shipped behind `--features tts` (via `kokoros` git dep) |
| ORBIS Python sidecar plumbing | shipped as `orbis-sidecar`; awaits ORBIS WS entry point ([#12](./docs/how-to/integrate-orbis-sidecar.md)) |
| React voice panels (mic / TTS playback) | shipped as Transcribe + Speak tabs; work against stubs today, pick up real engines automatically |

## Quick start

```sh
pnpm install
pnpm tauri dev
```

Opens the chat UI against a streaming stub — proves the full transport
without downloading any models. ~30 s cold compile.

Real Gemma 4 on Apple Silicon:

```sh
pnpm tauri dev -- --features llm,metal
```

First build is 10–15 min; first request downloads ~1.5 GB of weights.
Both cached afterwards.

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
│    │  /v1/chat/completions  ← mistralrs     │
│    │  /v1/audio/transcriptions ← whisper-rs │
│    │  /v1/audio/speech ← kokoros (pending)  │
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

Backend: Tauri 2 · tokio · axum 0.7 · mistralrs 0.8 · whisper-rs 0.16 · tokio-tungstenite 0.24

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
pnpm tauri dev -- --features llm,metal            # ...with real Gemma 4 on Apple Silicon
pnpm tauri build -- --features engines,metal      # release build, all engines
cargo test --workspace                            # 11 tests across voice-core + sidecar
pnpm typecheck                                    # TS
pnpm lint                                         # Biome
```

## License

MIT or Apache-2.0, at your option. Model weights carry their own
licenses (Gemma under Google's Gemma Terms of Use, Whisper under MIT,
Kokoro under Apache-2.0) — see the model cards.
