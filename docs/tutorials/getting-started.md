# Getting started

At the end of this tutorial you'll have protoApp running locally with a
streaming chat UI talking to an OpenAI-compatible server that's living
inside the Tauri process.

No model download is required — the default build uses a streaming stub
so you can verify the whole stack works before committing to a 1.5 GB
model pull.

## 1. Prerequisites

You need:

- macOS, Linux, or Windows
- [Rust](https://rustup.rs) 1.80+ (`rustc --version`)
- [Node.js](https://nodejs.org) 20+ and [pnpm](https://pnpm.io/installation) 9+
- The platform prerequisites for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

Verify:

```sh
rustc --version
pnpm --version
node --version
```

## 2. Clone and install

```sh
git clone https://github.com/protolabsai/protoApp
cd protoApp
pnpm install
```

## 3. Run the app

```sh
pnpm tauri dev
```

The first launch compiles the Rust workspace (~30 seconds clean). When
the window opens, you'll see a chat panel.

## 4. Send a message

Type "hello" and press Send. You should see a streaming reply like:

> \[stub reply — build with `--features metal` (macOS) or `--features cuda` for real Gemma 4 inference\] You said: hello

That "stub reply" is the point of this tutorial — it proves:

1. The frontend OpenAI SDK resolved the Tauri command `get_api_base_url` and got a `http://127.0.0.1:<port>` URL.
2. The Rust side bound an Axum server on an ephemeral port.
3. `/v1/chat/completions` streamed Server-Sent Events back to the browser.
4. The frontend accumulated deltas and rendered them live.

If any of those steps broke, it would have been obvious — no model to
blame.

## 5. Run the tests

In a second terminal:

```sh
cargo test --workspace
```

You should see 11 tests passing across `protolabs-voice-core` (6) and
`orbis-sidecar` (5).

## Next

- [Run Gemma 4 locally](./run-gemma-4-locally.md) — swap the stub for real inference.
- [OpenAI-compatible API reference](../reference/openai-api.md) — what endpoints exist.
- [Architecture overview](../explanation/architecture.md) — how the pieces fit.
