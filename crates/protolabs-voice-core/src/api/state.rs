use std::sync::Arc;

/// Shared state for the OpenAI-compatible server.
///
/// The LLM slot is a `OnceCell` so the first chat request pays the load cost
/// and every concurrent caller afterwards waits on the same cell.
#[derive(Default)]
pub struct AppState {
    #[cfg(feature = "llm")]
    pub llm: tokio::sync::OnceCell<mistralrs::Model>,
}

impl AppState {
    /// Kept for call-site clarity; equivalent to `AppState::default()`.
    pub fn new() -> Self {
        Self::default()
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
    /// Returns a borrow tied to the AppState (which is always held via Arc),
    /// so callers should keep the Arc alive for the duration of the borrow.
    pub async fn llm_or_load(&self) -> anyhow::Result<&mistralrs::Model> {
        self.llm
            .get_or_try_init(crate::engines::llm::load_default)
            .await
    }
}
