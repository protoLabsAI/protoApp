//! Real inference engines wired behind per-engine cargo features.
//!
//! Submodules listed here mirror what's actually exported. Today that's just
//! `llm` — the `stt` and `tts` features gate endpoint-side stubs in `api/`
//! but don't yet have their own engine modules in this directory. Those
//! modules land once the upstream crates are in a buildable state (see
//! `docs/explanation/engine-choices.md`).
//!
//! The API handlers fall back to streaming stubs when a feature is off, so
//! the plumbing is always exercisable end-to-end without the 10+ minute cold
//! compile of real engines.

#[cfg(feature = "llm")]
pub mod llm;
