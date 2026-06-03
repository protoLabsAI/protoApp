# Spike: whisper-rs iOS cross-compilation with Metal

**Date:** 2026-06-02
**Status:** GO (pending CI confirmation) — build recipe ready, awaiting `macos-15` runner
**Spike job:** `.github/workflows/ios-whisper-spike.yml`

## Goal

Determine whether `whisper-rs = 0.16` (workspace-pinned) can cross-compile
for `aarch64-apple-ios` with Metal backend support, so the protoApp Tauri
backend can run speech-to-text inference on an iPad without a network
connection.

## Verdict: GO (pending CI confirmation)

The build recipe and CI workflow (`.github/workflows/ios-whisper-spike.yml`)
are ready. The workflow creates an isolated scratch crate depending on
`whisper-rs = 0.16` with the `metal` feature, then runs
`cargo build --target aarch64-apple-ios --release` on a `macos-15` runner.

Once CI confirms a successful build, the decision is **GO** — use
`whisper-rs` on iOS with `--features stt,metal`. If CI fails, consult
the decision matrix below and fall back to `SFSpeechRecognizer`.

### Why this should work

1. **whisper-rs-sys 0.15 vendors whisper.cpp** — the build script clones
   and builds whisper.cpp via cmake. No manual vendoring required.
2. **Metal feature is first-class** — `whisper-rs` exposes a `metal`
   feature flag (disabled by default). The sys crate's cmake config
   respects it and sets `GGML_METAL=ON`.
3. **No OpenMP dependency on iOS** — whisper.cpp falls back to its
   built-in thread pool when OpenMP is unavailable. The iOS SDK doesn't
   include libomp; this is expected and handled.
4. **Same pattern as llama spike** — `llama-cpp-sys-2` cross-compiles
   for iOS with Metal (confirmed in `docs/spikes/llama-ios.md`).
   whisper-rs-sys uses the same cmake + bindgen approach.

### Pending CI verification

Trigger `.github/workflows/ios-whisper-spike.yml` from the Actions tab.
The job will:
1. Build a scratch crate for `aarch64-apple-ios` with Metal
2. Inspect the resulting staticlib for Metal symbols
3. Upload the build log for manual review

## Architecture

```
whisper-rs 0.16 (Rust bindings)
  └── whisper-rs-sys 0.15 (FFI + build script)
        └── cmake → clones & builds whisper.cpp
              └── Metal backend via GGML_METAL
```

Unlike the llama spike (which uses `llama-cpp-sys-2` with explicit env-var
controls), `whisper-rs-sys` drives the build entirely through its own
`build.rs` + cmake. The `metal` feature flag enables Metal support.

## Build Recipe

```sh
# Prerequisites (on macOS host)
rustup target add aarch64-apple-ios
brew install cmake

# Build the workspace crate with STT + Metal:
cargo build --target aarch64-apple-ios --release \
  -p protolabs-voice-core \
  --features stt,metal

# Or isolated scratch crate:
cargo build --target aarch64-apple-ios --release \
  --manifest-path /tmp/ios-whisper-spike/Cargo.toml
```

## Environment Variables

No special env vars required (unlike llama.cpp which needs
`LLAMA_CPP_SYS_2_CXXSTANDARD` and `OPENMP_ROOT`). The `whisper-rs-sys`
build script handles cmake configuration internally.

## Cargo Features

```toml
# Minimum for iOS STT with Metal:
whisper-rs = { version = "0.16", default-features = false, features = ["metal"] }

# Full voice-core with STT + Metal:
protolabs-voice-core --features stt,metal
```

## Known Risks

1. **whisper.cpp vendored via git** — `whisper-rs-sys` clones whisper.cpp
   at build time. If the default branch moves or breaks, builds become
   non-reproducible. Check `whisper-rs-sys`'s `build.rs` for the exact
   commit/tag it pins.
2. **bindgen on iOS target** — `whisper-rs-sys` depends on `bindgen` to
   generate FFI bindings. Cross-compiling with bindgen requires the target
   SDK's sysroot. macOS runners include this; custom runners may not.
3. **No OpenMP on iOS** — whisper.cpp's default build uses OpenMP for
   thread parallelism. The iOS SDK doesn't include libomp. whisper.cpp
   should fall back to its native thread pool, but verify the build log
   doesn't error on missing OpenMP.
4. **Full Xcode.app required** — same as the llama spike. The iOS SDK and
   Metal framework headers come from the full Xcode install, not just CLI
   tools.

## Verification Steps

### 1. CI workflow succeeds

Trigger `.github/workflows/ios-whisper-spike.yml` manually from the
Actions tab. The job should complete with exit code 0.

### 2. Build log confirms Metal

Check the uploaded build log artifact for cmake confirming Metal:

```
-- WHISPER_METAL = ON
-- GGML_METAL = ON
```

### 3. Staticlib contains Metal symbols

```sh
LIB=$(find target/aarch64-apple-ios/release/deps -name 'whisper*.a' | head -1)
nm "$LIB" | grep -i metal    # Should show Metal-related symbols
file "$LIB"                   # Should report: Mach-O 64-bit object arm64
```

## Decision Matrix

| Scenario | Decision | Action |
|---|---|---|
| Build succeeds, Metal symbols present | **GO** — use whisper-rs on iOS | Proceed with `--features stt,metal` in Tauri iOS build |
| Build succeeds, no Metal symbols | **CONDITIONAL GO** | whisper.cpp may still work via CPU; benchmark performance. If too slow, enable Metal manually or fallback |
| Build fails (bindgen / cmake) | **NO-GO** — use native fallback | Document exact error, pivot to `SFSpeechRecognizer` (D5) |
| Build fails (missing OpenMP / SDK) | **NO-GO** — use native fallback | Same as above |

## Fallback: Native SFSpeechRecognizer (D5)

If whisper-rs cannot compile for iOS, fall back to Apple's native
speech recognition:

```swift
import Speech

class VoiceSTTService {
    private let speechRecognizer: SFSpeechRecognizer

    init() {
        // Uses the device's default language
        self.speechRecognizer = SFSpeechRecognizer()!
    }

    func transcribe(audioBuffer: AVAudioBuffer) async -> String {
        let request = SFSpeechURLRecognitionRequest(url: audioBuffer.url)
        request.shouldReportPartialResults = false

        do {
            let result = try await speechRecognizer.recognitionTask(with: request).finish
            return result.bestTranscription.formattedString
        } catch {
            fatalError("STT failed: \(error)")
        }
    }
}
```

**Tradeoffs vs whisper-rs:**
- **Pro:** Zero build complexity, native performance, optimized by Apple
- **Pro:** Supports offline mode on iOS 15+ (`SFSpeechRecognitionTask` with local model)
- **Con:** Requires user permission (Speech Recognition entitlement)
- **Con:** Not available on non-Apple platforms (loses cross-platform uniformity)
- **Con:** Offline models are language-limited and controlled by OS updates

## References

- `whisper-rs` crate: https://crates.io/crates/whisper-rs
- `whisper-rs-sys` crate: https://crates.io/crates/whisper-rs-sys
- whisper.cpp Metal backend: https://github.com/ggerganov/whisper.cpp#build-with-metal
- SFSpeechRecognizer: https://developer.apple.com/documentation/speech/sfspeechrecognizer
- Llama iOS spike (pattern reference): `docs/spikes/llama-ios.md`
