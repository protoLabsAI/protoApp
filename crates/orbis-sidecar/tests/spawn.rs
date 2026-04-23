//! Exercises `Sidecar::spawn` against a shell fixture that prints the
//! `ORBIS_READY ws://...` line and then hangs. Verifies:
//!   * stdout line is parsed and `ws_url` is populated
//!   * process is killed on `shutdown()` / drop
//!
//! Unix-only because the fixture uses `sh -c`.

#![cfg(unix)]

use std::time::Duration;

use orbis_sidecar::{Sidecar, SpawnConfig};

#[tokio::test]
async fn parses_ready_line_and_shuts_down() {
    let cfg = SpawnConfig {
        program: std::path::PathBuf::from("sh"),
        args: vec![
            "-c".into(),
            // Print the readiness line, then block until killed.
            "echo 'ORBIS_READY ws://127.0.0.1:19999/ws'; exec cat".into(),
        ],
        readiness_timeout: Duration::from_secs(5),
        env: vec![],
    };

    let sidecar = Sidecar::spawn(cfg).await.expect("spawn");
    assert_eq!(sidecar.ws_url, "ws://127.0.0.1:19999/ws");
    sidecar.shutdown(Duration::from_secs(2)).await;
}

#[tokio::test]
async fn errors_on_readiness_timeout() {
    let cfg = SpawnConfig {
        program: std::path::PathBuf::from("sh"),
        args: vec!["-c".into(), "sleep 5".into()], // never prints ready line
        readiness_timeout: Duration::from_millis(300),
        env: vec![],
    };
    let res = Sidecar::spawn(cfg).await;
    assert!(matches!(
        res,
        Err(orbis_sidecar::SpawnError::ReadinessTimeout(_))
    ));
}

#[tokio::test]
async fn errors_when_child_exits_before_ready() {
    let cfg = SpawnConfig {
        program: std::path::PathBuf::from("sh"),
        args: vec!["-c".into(), "exit 0".into()],
        readiness_timeout: Duration::from_secs(2),
        env: vec![],
    };
    let res = Sidecar::spawn(cfg).await;
    assert!(matches!(res, Err(orbis_sidecar::SpawnError::EarlyExit)));
}
