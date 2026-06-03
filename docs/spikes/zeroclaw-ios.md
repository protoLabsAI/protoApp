# Spike: zeroclaw-runtime iOS cross-compilation (M5 feasibility)

**Date:** 2026-06-03
**Status:** PENDING — host build confirmed, iOS cross-compile under test in CI
**Spike job:** `.github/workflows/ios-zeroclaw-spike.yml`

## Goal

Milestone **M5** replaces the ORBIS Python sidecar with an in-process Rust
agent: [`zeroclaw`](https://github.com/protoLabsAI/zeroclaw), vendored. On
iPad the agent *must* run in-process — iOS forbids spawning an external
interpreter (the same constraint that kills ORBIS). So the entire
`zeroclaw-runtime` dependency tree has to cross-compile for
`aarch64-apple-ios`.

This spike builds a minimal scratch staticlib that depends on
`zeroclaw-runtime` (`default-features = false`, pinned to zeroclaw commit
`ea2d849f`) for the iOS target.

## What's already confirmed

- **Host build (aarch64-apple-darwin): GO.** `zeroclaw-runtime`
  (`default-features = false`) compiles cleanly as an embedded git
  dependency in **1m42s**, pulling `zeroclaw-config`, `-providers`,
  `-memory`, `-tools`, `-tool-call-parser`. The `publish = false`
  workspace members resolve fine via a git dependency.
- **Embeddability: GO.** zeroclaw's own `apps/tauri` uses a separate-process
  model, but `zeroclaw-runtime` is a library and embeds directly — no
  sidecar process needed.

## What this spike decides

| Outcome | Meaning | Next action |
|---|---|---|
| Build succeeds | `zeroclaw-runtime` cross-compiles for iOS as-is | Wire it into the workspace via the `vendor/zeroclaw` submodule + a thin in-process bridge crate |
| Build fails on a specific dep | That dep is iOS-hostile (fork/exec, a desktop-only C lib, etc.) | Feature-gate it off, or patch the `protoLabsAI/zeroclaw` fork ("modify as needed"), then re-run |

A NO-GO here is still useful: the failing dependency names exactly what
needs trimming before M5 can ship.

## Vendoring plan (M5)

1. Add `protoLabsAI/zeroclaw` as a git submodule at `vendor/zeroclaw`,
   pinned to the spike's commit (so on-device builds are reproducible and
   the fork is modifiable in place).
2. Add a thin `protoapp-agent` bridge crate to the workspace that depends on
   `zeroclaw-runtime` (trimmed feature set) and exposes a small in-process
   embed API to the Tauri shell.
3. Configure a zeroclaw LLM provider pointed at voice-core's local
   OpenAI-compatible server (`http://127.0.0.1:<port>/v1`) so the agent
   runs fully on-device.
4. Retire `crates/orbis-sidecar`.
