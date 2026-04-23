# The Rust voice hot path vs the Python agent brain

This is the most important architectural decision in the repo, because
it's the one we could have gotten wrong and broken everything. Here's
what we chose and why.

## The two concerns

A conversational voice app has two very different workloads:

**Hot path** — audio in, audio out
- 20 ms audio frames at 48 kHz
- Voice activity detection, turn detection, echo cancellation
- Token-by-token STT, LLM, TTS streaming
- Frame-accurate interruption
- Any GC stutter >20 ms is audible to the user

**Cold path** — the agent brain
- Memory, tool calling, multi-turn planning
- Integration with external services (calendar, email, memory stores)
- Identity/auth, user context, long-running background work
- Latency budget: hundreds of ms, sometimes seconds

The mistake most voice AI stacks make is to run both workloads on the
same substrate. Pipecat is the most popular — everything's in Python.
On a busy Mac, its 20 ms frames see 50–100 ms GC pauses often enough
that you can hear them.

## What we chose

**Rust owns the hot path. Python owns the brain. They talk via WebSocket.**

```
┌──────────────── Tauri process (Rust) ─────────────────┐
│                                                        │
│  React UI  ← invoke/emit ──  Tauri commands            │
│                             ↓                          │
│                    protolabs-voice-core                │
│              ┌───────────────────────────┐             │
│              │ /v1/chat/completions      │             │
│              │ /v1/audio/transcriptions  │             │
│              │ /v1/audio/speech          │             │
│              │ + in-process engine API   │             │
│              └───────────────────────────┘             │
│                             ↓                          │
│                     orbis-sidecar                      │
│                  (spawn + WS client)                   │
└──────────────────────────┬─────────────────────────────┘
                           │ ws://127.0.0.1:<ephemeral>
                           ↓
            ┌──────── ORBIS Python sidecar ─────────┐
            │  agent/, a2a/, memory/, auth/, tools/ │
            │  (unchanged from the existing repo)   │
            └───────────────────────────────────────┘
```

The Rust side:
- Captures audio, runs VAD, does STT, does TTS
- Exposes the OpenAI surface for pure-LLM workloads
- Routes "user said X" → WebSocket → Python → "agent replies Y" → TTS

The Python side:
- Receives `{"type":"user","text":"..."}` text messages
- Runs the agent loop (tool calling, memory, etc.)
- Emits `{"type":"token","text":"..."}` streams back

No audio ever crosses the seam. That's the point.

## Why this exact shape

It's not novel — it's what [Feros](https://github.com/ferosai/feros)
did after open-sourcing their production voice stack. The rationale is
the same:

- Rust gives us zero GC pauses on 20 ms frames.
- Python gives us the entire agent-framework ecosystem (LangChain, LlamaIndex, Pydantic-AI, the A2A protocol ORBIS already uses) without rewrite.
- The WebSocket seam means we can evolve each side independently.
- It also means a future deployment variant can split the two across machines.

## Why not rewrite ORBIS in Rust

We considered it. Kalosm (floneum/floneum) gives us LLM + Whisper + VAD
primitives. In another 3–6 weeks we could have a Rust-only ORBIS.

Reasons we didn't:

1. **Agent frameworks move faster in Python.** MCP, A2A, most tool-use scaffolding, most memory stores — they're all Python-first. Rewriting means perpetually chasing.
2. **ORBIS already works.** Throwing that away is a real cost, not an abstract one.
3. **GC pauses don't affect the brain.** The workloads where Python hurts are 20 ms audio frames. A 100 ms GC pause on the agent thread is invisible — the user is already waiting on LLM tokens.
4. **Isolation is a feature.** If the Python side crashes, the Rust side survives, the audio stays live, and the UI reports the error cleanly. Monolithic Rust has a much bigger blast radius.

## Why a WebSocket, not a Tauri sidecar pipe or gRPC

- **WebSocket** is already the lingua franca of agent frameworks — ORBIS likely has WebSocket support anyway.
- Human-readable JSON on the wire means we can `wscat` against the sidecar during dev without tooling.
- Bidirectional streaming is natural.
- gRPC was too heavy for the message volume we expect. Protobuf schemas would be overkill.
- Raw Tauri sidecar stdio pipes don't handle back-pressure or bidirectional streaming without hand-rolling a frame protocol.

## When to revisit

If any of these become untrue, reconsider:

- Python cold start grows past ~5 s (users notice).
- The `{type: ...}` JSON protocol starts accreting so many variants that serde's tagged enums feel like the wrong tool.
- We start shipping audio over the seam (we should not — that's what the Rust side is for).
- Rust agent frameworks catch up to Python's maturity (Kalosm is close on primitives; ecosystem isn't there yet).

Until then, stay the course.
