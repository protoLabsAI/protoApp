# Workspace crates

protoApp is a Cargo workspace with three members, plus a vendored agent
runtime and the React frontend living alongside.

## Layout

```
protoApp/
├── Cargo.toml                       # workspace root
├── Cargo.lock
├── src-tauri/                       # workspace member — Tauri wrapper
├── crates/
│   ├── protolabs-voice-core/        # workspace member — engine substrate
│   └── protoapp-agent/              # workspace member — in-process zeroclaw bridge
├── vendor/
│   └── zeroclaw/                    # git submodule — the agent runtime (own workspace)
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
- `engines::llm` — llama-cpp-2 wrapper loading Qwen3-4B-Instruct-2507, feature-gated behind `llm`

## `protoapp-agent` — `crates/protoapp-agent/`

The in-process agent bridge. Embeds `zeroclaw-runtime` (from the
`vendor/zeroclaw` submodule) and drives one agent turn per call. Replaces
the old `orbis-sidecar` — iOS can't spawn an external interpreter, so the
agent runs inside the host process rather than as a Python sidecar.

Public surface:

```rust
pub async fn ask(base_url: &str, model: &str, message: &str, session_id: Option<&str>)
    -> anyhow::Result<String>;
```

- Renders a zeroclaw `config.toml` wiring a `default` agent to voice-core's
  local OpenAI-compatible server (`{base_url}/v1`), then calls
  `zeroclaw_runtime::agent::process_message`.
- Depends on `zeroclaw-runtime` + `zeroclaw-config` by path
  (`default-features = false`); `vendor/zeroclaw` is its own cargo
  workspace, excluded from protoApp's.

See [the how-to](../how-to/use-the-in-process-agent.md) for usage and the
generated config.

## Why these crates?

- `protolabs-voice-core` is the reusable substrate — protoApp, a future orbis-tauri, a headless CLI can all embed it.
- `protoapp-agent` keeps the zeroclaw embedding (and its large dep tree) isolated behind a tiny `ask()` surface, so the Tauri shell and voice-core stay agent-agnostic.
- `protoApp` is the thinnest possible Tauri wrapper that wires them together.

If this grows another crate, it'll likely be a shared audio-IO layer
(cpal + VAD + echo cancellation) that belongs outside voice-core.
