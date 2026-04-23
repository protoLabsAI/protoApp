//! Wire protocol between the Rust host and the ORBIS Python sidecar.
//!
//! JSON messages tagged by a `type` field (serde's `#[serde(tag = "type")]`).
//! Intentionally minimal — extend incrementally as ORBIS's capabilities grow.
//! Text-only for now; audio stays in Rust.

use serde::{Deserialize, Serialize};

/// Messages the Rust host sends to the Python agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutgoingMessage {
    /// User utterance, post-STT. The agent should reply with [`IncomingMessage::Token`]
    /// stream culminating in [`IncomingMessage::TurnEnd`].
    User { text: String },
    /// Free-form interruption — the agent should stop generating and
    /// truncate the pending context to this point.
    Interrupt,
    /// Push metadata to the agent (e.g. user settings, feature flags).
    Context {
        key: String,
        value: serde_json::Value,
    },
    /// Liveness probe; the sidecar should reply with the same request id.
    Ping { id: String },
}

/// Messages the Python agent sends back to the Rust host.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingMessage {
    /// A streamed reply token — feed to TTS as soon as it arrives.
    Token { text: String },
    /// Structured tool call the agent wants executed (search, memory, etc.).
    /// The Rust host routes this however it likes (another Tauri command,
    /// a service call, or the LLM surface in voice-core).
    ToolCall {
        name: String,
        args: serde_json::Value,
        id: String,
    },
    /// Signals end of an agent turn — flush TTS, re-open the mic.
    TurnEnd {
        finish_reason: Option<String>,
    },
    /// Response to [`OutgoingMessage::Ping`].
    Pong { id: String },
    /// Agent error. Not fatal on its own; the host decides how to surface it.
    Error { message: String },
}
