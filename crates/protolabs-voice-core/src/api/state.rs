use std::sync::Arc;

use crate::engines::events::{NullEmitter, StatusEmitter};

/// Shared state for the OpenAI-compatible server.
///
/// The LLM slot is a `OnceCell` so the first chat request pays the load cost
/// and every concurrent caller afterwards waits on the same cell. It holds
/// `Arc<LlamaModel>` rather than a bare model so handlers can cheaply clone
/// into `spawn_blocking` workers without moving out of the cell.
///
/// `emitter` is how engine modules publish life-cycle events
/// ("downloading", "loading", "ready", "error") up to the host — a Tauri
/// app forwards them to the webview; tests use the no-op `NullEmitter`.
pub struct AppState {
    #[cfg(feature = "llm")]
    pub llm: tokio::sync::OnceCell<Arc<llama_cpp_2::model::LlamaModel>>,
    pub emitter: Arc<dyn StatusEmitter>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            #[cfg(feature = "llm")]
            llm: tokio::sync::OnceCell::new(),
            emitter: Arc::new(NullEmitter),
        }
    }
}

impl AppState {
    /// Kept for call-site clarity; equivalent to `AppState::default()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a state that forwards engine life-cycle events through the
    /// supplied sink. `Arc<dyn StatusEmitter>` is cheaply cloneable, so
    /// engine modules can hand a fresh handle to each spawn without locks.
    pub fn with_emitter(emitter: Arc<dyn StatusEmitter>) -> Self {
        Self {
            emitter,
            ..Default::default()
        }
    }
}

#[allow(dead_code)]
impl AppState {
    pub fn into_shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

#[cfg(feature = "llm")]
impl AppState {
    /// Resolve the default LLM instance, loading it on first use.
    /// Returns a borrow tied to the AppState (always held via Arc); callers
    /// should `.clone()` the inner Arc before moving into `spawn_blocking`.
    pub async fn llm_or_load(
        &self,
    ) -> anyhow::Result<&Arc<llama_cpp_2::model::LlamaModel>> {
        let emitter = self.emitter.clone();
        self.llm
            .get_or_try_init(|| async move {
                crate::engines::llm::load_default(&emitter).await
            })
            .await
    }
}
