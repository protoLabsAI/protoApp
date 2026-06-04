# protoApp — status

_Last updated: 2026-06-03. Kept in the root because `HANDOFF.md` points at
it and because status rots fast enough that it deserves to be reviewed
on every push._

## One-line summary

Working Tauri 2 desktop app with an in-process OpenAI-compatible HTTP
server. Chat, transcription, and speech endpoints are all real behind
their Cargo features. Default LLM is **Qwen3-4B-Instruct-2507** on
`llama-cpp-2` because Gemma 4 is blocked upstream — see below.

**Active direction: the iPad pivot.** The product is moving from a macOS
desktop app to an on-device iPad/iOS app (milestones M1–M6). On iPad the
ORBIS Python sidecar can't survive — iOS forbids spawning an external
interpreter — so the agent becomes an in-process Rust runtime
([zeroclaw](https://github.com/protoLabsAI/zeroclaw), vendored, M5). See
the [iPad pivot](#ipad-pivot-m1m6) section below.

## What works today

| Area | Status | Notes |
|---|---|---|
| OpenAI `/v1/*` surface | ✅ | `/v1/models`, `/v1/chat/completions`, `/v1/audio/transcriptions`, `/v1/audio/speech`, `/healthz`. 11/11 tests green. |
| Streaming SSE chat UI | ✅ | Abort + reqId guards in `src/hooks/use-chat.ts`. |
| Real LLM chat | ✅ behind `--features llm` | Qwen3-4B-Instruct-2507 (~2.5 GB, Q4_K_M, Apache-2.0). First-use downloads weights into `~/.cache/protoapp/llm/`. |
| Real STT | ✅ behind `--features stt` | whisper-rs 0.16 against `ggml-base.en-q5_1.bin`. Requires `brew install cmake` once. |
| Real TTS | ✅ behind `--features tts` | kokoros (git dep), Apache-2.0. Requires `cmake`. |
| Metal / CUDA feature gates | ✅ | `--features llm,metal` on Apple Silicon. |
| In-process agent (zeroclaw) | ✅ | `protoapp-agent` embeds `zeroclaw-runtime`; verified end-to-end (real Qwen3-4B reply via the agent on Metal). Replaces the ORBIS sidecar — see [docs/how-to/use-the-in-process-agent.md](./docs/how-to/use-the-in-process-agent.md). |
| React voice panels | ✅ | Chat, Agent, Transcribe, Speak tabs; engine banner shows load / download / error states via a live `engine-status` Tauri event bus. |

## Known blockers / upstream bugs

### Gemma 4: blocked by llama.cpp FGDN tensor-name assert

**What we want**: default to Google's Gemma 4 E4B — strong function
calling, vision-capable, MatFormer runtime footprint.

**What's wrong**: `llama.cpp`'s Fused Gated Delta Network path asserts
on Gemma 4's tensor naming:

```
/Users/kj/.cargo/registry/.../llama-cpp-sys-2-0.1.143/llama.cpp/src/llama-context.cpp:485:
GGML_ASSERT(strncmp(n->name, LLAMA_TENSOR_NAME_FGDN_AR "-", prefix_len) == 0) failed
```

Weights load successfully (`Gemma 4 E4B ready` prints), then the first
inference aborts the process. Reproduced on `llama-cpp-sys-2` both
`0.1.143` and `0.1.145` (the 0.1.145 FGDN rework moved the line number
from 485 → 487 but didn't change the behavior).

**What we tried**:

1. `mistralrs` 0.8.1 → hit three separate Gemma 4 bugs (missing GGUF
   arch enum — upstream [#2098](https://github.com/EricLBuehler/mistral.rs/issues/2098);
   `Gemma4ForConditionalGeneration` rejected by `TextModelBuilder`; serde
   duplicate-field on `expert_intermediate_size` via `ModelBuilder` —
   upstream [#2119](https://github.com/EricLBuehler/mistral.rs/issues/2119) filed by us).
2. `mistralrs` master (2d4ba4f) → same duplicate-field bug; no newer
   commits exist.
3. Swapped to `llama-cpp-2` — hit FGDN assert on both 0.1.143 and
   0.1.145.
4. Disabled Flash Attention (`LLAMA_FLASH_ATTN_TYPE_DISABLED`) — no
   effect, FGDN resolution is independent.
5. Tried Qwen3.5-4B (also uses Gated Delta Networks) — same FGDN assert.

**Workaround in place**: default swapped to Qwen3-4B-Instruct-2507, a
classic attention transformer that never exercises the FGDN path. This
is the model that confirmed-streamed tokens on 0.1.143 end-to-end.

**Unblock plan**: monitor upstream llama.cpp for a PR that updates the
FGDN tensor-name check to accept Gemma 4's actual tensor names (or gates
the FGDN path to models that have correctly-named tensors). When that
lands and ships in a `llama-cpp-sys-2` point release, bump the pin and
flip `DEFAULT_MODEL_REPO` back to `unsloth/gemma-4-E4B-it-GGUF`.

## iPad pivot (M1–M6)

The product is being ported from a macOS desktop app to an on-device
iPad/iOS app. Milestones (the empty `epic/m2…m6` branches are roadmap
markers; recreate from `main` when work on each begins):

| Milestone | Goal | State |
|---|---|---|
| **M1** — engine portability | Prove the inference engines cross-compile to `aarch64-apple-ios` with Metal | ✅ **Done, CI-confirmed 2026-06-03.** llama-cpp-sys-2 0.1.143 (2m22s) + whisper-rs 0.16 both build on `macos-15`. See `docs/spikes/{llama,whisper}-ios.md`; jobs in `.github/workflows/ios-*-spike.yml`. |
| **M2** — iPad walking skeleton | `tauri ios init`, stub build on simulator/device; iOS-in-CI on an owned macOS runner | ⏳ Not started. Needs full Xcode.app (CLI tools alone won't cross-compile). |
| **M3** — chat on iPad | Qwen3-4B running on-device | ⏳ Not started. Unblocked by M1. |
| **M4** — voice on iPad | whisper + kokoro on-device | ⏳ Not started. Unblocked by M1. |
| **M5** — Rust agent replaces ORBIS | Vendor [zeroclaw](https://github.com/protoLabsAI/zeroclaw) as an **in-process** runtime; retire `crates/orbis-sidecar` | ✅ **Done.** `vendor/zeroclaw` submodule + `crates/protoapp-agent` embed `zeroclaw-runtime` in-process (lib, not zeroclaw's `apps/tauri`). Provider → voice-core's local server; cross-compiles to iOS (`docs/spikes/zeroclaw-ios.md`); verified end-to-end with a real Qwen3-4B reply. `orbis-sidecar` removed. Runtime-on-device validation waits for M2. |
| **M6** — polish & ship | Mobile UX, installable | ⏳ Not started. |

### Carry-over desktop tasks

In-repo task-tracker items (not GitHub issues), still relevant:

- **#21** — preload engines on user demand so the first chat doesn't block on a 2.5 GB download (worse on a metered iPad — fold into M3).
- ~~**#12** — wire ORBIS Python into `orbis-sidecar`~~ — **superseded by M5.** ORBIS-on-device is replaced by an in-process Rust agent; don't build new ORBIS-sidecar wiring.

## Cargo pin cheat sheet

These pins are load-bearing; read the comment on each before changing.

| Pin | File | Reason |
|---|---|---|
| `llama-cpp-2 = "=0.1.143"` | `Cargo.toml` | Verified streaming with Qwen3-4B-Instruct-2507. 0.1.145 same FGDN behavior. |
| `llama-cpp-sys-2 = "=0.1.143"` | `Cargo.toml` | Must match `llama-cpp-2` major/minor/patch. |
| `kokoros = { git = ..., rev = "7089168..." }` | `Cargo.toml` | Not on crates.io; pin the revision for reproducible builds. |
| `CMAKE_POLICY_VERSION_MINIMUM=3.5` | `.cargo/config.toml` | Kokoros pulls in C projects that still target CMake ≤ 3.4. Removing this breaks `--features tts` on machines with CMake 4. |

## How to verify "it works" after pulling

```sh
pnpm install
cargo test --workspace
pnpm tauri dev -- --features llm,metal   # macOS
# or --features llm,cuda on NVIDIA, or just --features llm for CPU (slow)
```

Send a message in the **Chat** tab. On first run it downloads
Qwen3-4B-Instruct-2507-Q4_K_M.gguf (~2.5 GB) into
`~/.cache/protoapp/llm/`, then you should see tokens streaming in
within a few seconds of `Qwen3-4B-Instruct-2507 ready` in the logs.
