//! Real inference engines wired behind per-engine cargo features.
//!
//! Each submodule is empty in the default build — the API handlers fall back
//! to streaming stubs when the feature is off, so the plumbing is always
//! exercisable end-to-end without the 10+ minute cold compile.
//!
//! Submodules and their gates:
//!   * `llm`  → `--features llm` (mistralrs)
//!   * `stt`  → `--features stt` (whisper-rs, pending)
//!   * `tts`  → `--features tts` (pending)

#[cfg(feature = "llm")]
pub mod llm;
