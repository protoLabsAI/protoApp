//! protoapp-agent — in-process [zeroclaw] agent bridge.
//!
//! On iPad the agent must run **inside** the host process: iOS forbids
//! spawning an external interpreter, so the ORBIS Python sidecar can't
//! survive the port. This crate embeds `zeroclaw-runtime` as a library and
//! drives one agent turn per call. The agent's model provider points at
//! `protolabs-voice-core`'s local OpenAI-compatible server, so inference
//! stays on-device — zeroclaw is the brain, voice-core is the local model.
//!
//! ```ignore
//! let reply = protoapp_agent::ask(
//!     "http://127.0.0.1:53217",     // voice-core base URL
//!     "Qwen3-4B-Instruct-2507",     // model the local server serves
//!     "What's on my calendar?",
//!     Some("session-1"),
//! ).await?;
//! ```
//!
//! [zeroclaw]: https://github.com/protoLabsAI/zeroclaw

use std::path::PathBuf;
use std::sync::Once;

use anyhow::{Context, Result};

/// zeroclaw config that wires a single `default` agent to the local
/// voice-core server as an OpenAI-compatible provider. Shape mirrors
/// zeroclaw's own `docs/book/src/_snippets/minimal-config.toml`:
/// `[providers.models.<family>.<alias>]` + `[agents.<alias>]` +
/// `[risk_profiles.<alias>]`. `{{BASE_URL}}` / `{{MODEL}}` are filled at
/// runtime; the rendered file is written where the user can inspect and
/// tweak it (see [`config_dir`]).
const CONFIG_TEMPLATE: &str = r#"schema_version = 3

# protoApp's local OpenAI-compatible server (protolabs-voice-core). The key
# is ignored by that server but zeroclaw requires the field to be present.
[providers.models.openai.local]
model = "{{MODEL}}"
api_key = "local"
uri = "{{BASE_URL}}/v1"

[agents.default]
model_provider = "openai.local"
risk_profile = "default"

[risk_profiles.default]
level = "supervised"
"#;

/// Directory holding the generated `config.toml` plus zeroclaw's own state
/// (memory db, cost records). Pointed at by `ZEROCLAW_CONFIG_DIR` so
/// `Config::load_or_init` reads it instead of `~/.zeroclaw`.
fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir().context("no config dir available on this platform")?;
    Ok(base.join("protoapp").join("agent"))
}

/// Render the config for `(base_url, model)` and write it to disk, then
/// point `ZEROCLAW_CONFIG_DIR` at the directory. Rewrites `config.toml` only
/// when the rendered content differs, so a hand-edit during testing survives
/// repeated calls with the same inputs.
fn ensure_config(base_url: &str, model: &str) -> Result<PathBuf> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir).context("create agent config dir")?;

    let rendered = CONFIG_TEMPLATE
        .replace("{{BASE_URL}}", base_url.trim_end_matches('/'))
        .replace("{{MODEL}}", model);
    let path = dir.join("config.toml");
    let differs = std::fs::read_to_string(&path)
        .map(|existing| existing != rendered)
        .unwrap_or(true);
    if differs {
        std::fs::write(&path, &rendered).context("write zeroclaw config.toml")?;
        tracing::info!(dir = %dir.display(), "wrote zeroclaw agent config");
    }

    // zeroclaw resolves its config dir from this env var (falling back to
    // ~/.zeroclaw). Set once; the value is stable for the process.
    static SET_ENV: Once = Once::new();
    SET_ENV.call_once(|| {
        // SAFETY: called once, before any zeroclaw config load, and no other
        // thread mutates the environment in this app.
        std::env::set_var("ZEROCLAW_CONFIG_DIR", &dir);
    });

    Ok(dir)
}

/// Drive one agent turn in-process and return the assistant's reply text.
///
/// `base_url` is voice-core's server root (e.g. `http://127.0.0.1:<port>`);
/// `/v1` is appended for the OpenAI wire API. `model` must match a model the
/// local server serves. `session_id` threads zeroclaw's conversation memory
/// across turns; pass `None` for a one-shot.
pub async fn ask(
    base_url: &str,
    model: &str,
    message: &str,
    session_id: Option<&str>,
) -> Result<String> {
    ensure_config(base_url, model)?;

    let config = zeroclaw_config::schema::Config::load_or_init()
        .await
        .context("load zeroclaw config")?;

    zeroclaw_runtime::agent::process_message(config, "default", message, session_id)
        .await
        .context("zeroclaw process_message")
}
