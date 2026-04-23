use std::sync::Arc;

/// Shared state for the OpenAI-compatible server.
///
/// Right now this only holds the inference engines (behind a feature flag);
/// later it will also own the model-download cache and progress channels.
pub struct AppState {
    #[cfg(feature = "engines")]
    pub llm: tokio::sync::Mutex<Option<Arc<mistralrs::MistralRs>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "engines")]
            llm: tokio::sync::Mutex::new(None),
        }
    }
}

#[allow(dead_code)]
impl AppState {
    pub fn into_shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}
