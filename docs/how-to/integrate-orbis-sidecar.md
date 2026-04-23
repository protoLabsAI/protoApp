# Integrate ORBIS as a Python sidecar

The `orbis-sidecar` crate gives you the Rust half: a process spawner
that owns the child, parses a readiness line, and exposes a typed
WebSocket client. Here's how to wire it up.

## 1. On the Python side

Your sidecar binary (a PyApp-bundled ORBIS, or `python -m orbis`) must:

1. Bind a WebSocket server on `127.0.0.1:<ephemeral port>`.
2. Print **exactly one line** to stdout: `ORBIS_READY ws://127.0.0.1:<port>/<path>` (any path works).
3. Stay in the foreground until it receives SIGKILL or a graceful shutdown message.
4. Speak the [protocol](../reference/orbis-sidecar-protocol.md) — minimally, accept `{"type":"user","text":"..."}` and reply with zero or more `{"type":"token","text":"..."}` followed by a `{"type":"turn_end"}`.

That's the full contract. Nothing else is mandatory.

## 2. In your Rust host

```rust
use orbis_sidecar::{Sidecar, SpawnConfig, OutgoingMessage, IncomingMessage};
use std::time::Duration;

let sidecar = Sidecar::spawn(SpawnConfig {
    program: "/Applications/protoApp.app/Contents/Resources/orbis".into(),
    args: vec![],
    readiness_timeout: Duration::from_secs(30),
    env: vec![("ORBIS_LOG_LEVEL".into(), "info".into())],
}).await?;

let mut client = sidecar.connect().await?;
client.send(OutgoingMessage::User { text: "hello".into() }).await?;

while let Some(msg) = client.next().await {
    match msg? {
        IncomingMessage::Token { text } => { /* feed to TTS */ }
        IncomingMessage::TurnEnd { .. } => break,
        IncomingMessage::Error { message } => tracing::error!(%message),
        _ => {}
    }
}
```

## 3. Tauri lifecycle

Tie `sidecar.shutdown(grace)` to your Tauri `RunEvent::ExitRequested`:

```rust
tauri::Builder::default()
    .setup(|app| {
        let sidecar = tauri::async_runtime::block_on(
            Sidecar::spawn(SpawnConfig::default())
        )?;
        app.manage(sidecar);
        Ok(())
    })
    .on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            let sidecar = window.state::<Sidecar>();
            tauri::async_runtime::block_on(
                sidecar.shutdown(std::time::Duration::from_secs(2))
            );
        }
    })
    .run(ctx)
```

## 4. Bundle the Python binary

Build with [PyApp](https://github.com/ofek/pyapp) — it gives you a
Rust-launched native binary with a portable Python runtime inside,
cleaner macOS code-signing than PyInstaller, and a smaller `.app`.

Then drop the binary under `src-tauri/binaries/` and register in
`tauri.conf.json`:

```json
{
  "bundle": {
    "externalBin": ["binaries/orbis"]
  }
}
```

Tauri will resolve `orbis-aarch64-apple-darwin`,
`orbis-x86_64-pc-windows-msvc.exe`, etc. based on the build target.

## Caveats

- **Code signing on macOS** — PyApp handles per-`.so` signing better than PyInstaller; still expect to spend a few hours on the first notarization cycle.
- **Cold start** — Python + ORBIS imports take 2–4 s. Spawn the sidecar during Tauri setup, not on first message, so it's warm when the user types.
- **Back-pressure** — the sidecar shouldn't emit tokens faster than the WebSocket drains. If you see lag, add a `tokio::sync::mpsc` bounded channel on the Python side.
- **Graceful shutdown** — `shutdown(grace)` currently SIGKILLs and waits `grace`. Add a `Shutdown` variant to `OutgoingMessage` and have the Python side respond by exiting cleanly if you need to flush state.

## What ORBIS still needs before this works end-to-end

- Document the entry point (currently the README is 404 on the default branch).
- Implement the readiness stdout line.
- Implement the WebSocket server + protocol.

See the [follow-up task](../../README.md#roadmap) for progress.
