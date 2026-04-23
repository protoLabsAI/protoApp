//! Real inference engines wired behind the `engines` cargo feature.
//!
//! This module is empty in the default build — the API handlers fall back to
//! streaming stubs when the feature is off, so the plumbing is always
//! exercisable end-to-end without the 10+ minute cold compile.

#[cfg(feature = "llm")]
pub mod llm;
