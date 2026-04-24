//! Cross-cutting engine status plumbing.
//!
//! Engines publish life-cycle events ("loading", "downloading", "ready",
//! "error") through a [`StatusEmitter`] trait object stored in
//! [`crate::api::state::AppState`]. The host process (Tauri, a CLI, a test)
//! decides where those events actually go — a Tauri app forwards them to a
//! browser `listen("engine-status", …)`, a CLI might just log them, and a
//! unit test uses the [`NullEmitter`] default.
//!
//! Event shape (on the wire):
//!
//! ```json
//! { "engine": "llm",  "phase": "loading" }
//! { "engine": "llm",  "phase": "ready" }
//! { "engine": "stt",  "phase": "downloading", "bytes": 12345, "total": 67890 }
//! { "engine": "tts",  "phase": "error", "message": "ort: ..." }
//! ```

use std::sync::Arc;

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    Llm,
    Stt,
    Tts,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    /// Engine is spinning up — model load, warmup, etc. No progress available.
    Loading,
    /// Weights or assets are being pulled from an external source. `total` is
    /// `None` when the source didn't advertise Content-Length.
    Downloading {
        bytes: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        total: Option<u64>,
    },
    /// Ready to serve requests.
    Ready,
    /// Terminal failure for this engine. Host should surface + stop showing a
    /// spinner. A subsequent successful [`Phase::Ready`] clears it.
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub engine: Engine,
    #[serde(flatten)]
    pub phase: Phase,
}

/// Host-side hook for forwarding [`EngineStatus`] to wherever the UI lives.
/// Implementations must be `Send + Sync` so engines can emit from any async
/// task without adapter layers.
pub trait StatusEmitter: Send + Sync + 'static {
    fn emit(&self, status: EngineStatus);
}

/// Default used by tests and by hosts that don't care about these events.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEmitter;

impl StatusEmitter for NullEmitter {
    fn emit(&self, _status: EngineStatus) {}
}

/// Convenience shortcuts so engine modules stay readable.
pub fn emit_loading(sink: &Arc<dyn StatusEmitter>, engine: Engine) {
    sink.emit(EngineStatus {
        engine,
        phase: Phase::Loading,
    });
}

pub fn emit_downloading(
    sink: &Arc<dyn StatusEmitter>,
    engine: Engine,
    bytes: u64,
    total: Option<u64>,
) {
    sink.emit(EngineStatus {
        engine,
        phase: Phase::Downloading { bytes, total },
    });
}

pub fn emit_ready(sink: &Arc<dyn StatusEmitter>, engine: Engine) {
    sink.emit(EngineStatus {
        engine,
        phase: Phase::Ready,
    });
}

pub fn emit_error(sink: &Arc<dyn StatusEmitter>, engine: Engine, message: impl Into<String>) {
    sink.emit(EngineStatus {
        engine,
        phase: Phase::Error {
            message: message.into(),
        },
    });
}
