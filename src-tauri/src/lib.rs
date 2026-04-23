use std::net::SocketAddr;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands};

pub mod api;
mod commands;

/// Injected into Tauri managed state so commands (and the frontend) can
/// discover the ephemeral port the local OpenAI-compatible server is bound to.
pub struct ApiServer {
    pub addr: SocketAddr,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tower_http=warn")),
        )
        .try_init();

    let specta_builder = Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::greet,
        commands::get_api_base_url,
    ]);

    #[cfg(debug_assertions)]
    specta_builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("failed to export TypeScript bindings");

    // Tauri's default runtime isn't tokio, so we own one for the API server
    // and leak it — the server lives for the app's lifetime regardless.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");
    let handle = rt.handle().clone();
    Box::leak(Box::new(rt));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            let (addr, server_fut) = handle.block_on(async { api::bind().await })?;
            tracing::info!(%addr, "OpenAI-compatible server listening");
            handle.spawn(async move {
                if let Err(e) = server_fut.await {
                    tracing::error!(?e, "api server exited with error");
                }
            });
            app.manage(ApiServer { addr });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
