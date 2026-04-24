use std::net::SocketAddr;
use std::sync::Arc;

use protolabs_voice_core::engines::events::{EngineStatus, StatusEmitter};
use protolabs_voice_core::{self as voice_core, AppState};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_specta::{Builder, collect_commands};

mod commands;

/// Injected into Tauri managed state so commands (and the frontend) can
/// discover the ephemeral port the local OpenAI-compatible server is bound to.
pub struct ApiServer {
    pub addr: SocketAddr,
}

/// Forwards voice-core engine life-cycle events into the webview via
/// Tauri's event bus. The frontend listens with
/// `listen<EngineStatus>("engine-status", …)`.
struct TauriEmitter {
    handle: AppHandle,
}

impl StatusEmitter for TauriEmitter {
    fn emit(&self, status: EngineStatus) {
        if let Err(e) = self.handle.emit("engine-status", status) {
            tracing::warn!(?e, "failed to emit engine-status");
        }
    }
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
        // Hide the window instead of tearing down the whole app when the user
        // clicks the red traffic light or hits Cmd+W. The API server (+ any
        // mid-download model weights) stay alive. Explicit "Quit" in the
        // tray menu is the real exit path.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            // --- API server ----------------------------------------------
            // Thread an `AppHandle` into voice-core's AppState so the
            // engine modules can emit life-cycle events straight to the
            // webview (download progress, ready, error).
            let emitter: Arc<dyn StatusEmitter> = Arc::new(TauriEmitter {
                handle: app.handle().clone(),
            });
            let state = Arc::new(AppState::with_emitter(emitter));
            let (addr, server_fut) = handle.block_on(async {
                voice_core::bind_with_state(state.clone()).await
            })?;
            tracing::info!(%addr, "OpenAI-compatible server listening");

            let app_handle = app.handle().clone();
            handle.spawn(async move {
                match server_fut.await {
                    Ok(()) => tracing::info!("api server exited cleanly"),
                    Err(e) => {
                        tracing::error!(?e, "api server exited with error");
                        let _ = app_handle.emit(
                            "api-server-error",
                            serde_json::json!({ "error": e.to_string() }),
                        );
                    }
                }
            });
            app.manage(ApiServer { addr });

            // --- Tray icon + menu ----------------------------------------
            let show = MenuItem::with_id(app, "show", "Show protoApp", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show, &hide, &quit])?;

            let default_icon = app
                .default_window_icon()
                .cloned()
                .expect("tauri.conf.json icon entry should populate a default icon");

            TrayIconBuilder::with_id("main")
                .tooltip("protoApp — local OpenAI-compatible server")
                .icon(default_icon)
                .menu(&tray_menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
