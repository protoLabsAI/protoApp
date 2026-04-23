//! WebSocket client that speaks the orbis-sidecar protocol.
//!
//! Every network-facing await is wrapped in a timeout so a stalled sidecar
//! can't hang the host process. Timeouts are per-operation and default to
//! values that match typical agent pacing; override with the `*_TIMEOUT_*`
//! env vars below.

use std::time::Duration;

use anyhow::{Result, anyhow};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

use crate::protocol::{IncomingMessage, OutgoingMessage};

/// Upper bound on the initial WebSocket handshake.
/// Override via `ORBIS_WS_CONNECT_TIMEOUT_SECS`.
fn connect_timeout() -> Duration {
    duration_from_env("ORBIS_WS_CONNECT_TIMEOUT_SECS", 10)
}
/// Upper bound on a single `send` round-trip (serialization + write).
fn send_timeout() -> Duration {
    duration_from_env("ORBIS_WS_SEND_TIMEOUT_SECS", 15)
}
/// Upper bound on a single `next` await — generous because agent turns
/// naturally pause while the LLM thinks. Tune down in latency-sensitive apps.
fn recv_timeout() -> Duration {
    duration_from_env("ORBIS_WS_RECV_TIMEOUT_SECS", 120)
}
/// Upper bound on graceful close.
fn close_timeout() -> Duration {
    duration_from_env("ORBIS_WS_CLOSE_TIMEOUT_SECS", 5)
}

fn duration_from_env(var: &str, default_secs: u64) -> Duration {
    std::env::var(var)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(default_secs))
}

pub struct Client {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Client {
    pub async fn connect(url: &str) -> Result<Self> {
        let t = connect_timeout();
        let (ws, _resp) = timeout(t, tokio_tungstenite::connect_async(url))
            .await
            .map_err(|_| anyhow!("websocket connect timed out after {:?}", t))??;
        Ok(Self { ws })
    }

    pub async fn send(&mut self, msg: OutgoingMessage) -> Result<()> {
        let text = serde_json::to_string(&msg)?;
        let t = send_timeout();
        timeout(t, self.ws.send(Message::Text(text.into())))
            .await
            .map_err(|_| anyhow!("websocket send timed out after {:?}", t))??;
        Ok(())
    }

    /// Receive the next message. Returns `None` when the stream closes cleanly.
    /// A timeout yields `Some(Err(..))` so the caller can surface it as an error
    /// rather than mistaking it for a clean close.
    pub async fn next(&mut self) -> Option<Result<IncomingMessage>> {
        let t = recv_timeout();
        loop {
            let frame = match timeout(t, self.ws.next()).await {
                Ok(opt) => opt?,
                Err(_) => {
                    return Some(Err(anyhow!(
                        "timeout waiting for websocket message after {:?}",
                        t
                    )));
                }
            };
            match frame {
                Ok(Message::Text(txt)) => {
                    let parsed = serde_json::from_str::<IncomingMessage>(&txt)
                        .map_err(anyhow::Error::from);
                    return Some(parsed);
                }
                Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_))
                | Ok(Message::Frame(_)) => continue,
                Ok(Message::Close(_)) => return None,
                Err(e) => return Some(Err(anyhow::Error::from(e))),
            }
        }
    }

    pub async fn close(mut self) -> Result<()> {
        let t = close_timeout();
        timeout(t, self.ws.close(None))
            .await
            .map_err(|_| anyhow!("websocket close timed out after {:?}", t))??;
        Ok(())
    }
}
