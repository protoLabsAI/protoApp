# Enable Metal or CUDA

The `llm` feature on its own builds `llama-cpp-2` against the CPU
backend, which works for 4B params (~2–5 tok/s on M-series) but is a
lot slower than using the GPU. Enable a GPU backend too.

## Apple Silicon (Metal + Accelerate)

```sh
pnpm tauri dev -- --features llm,metal
```

This turns on both `llama-cpp-2/metal` (GPU kernels) and
`whisper-rs/metal` (STT, if also enabled). Apple's Accelerate
(first-class BLAS) is already bundled into llama.cpp's Metal backend.

Command Line Tools (`xcode-select --install`) is sufficient — you do
**not** need the full Xcode.app for llama.cpp's Metal path.

## NVIDIA (CUDA)

```sh
pnpm tauri dev -- --features llm,cuda
```

Requires the CUDA toolkit to be installed — `nvcc --version` must work.
On Linux, match your driver's CUDA version (usually 12.x).

llama.cpp auto-selects FlashAttention kernels when the GPU supports
them (Ampere or newer). There's no dedicated `flash-attn` feature to
flip here.

## Composing features

You can stack with other engines:

```sh
cargo build -p protoapp --features engines,metal     # all three engines, Metal GPU
cargo build -p protoapp --features llm,stt,metal     # LLM + STT, no TTS, Metal
cargo build -p protoapp --features llm,cuda          # LLM on NVIDIA
```

## Verifying it's actually on

Run with `RUST_LOG=info`:

```sh
RUST_LOG=info pnpm tauri dev -- --features llm,metal
```

llama.cpp's boot log reports the selected device (e.g.
`ggml_metal_init: GPU Family: MTLGPUFamilyApple8`) and the layer
offload count. On Apple Silicon you should see a non-zero number of
layers offloaded to Metal; on CPU-only builds, all layers stay on CPU.

## Why GPU features don't pull in engines

The workspace uses `crate?/feature` optional-feature syntax:

```toml
metal = ["llama-cpp-2?/metal", "whisper-rs?/metal"]
cuda  = ["llama-cpp-2?/cuda", "whisper-rs?/cuda"]
```

The `?` means "enable the GPU feature on `llama-cpp-2` **only if**
some other feature already enabled `llama-cpp-2`." So
`--features metal` alone is a harmless no-op. This is what lets CI
build multiple combinations without exploding the feature matrix.
