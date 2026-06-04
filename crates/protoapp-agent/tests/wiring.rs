//! Wiring test: drive a real agent turn against voice-core's stub server.
//!
//! This exercises the whole in-process path without the 2.5 GB model or a GUI:
//! `protoapp_agent::ask` → zeroclaw config + provider → HTTP →
//! voice-core's `/v1/chat/completions` (stub echo, since the `llm` feature is
//! off in this test build). A non-error return proves zeroclaw successfully
//! built the request, reached the local server, and parsed the reply — the
//! exact handshake that real testing needs to confirm.
//!
//! Run: `cargo test -p protoapp-agent --test wiring -- --nocapture`

use std::sync::Arc;

use protolabs_voice_core::{bind_with_state, AppState};

#[tokio::test]
async fn agent_reaches_voice_core_stub() {
    // Point zeroclaw's config/state at an isolated temp dir so the test
    // doesn't touch the developer's real ~/.config/protoapp/agent.
    let tmp = std::env::temp_dir().join(format!("protoapp-agent-wiring-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp);
    // SAFETY: single-threaded test setup, before any zeroclaw config load.
    std::env::set_var("ZEROCLAW_CONFIG_DIR", &tmp);

    // Stand up the in-process OpenAI-compatible server (stub engines).
    let (addr, server) = bind_with_state(Arc::new(AppState::new()))
        .await
        .expect("bind voice-core server");
    tokio::spawn(server);
    let base_url = format!("http://{addr}");

    // Catalog id (lowercase) — anything else 404s at voice-core.
    let result =
        protoapp_agent::ask(&base_url, "qwen3-4b-instruct-2507", "Say hello.", Some("wiring-1"))
            .await;

    match result {
        Ok(reply) => {
            eprintln!("agent reply: {reply:?}");
            // The stub echoes rather than reasons; we only assert the round
            // trip produced *some* text, i.e. the provider handshake worked.
            assert!(!reply.trim().is_empty(), "agent returned an empty reply");
        }
        Err(e) => panic!("agent turn failed (wiring issue): {e:#}"),
    }
}
