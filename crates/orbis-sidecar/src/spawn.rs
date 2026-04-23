//! Spawn + lifecycle for the Python sidecar.
//!
//! Protocol: the child is expected to:
//!   1. Bind its WebSocket server on an ephemeral port.
//!   2. Print a single line `ORBIS_READY ws://127.0.0.1:<port>` to stdout.
//!   3. Stay in the foreground until it receives SIGTERM.
//!
//! Any subsequent stdout/stderr is forwarded to tracing at INFO / WARN.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("failed to launch sidecar binary: {0}")]
    Launch(#[from] std::io::Error),
    #[error("sidecar exited before signalling readiness")]
    EarlyExit,
    #[error("timed out waiting for ORBIS_READY line after {0:?}")]
    ReadinessTimeout(Duration),
    #[error("could not parse ORBIS_READY line: {0:?}")]
    UnparsableReadyLine(String),
}

#[derive(Debug, Clone)]
pub struct SpawnConfig {
    /// Absolute path to the sidecar binary (PyApp-bundled ORBIS, `python3 -m orbis`,
    /// etc.). Defaults to the string `"orbis"`, resolved via `$PATH`.
    pub program: std::path::PathBuf,
    /// Extra arguments to pass on the command line (e.g. `["--log-level", "debug"]`).
    pub args: Vec<String>,
    /// How long to wait for the readiness line before giving up.
    pub readiness_timeout: Duration,
    /// Extra environment variables to set.
    pub env: Vec<(String, String)>,
}

impl Default for SpawnConfig {
    fn default() -> Self {
        Self {
            program: std::path::PathBuf::from("orbis"),
            args: vec![],
            readiness_timeout: Duration::from_secs(30),
            env: vec![],
        }
    }
}

/// A running sidecar. Drop to terminate (best-effort SIGKILL).
pub struct Sidecar {
    /// `ws://127.0.0.1:<port>` discovered from the ORBIS_READY line.
    pub ws_url: String,
    child: Arc<Mutex<Option<Child>>>,
}

impl Sidecar {
    pub async fn spawn(cfg: SpawnConfig) -> Result<Self, SpawnError> {
        let mut cmd = Command::new(&cfg.program);
        cmd.args(&cfg.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in &cfg.env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn()?;

        let stdout = child.stdout.take().expect("piped");
        let stderr = child.stderr.take().expect("piped");

        // Pump stderr to tracing.
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(target: "orbis.sidecar.stderr", "{line}");
            }
        });

        // Look for the ready line on stdout.
        let mut lines = BufReader::new(stdout).lines();
        let ws_url = match timeout(cfg.readiness_timeout, async {
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!(target: "orbis.sidecar.stdout", "{line}");
                if let Some(url) = line.strip_prefix("ORBIS_READY ") {
                    return Ok::<_, SpawnError>(url.trim().to_string());
                }
            }
            Err(SpawnError::EarlyExit)
        })
        .await
        {
            Ok(res) => res?,
            Err(_) => {
                let _ = child.kill().await;
                return Err(SpawnError::ReadinessTimeout(cfg.readiness_timeout));
            }
        };

        if !is_loopback_ws_url(&ws_url) {
            let _ = child.kill().await;
            return Err(SpawnError::UnparsableReadyLine(ws_url));
        }

        // Keep forwarding stdout after ready.
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!(target: "orbis.sidecar.stdout", "{line}");
            }
        });

        Ok(Self {
            ws_url,
            child: Arc::new(Mutex::new(Some(child))),
        })
    }

    /// Open a new WebSocket connection to the sidecar. Safe to call multiple
    /// times — each returns an independent [`crate::Client`].
    pub async fn connect(&self) -> anyhow::Result<crate::Client> {
        crate::Client::connect(&self.ws_url).await
    }

    /// Wait up to `grace` for the sidecar to exit after we ask it to close,
    /// then SIGKILL if it hasn't.
    ///
    /// The "ask it to close" path is currently a hard kill — if you need
    /// graceful SIGTERM, wire it via an `Interrupt`/`Shutdown` WebSocket
    /// message on your own protocol before calling this.
    pub async fn shutdown(&self, grace: Duration) {
        let mut guard = self.child.lock().await;
        if let Some(mut child) = guard.take() {
            // start_kill sends SIGKILL on Unix / TerminateProcess on Windows.
            let _ = child.start_kill();
            let _ = timeout(grace, child.wait()).await;
        }
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        // kill_on_drop(true) on the Command takes care of the async case.
        // Nothing else to do — `shutdown()` is the explicit path.
    }
}

/// Parse + validate a sidecar-supplied ws URL.
/// Requires `ws://`/`wss://` and a loopback host — prevents a compromised
/// child from redirecting our client to a remote server.
fn is_loopback_ws_url(raw: &str) -> bool {
    let url = match url::Url::parse(raw) {
        Ok(u) => u,
        Err(_) => return false,
    };
    if !matches!(url.scheme(), "ws" | "wss") {
        return false;
    }
    match url.host() {
        Some(url::Host::Domain(d)) => d == "localhost",
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_loopback_ws_url;

    #[test]
    fn accepts_loopback_ws_urls() {
        for u in [
            "ws://127.0.0.1:19999/ws",
            "ws://localhost:8080",
            "wss://127.0.0.1:8443/a/b",
            "ws://[::1]:9000",
            "ws://127.5.6.7:1/",
        ] {
            assert!(is_loopback_ws_url(u), "should accept: {u}");
        }
    }

    #[test]
    fn rejects_non_loopback_or_malformed() {
        for u in [
            "ws://evil.example.com/ws",
            "http://127.0.0.1:19999",   // wrong scheme
            "ORBIS_READY=not-a-valid-url",
            "ws://10.0.0.1/",
            "",
        ] {
            assert!(!is_loopback_ws_url(u), "should reject: {u}");
        }
    }
}
