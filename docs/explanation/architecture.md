# Architecture overview

## One-paragraph mental model

protoApp is a Tauri desktop app where **the Rust host process runs a
local OpenAI-compatible HTTP server**, and the React frontend talks to
that server using the standard `openai` npm package — same SDK,
different `baseURL`. The server is backed by in-process Rust inference
engines (mistralrs for LLM, whisper-rs for STT, kokoros for TTS). A
second workspace crate, `orbis-sidecar`, provides the plumbing for a
Python agent process to hang off this system for higher-order
reasoning without ever touching the audio hot path.

## Why Tauri + in-process Rust inference

The realistic alternative was browser-only — WebLLM, Transformers.js,
Kokoro.js on WebGPU. We took that path far enough to write it up (see
the research thread in git log), then rejected it:

| Dimension | Browser (WebLLM/Transformers.js) | **Rust-native via Tauri** |
|---|---|---|
| LLM throughput on M3 Max (Llama 3.1 8B q4) | ~41 tok/s (~75 % native) | ~80–100 tok/s (~100 % native) |
| First-token latency | 200–600 ms (WebGPU pipeline compile) | 30–80 ms |
| STT (30 s clip) | 2–5 s | 0.5–1.5 s |
| JS shipped to webview | ~3–5 MB + WASM | ~0 MB |
| Audio GC stutter | possible (JS heap) | none (Tokio + Rust) |
| OpenAI SDK drop-in | needs a Service Worker shim | native `/v1/*` HTTP |

The Rust-native path wins on every dimension that matters for a
conversational voice app.

## Why an HTTP server at all (vs Tauri commands)

We could have exposed `chat`, `transcribe`, `speak` as Tauri commands
and skipped HTTP entirely. We didn't, because:

1. **OpenAI SDK drop-in** — `new OpenAI({ baseURL })` works unchanged. Tauri-command-shaped APIs would need a custom shim.
2. **External tooling** — curl, LangChain.js, the Vercel AI SDK, any OpenAI-compatible client can hit `http://127.0.0.1:<port>/v1` during dev. Great for debugging and scripting.
3. **Same-origin security** — binding to `127.0.0.1` only makes the surface safe by construction. No authentication needed.

The cost is one extra JSON hop vs direct in-process calls
(~1–3 ms) — negligible next to model inference time.

## The three workspace crates

See [reference/workspace-crates.md](../reference/workspace-crates.md)
for the concrete layout. The important property is:

- **`protolabs-voice-core`** is reusable. A future `orbis-tauri` app, a headless CLI, or a Cloud Run deployment can all embed it.
- **`orbis-sidecar`** is pure IPC — no engine code. We can use it without voice, or use voice without it.
- **`protoapp`** is the thinnest possible Tauri shell proving both.

## Feature flags as a complexity throttle

Compiling mistralrs cold takes 10–15 minutes. That's poisonous to the
"clone and contribute" experience for anyone not working on inference.
So the default build is stub-only — 25 s compile, still useful because
the streaming stub proves every layer of transport works end-to-end.

You pay for what you opt into: `--features llm`, `--features stt`,
`--features tts`, combined with `--features metal` or `--features cuda`
for GPU backends. The `crate?/feature` syntax keeps the feature
matrix composable (details in
[reference/cargo-features.md](../reference/cargo-features.md)).

## What's intentionally out of scope (today)

- **Pipeline orchestration** (turn detection, interruption, echo cancellation) — belongs in a future audio-IO crate, not in voice-core. For now, the pipeline is your frontend's responsibility.
- **Remote sharing** — the server binds `127.0.0.1` only. If you need remote clients, front it with a Tauri-sidecar reverse proxy that does auth properly.
- **Multi-tenant model serving** — we load one instance per engine, globally. Fine for desktop; insufficient for server.

## Further reading

- [Engine choices](./engine-choices.md) — *why* mistralrs + whisper-rs + Kokoro, specifically.
- [Voice hot path vs agent brain](./voice-hotpath-vs-agent-brain.md) — *why* ORBIS stays Python instead of getting rewritten in Rust.
