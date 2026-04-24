//! Real inference engines wired behind per-engine cargo features.
//!
//! Submodules:
//!   * `llm` — mistralrs + Gemma 4 E2B (GGUF) behind `--features llm`
//!   * `stt` — whisper-rs + whisper.cpp behind `--features stt`
//!   * `tts` — kokoros + Kokoro-82M ONNX behind `--features tts`
//!
//! The API handlers fall back to streaming stubs when a feature is off, so
//! the plumbing is always exercisable end-to-end without the 10+ minute cold
//! compile of real engines.

pub mod events;

#[cfg(feature = "llm")]
pub mod llm;

#[cfg(feature = "stt")]
pub mod stt;

#[cfg(feature = "tts")]
pub mod tts;
