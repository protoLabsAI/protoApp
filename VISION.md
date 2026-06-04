# protoApp — Vision & Roadmap

_Last updated: 2026-06-03. The north star and the plan to get there.
Current build state lives in [STATUS.md](./STATUS.md); the three
non-obvious gotchas in [HANDOFF.md](./HANDOFF.md)._

## One sentence

**protoApp is a voice-first companion orb for iPad — ORBIS reborn
native — whose superpower is delegating heavy work to your protoAgent
fleet over A2A.**

You talk to the orb. It talks back in real time, remembers you, and has
a personality that drifts over time. When you ask for something it can't
do itself, it routes the work to one of your cloud agents and speaks the
answer back. Everything that can run on-device does; the heavy reasoning
lives in the fleet.

## Why this shape

Three protoLabs systems compose into protoApp **without overlap**:

| Layer | We take it from | Where it runs |
|---|---|---|
| **Companion experience** — voice, the orb, personality, mood, memory | [ORBIS](https://github.com/protoLabsAI/ORBIS) | on-device (iPad, Rust) |
| **Heavy reasoning** — the agents that do real work | [protoAgent](https://github.com/protoLabsAI/protoAgent) (LangGraph A2A) | the cloud fleet |
| **The wire between them** — delegation | A2A (JSON-RPC + SSE + cost-v1) | network |

The keystone is a decision ORBIS already made and froze (its
`DECISIONS.md`): **"Heavy reasoning via `delegate_to`, not in-process.
No bundled protoAgent. No in-process LangGraph. Smart agents are external
delegate targets."** That is *why* protoApp's hard constraint —
**iOS can't run a Python interpreter** — costs us nothing: the Python
agents were always meant to be external A2A targets. protoApp's
on-device agent is the **router + companion**; the protoAgent fleet is
the **brains**; A2A is the **wire**.

So protoApp is not "ORBIS ported to Rust." It's **ORBIS's product,
rebuilt native for iPad**, with the Python pieces that can't ship on iOS
(Pipecat, LangGraph) replaced by on-device Rust ([zeroclaw](https://github.com/protoLabsAI/zeroclaw),
already embedded — see [STATUS.md](./STATUS.md)) and remote A2A delegation.

## Architecture

```
┌───────────────── iPad — protoApp (Tauri / Rust) ─────────────────┐
│                                                                  │
│   The Orb  ── WebGL/GLSL, personality-reactive (reuse ORBIS)     │
│      ▲                                                            │
│   realtime voice loop ── VAD · barge-in · backchannel (native)   │
│      │   whisper-rs (STT) ─┐         ┌─ kokoros (TTS)             │
│      ▼                     ▼         ▲                            │
│   ┌──────────── companion brain (zeroclaw, in-process) ────────┐ │
│   │  Qwen3-4B router + personality/mood + SQLite memory        │ │
│   │  tool surface (small, ORBIS-shaped):                       │ │
│   │    delegate_to · adjust_personality · remember ·           │ │
│   │    show_inbox · confirm                                    │ │
│   └───────────────────────────┬───────────────────────────────┘ │
│        LLM provider → 127.0.0.1/v1 (protolabs-voice-core)        │
└───────────────────────────────┼─────────────────────────────────┘
                                 │  delegate_to → A2A (JSON-RPC + SSE, cost-v1)
                                 ▼
              ┌────────────────────────────────────────────┐
              │  The fleet (cloud, Python)                  │
              │  protoAgent-based A2A agents: quinn, …      │
              │  (LangGraph + LiteLLM + tools + memory)     │
              │  operated via protoLabs Studio / operator_api│
              └────────────────────────────────────────────┘
```

On-device: `protolabs-voice-core` (the OpenAI-compatible engine
substrate — Qwen3-4B + whisper + kokoro, Metal) and `protoapp-agent`
(the embedded zeroclaw companion brain) are **already built and
verified** (M5). The orb, the realtime voice loop, the companion layer,
and the A2A delegate are what's ahead.

## What each system contributes

### From ORBIS — the experience (re-implemented native)
- **Voice-first, router-first.** Realtime bidirectional voice is *the*
  interaction; text is a secondary accessibility mode.
- **The orb.** A visible, expressive form whose state reflects the
  companion's mood/personality. ORBIS's GLSL can be reused in protoApp's
  Tauri webview (WebGL).
- **Companion layer.** Slow-drift personality (per-axis), mood,
  soft-neglect behavior, persistent memory. ORBIS's SQLite schema
  (`facts` with valid/invalid/confidence, `personality_axes`, `mood`,
  `sessions`) is the model; maps onto zeroclaw's sqlite memory or a
  dedicated companion store.
- **Tiny tool surface.** `delegate_to` (the spine), `adjust_personality`,
  and optionally `remember` / `show_inbox` / `confirm`. Heavy capability
  comes through delegation, not a big local toolbox.

### From protoAgent — the fleet contract & extensibility
- **A2A protocol.** JSON-RPC + SSE, `message/send`, cost-v1 DataPart for
  cost reporting, `a2a.trace` propagation. This is the `delegate_to`
  wire. ORBIS's `a2a_outbound.py` is the reference contract to port to
  Rust.
- **Extensibility model.** SKILL.md skills, MCP servers, plugins —
  protoAgent's three opt-in extension points. **zeroclaw already supports
  all three natively** (`zeroclaw-runtime` has skills/tools/plugins + MCP),
  so the on-device companion gets a protoAgent-grade extensible base for
  free.
- **Operator conventions.** `operator_api` (beads/notes/runtime/subagents)
  + protoLabs Studio are how the fleet is run — the substrate behind
  later fleet-ops features.

### Native-new (the build ahead)
- Realtime conversational voice on iOS **without Pipecat** (Rust/native
  VAD, barge-in, echo-guard).
- An **A2A client in Rust** (zeroclaw ships ACP, not A2A — assume net-new).
- The orb rendered in the Tauri webview.
- The companion memory/personality/mood store + drift logic.

## Roadmap

Phases layer on top of the existing M-track. ✅ = done & verified.

### Phase A — Foundation _(nearly complete)_
- **M1** ✅ engine portability to iPad (llama/whisper, Metal) — CI-proven.
- **M5** ✅ in-process Rust agent (zeroclaw embedded; real reply verified).
- **M2** ⏳ Tauri iOS walking skeleton — *next*, gated on Xcode. Stub on
  simulator/device; validates the M5 runtime caveat on real hardware.
- **M3 / M4** chat + voice engines on-device (engines work today; M2 puts
  them on the device).

### Phase B — Companion _(the heart of v1)_
- **B1 — Realtime voice loop (spike first).** Prove native iOS
  always-listening + barge-in + echo-guard is feasible without Pipecat,
  the way M1/M5 were proven before building. Biggest technical risk; de-risk early.
- **B2 — Companion store.** Port ORBIS's SQLite schema (facts,
  personality_axes, mood, sessions); wire session persistence + memory
  recall into the zeroclaw turn.
- **B3 — Personality & mood.** Slow-drift axes, mood state, soft-neglect;
  `adjust_personality` tool; expose state to the UI.
- **B4 — The orb.** WebGL/GLSL expressive form (reuse ORBIS shaders),
  reactive to mood/personality/speaking state.

### Phase C — Delegation _(the superpower)_
- **C1 — A2A client in Rust (spike first).** Port `a2a_outbound.py`'s
  contract: JSON-RPC + SSE + cost-v1. Expose as zeroclaw's `delegate_to`
  tool. Prove against one live protoAgent fleet agent (e.g. quinn).
- **C2 — Delegate UX.** "Orb, ask quinn to QA the release" → delegate →
  stream progress → speak the result. Target discovery/config
  (which agents, their A2A URLs + auth).

### Phase D — Extensible & Ship
- **D1 — Extensibility surface.** Expose zeroclaw skills / MCP / plugins
  so the companion is shapeable without a rebuild (protoAgent-grade).
- **D2 — Polish & ship.** Onboarding, single-owner auth, installable iPad
  build, performance pass.

### Later (post-v1) — Fleet command deck
The broader "manage the entire fleet from iPad" surface — board
monitoring, agent/PR control, spoken briefings/standups — built on
protoLabs Studio + `operator_api`. Deferred: v1's fleet interaction is
**delegation**, not a dashboard.

## Key technical bets & risks

1. **Realtime conversational voice on iOS-native is the hardest unknown.**
   ORBIS leans on Pipecat (Python); we can't. Mitigation: spike B1 before
   committing, exactly like M1/M5.
2. **A2A in Rust is net-new.** zeroclaw speaks ACP, not A2A. Mitigation:
   port ORBIS's well-factored `a2a_outbound.py` contract; spike C1 against
   a real agent.
3. **The orb in a webview** must stay smooth alongside on-device
   inference + audio. Mitigation: it's a self-contained WebGL component;
   prototype in isolation.
4. **On-device memory/personality** must not bloat the turn latency.
   Mitigation: ORBIS already solved the shape (SQLite + FTS5, vec optional).

## Non-goals (inherited from ORBIS's locked decisions)

- **No in-process LangGraph / no bundled protoAgent.** Smart agents are
  external A2A delegate targets.
- **No multi-tenant.** Single-owner, single-user, multi-device.
- **No skills *catalog* / interchangeable personas.** One orb persona per
  install, user-configurable via one config — distinct from the
  *extensibility* (skills/MCP/plugins) that adds capability.
- **Adults-only**, single-owner auth posture.
- **Fleet dashboard is post-v1.** v1 fleet interaction is delegation.

## Open questions

- **A2A target discovery/config** — how does the orb learn which fleet
  agents exist and their URLs/auth? (protoLabs Studio registry? a config
  file? both?)
- **Where the companion store lives** — extend zeroclaw's memory, or a
  dedicated `protoapp-companion` crate owning the ORBIS-shaped schema?
- **Wake/turn model** for realtime voice — always-on mic vs. wake word vs.
  tap-to-engage-then-conversational.
- **Auth to the fleet** — per-agent A2A keys; how stored on-device.
