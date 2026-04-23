# Workspace crates

protoApp is a Cargo workspace with three members, plus the React
frontend living alongside.

## Layout

```
protoApp/
├── Cargo.toml                       # workspace root
├── Cargo.lock
├── src-tauri/                       # workspace member — Tauri wrapper
├── crates/
│   ├── protolabs-voice-core/        # workspace member — engine substrate
│   └── orbis-sidecar/               # workspace member — Python sidecar plumbing
├── src/                             # React 19 + Vite 7 + shadcn frontend
└── docs/                            # this directory
```

## `protoApp` — `src-tauri/`

The Tauri shell. Thin by design — owns:
- Tauri lifecycle (setup, commands, state)
- The tokio runtime that hosts the HTTP server
- Tauri-specific commands (`get_api_base_url`, `greet`)
- tauri-specta TypeScript binding generation for `src/bindings.ts`

Depends on `protolabs-voice-core` via a path dep.

## `protolabs-voice-core` — `crates/protolabs-voice-core/`

The OpenAI-compatible router + engine wrappers. Shareable with any
Rust host — not coupled to Tauri.

Public surface:

```rust
pub use api::{bind, bind_with_state, router};
pub use api::state::AppState;
```

- `bind()` — binds on `127.0.0.1:0`, returns `(SocketAddr, future)`
- `bind_with_state(Arc<AppState>)` — same but you own the state (useful if Tauri commands want to preload the LLM, or a voice pipeline wants to call engines in-process without the HTTP hop)
- `router(Arc<AppState>) -> axum::Router` — compose into a larger axum app

Internal:
- `api::{chat, models, speech, state, transcriptions}` — endpoint modules
- `engines::llm` — mistralrs wrapper, feature-gated behind `llm`

## `orbis-sidecar` — `crates/orbis-sidecar/`

Spawn + WebSocket client for running the ORBIS Python agent as a
Tauri sidecar.

Public surface:

```rust
pub use client::Client;
pub use protocol::{IncomingMessage, OutgoingMessage};
pub use spawn::{Sidecar, SpawnConfig, SpawnError};
```

- `Sidecar::spawn(SpawnConfig)` — launches, waits for readiness line, manages lifecycle
- `Sidecar::connect()` — opens a new WebSocket to the running sidecar
- `Sidecar::shutdown(grace)` — graceful(-ish) shutdown, escalating to SIGKILL
- `Client` — sends `OutgoingMessage`, receives `IncomingMessage`

See [orbis-sidecar protocol](./orbis-sidecar-protocol.md) for the wire
format.

## Why three crates?

- `protolabs-voice-core` is the reusable substrate — protoApp, a future orbis-tauri, a headless CLI can all embed it.
- `orbis-sidecar` has zero engine logic; it's pure IPC plumbing. Keeping it separate avoids cross-contamination if we ever want voice without ORBIS or ORBIS without voice.
- `protoapp` is the thinnest possible Tauri wrapper that demonstrates both crates in one place.

If this grows to a fourth crate, it'll be for a shared audio-IO layer
(cpal + VAD + echo cancellation) that belongs outside voice-core.
