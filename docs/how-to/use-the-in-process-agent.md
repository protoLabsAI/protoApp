# Use the in-process agent

protoApp embeds [zeroclaw](https://github.com/protoLabsAI/zeroclaw) as an
**in-process** Rust agent (the `protoapp-agent` crate, backed by the
`vendor/zeroclaw` submodule). It's the iPad-safe replacement for the ORBIS
Python sidecar — iOS can't spawn an external interpreter, so the agent runs
inside the Tauri process. Its LLM provider is wired to the local
`protolabs-voice-core` server, so reasoning happens on-device against the same
model the **Chat** tab uses.

## Prerequisites

The agent needs a real LLM to produce real replies. Build with the `llm`
feature (and `metal` on Apple Silicon):

```sh
pnpm tauri dev -- --features llm,metal
```

Without `llm`, voice-core serves a stub and the agent will echo/placeholder.
The first real turn downloads the model (~2.5 GB) into `~/.cache/protoapp/`.

## Run a turn

1. Launch the app and open the **Agent** tab.
2. Type a message and press **Ask**. The frontend calls the `agent_ask` Tauri
   command, which drives one zeroclaw turn via `protoapp_agent::ask(...)` and
   returns the reply. A per-session id threads the agent's memory across turns.

## Where the agent config lives

On first call, `protoapp-agent` writes a zeroclaw `config.toml` to the OS
config dir under `protoapp/agent/` (e.g. `~/Library/Application Support/` or
`~/.config/protoapp/agent/` depending on platform) and points
`ZEROCLAW_CONFIG_DIR` at it. The generated config wires a single `default`
agent to the local server:

```toml
schema_version = 3

[providers.models.openai.local]
model = "qwen3-4b-instruct-2507"   # must match voice-core's /v1/models catalog
api_key = "local"
uri = "http://127.0.0.1:<port>/v1"

[agents.default]
model_provider = "openai.local"
risk_profile = "default"

[risk_profiles.default]
level = "supervised"
```

Edit that file to add tools, change the risk profile / autonomy level, or
switch the memory backend — `protoapp-agent` only rewrites it when the
`base_url`/`model` it was generated for change, so hand-edits survive.

## Modifying zeroclaw itself

`vendor/zeroclaw` is a git submodule pinned to a commit of
`protoLabsAI/zeroclaw`. To change agent behaviour at the source level, edit it
in place (it's depended on by path), then commit the new submodule pointer.
After cloning protoApp fresh, run `git submodule update --init` to populate it.

## How it cross-compiles to iPad

`zeroclaw-runtime` and its whole dependency tree cross-compile to
`aarch64-apple-ios` with `default-features = false` — confirmed in CI. See
[the spike](../spikes/zeroclaw-ios.md).
