//! protolabs-voice-core — OpenAI-compatible local HTTP server backed by
//! in-process Rust inference engines.
//!
//! This crate is the engine substrate shared between:
//!   * `protoApp` — standalone Tauri demo
//!   * `orbis-tauri` — the native ORBIS voice comm app
//!
//! The public surface is deliberately small: [`api::bind`] returns a server
//! future you can drive to completion, and [`api::router`] gives you the
//! Axum `Router` if you want to compose it into a larger service.
//!
//! Engines are feature-gated — see `Cargo.toml` for the `llm` / `stt` / `tts`
//! / `metal` / `cuda` flag combinations.

pub mod api;
mod engines;

pub use api::{bind, bind_with_state, router};
pub use api::state::AppState;
