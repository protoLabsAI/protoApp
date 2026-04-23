//! orbis-sidecar — run the ORBIS Python agent as a Tauri sidecar and talk
//! to it over WebSocket.
//!
//! The Rust host (protoApp, orbis-tauri, a headless CLI, etc.) is responsible
//! for the audio hot path — VAD, STT, TTS, and the OpenAI-compatible LLM
//! surface from `protolabs-voice-core`. The Python sidecar owns the higher
//! level agent/memory/a2a logic and communicates via text-only messages.
//!
//! This split mirrors Feros's proven architecture and keeps GC pauses out of
//! the audio frame cadence.
//!
//! ## Usage
//!
//! ```no_run
//! use orbis_sidecar::{Sidecar, SpawnConfig, OutgoingMessage};
//! use futures::StreamExt;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let sidecar = Sidecar::spawn(SpawnConfig::default()).await?;
//! let mut client = sidecar.connect().await?;
//! client
//!     .send(OutgoingMessage::User { text: "Hello".into() })
//!     .await?;
//! while let Some(msg) = client.next().await {
//!     println!("{msg:?}");
//! }
//! # Ok(()) }
//! ```

pub mod client;
pub mod protocol;
pub mod spawn;

pub use client::Client;
pub use protocol::{IncomingMessage, OutgoingMessage};
pub use spawn::{Sidecar, SpawnConfig, SpawnError};
