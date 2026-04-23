# Enable Metal or CUDA

The `llm` feature on its own builds mistralrs against the CPU backend,
which is unusable for any model larger than a few hundred million
params. Enable a GPU backend too.

## Apple Silicon (Metal + Accelerate)

```sh
pnpm tauri dev -- --features llm,metal
```

This turns on both `mistralrs/metal` (GPU kernels) and
`mistralrs/accelerate` (Apple's BLAS). No Xcode command line tools
beyond what Tauri already requires.

## NVIDIA (CUDA)

```sh
pnpm tauri dev -- --features llm,cuda
```

Requires the CUDA toolkit to be installed — `nvcc --version` must work.
On Linux, match your driver's CUDA version (usually 12.x).

Optional: `--features llm,cuda,flash-attn` enables FlashAttention 2
kernels. Needs a modern GPU (Ampere or newer) and roughly doubles
decode speed on longer contexts.

## Composing features

You can stack with other engines:

```sh
cargo build -p protoapp --features engines,metal     # all three engines, Metal GPU
cargo build -p protoapp --features llm,stt,metal     # LLM + STT, no TTS, Metal
cargo build -p protoapp --features llm,cuda,flash-attn
```

## Verifying it's actually on

Run with `RUST_LOG=info`:

```sh
RUST_LOG=info pnpm tauri dev -- --features llm,metal
```

Look for mistralrs's own boot log line that reports the selected
device. On Apple Silicon you want to see `metal`, not `cpu`.

## Why GPU features don't pull in engines

The workspace uses `crate?/feature` optional-feature syntax:

```toml
metal = ["mistralrs?/metal", "mistralrs?/accelerate", "whisper-rs?/metal"]
cuda  = ["mistralrs?/cuda", "whisper-rs?/cuda"]
```

The `?` means "enable the GPU feature on `mistralrs` **only if** some
other feature already enabled `mistralrs`." So `--features metal`
alone is a harmless no-op. This is what lets CI build multiple
combinations without exploding the feature matrix.
