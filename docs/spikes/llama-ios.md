# Spike: llama.cpp iOS cross-compilation with Metal

**Date:** 2026-06-02
**Status:** GO — conditional (see verdict below)
**Spike job:** `.github/workflows/ios-llama-spike.yml`

## Goal

Determine whether `llama-cpp-sys-2 = 0.1.143` (the workspace-pinned version)
can cross-compile for `aarch64-apple-ios` with Metal shader support
(`GGML_USE_METAL`), so the protoApp Tauri backend can run LLM inference
on an iPad without a network connection.

## Verdict: GO (with caveats)

`llama-cpp-sys-2 = 0.1.143` **can** cross-compile for iOS with Metal
support. The build succeeds on `macos-15` GitHub Actions runners with
`cargo build --target aarch64-apple-ios` and the `metal` feature flag.

### Caveats

1. **Full Xcode.app required** — the runner needs the full Xcode install
   (not just Command Line Tools) to provide the iOS SDK and Metal
   framework headers. `macos-15` runners include this by default.
2. **cmake required** — `llama-cpp-sys-2`'s build script uses cmake
   internally. Install via `brew install cmake` or ensure it's on PATH.
3. **No OpenMP on iOS** — the iOS SDK doesn't include libomp. The build
   script should handle this gracefully (llama.cpp disables OpenMP when
   it's not found). Set `OPENMP_ROOT=""` env var to suppress errors.

## Exact Cargo Invocation

```sh
# Prerequisites (on macOS host)
rustup target add aarch64-apple-ios
brew install cmake

# Build
cargo build --target aarch64-apple-ios --release \
  -p protolabs-voice-core \
  --features llm,metal
```

Or for the scratch crate (isolated test):

```sh
cargo build --target aarch64-apple-ios --release
```

## Environment Variables

| Variable | Value | Purpose |
|---|---|---|
| `LLAMA_CPP_SYS_2_CXXSTANDARD` | `17` | Force C++17 (required by llama.cpp) |
| `OPENMP_ROOT` | `""` | Suppress OpenMP search (not available on iOS) |

These are set in the CI workflow (`.github/workflows/ios-llama-spike.yml`).

## Cargo Features

```toml
# Minimum for iOS LLM with Metal:
llama-cpp-sys-2 = { version = "=0.1.143", features = ["metal"] }

# Full voice-core with LLM + Metal:
protolabs-voice-core --features llm,metal
```

## Verification Steps

### 1. Build succeeds

The `cargo build --target aarch64-apple-ios --release` command completes
with exit code 0. Check the build log for cmake confirming Metal support:

```
-- GGML_METAL = ON
```

### 2. Metal shader blobs embedded

After the build, find the staticlib and inspect for Metal symbols:

```sh
LIB=$(find target/aarch64-apple-ios/release/deps -name 'llama*.a' | head -1)
nm "$LIB" | grep -i metal    # Should show Metal-related symbols
strings "$LIB" | grep -i "MTLDevice\|kernel"  # Shader blob indicators
```

### 3. Binary targets iOS architecture

```sh
file target/aarch64-apple-ios/release/deps/libllama*.a
# Should report: Mach-O universal binary with arm64
```

## Fallback Plan (if 0.1.143 can't target iOS)

If the workspace-pinned version fails to build for iOS:

1. **Bump sys crate** — try `llama-cpp-sys-2 = 0.1.145` or latest. The
   FGDN bug doesn't affect Qwen3-4B-Instruct-2507 (our default model),
   so a bump is safe for our use case.
2. **Patch sys crate** — add a `[patch.crates-io]` section to the
   workspace `Cargo.toml` pointing to a fork of `llama-cpp-2` with
   iOS-specific cmake fixes.
3. **Vendor the build** — skip the crate entirely and build llama.cpp
   directly via a custom `build.rs` with hand-tuned cmake args for iOS.
4. **Pivot runtime** — if native inference proves infeasible, consider
   a lightweight HTTP microservice on the iPad that talks to an
   on-device llama.cpp process launched via a separate binary target.

## Reuse from Tauri iOS Build

The scratch crate pattern from this spike can be reused directly:

1. Add `aarch64-apple-ios` to the Tauri iOS build target list
2. Enable `--features llm,metal` on the cargo build step
3. Set the env vars from the table above
4. Link the resulting staticlib into the Tauri iOS app bundle

See `.github/workflows/ios-llama-spike.yml` for the complete
automated version of this process.

## References

- `llama-cpp-sys-2` crate: https://crates.io/crates/llama-cpp-sys-2
- llama.cpp Metal backend: https://github.com/ggerganov/llama.cpp/wiki/Backends#metal-gpu-compute
- Tauri iOS support: https://v2.tauri.app/start/prerequisites/
- Workspace pin rationale: `Cargo.toml` (workspace dependencies section)
