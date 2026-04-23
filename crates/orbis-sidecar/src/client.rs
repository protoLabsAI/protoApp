//! WebSocket client that speaks the orbis-sidecar protocol.

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};

use crate::protocol::{IncomingMessage, OutgoingMessage};

pub struct Client {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Client {
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws, _resp) = tokio_tungstenite::connect_async(url).await?;
        Ok(Self { ws })
    }

    pub async fn send(&mut self, msg: OutgoingMessage) -> Result<()> {
        let text = serde_json::to_string(&msg)?;
        self.ws.send(Message::Text(text.into())).await?;
        Ok(())
    }

    /// Receive the next message. Returns `None` when the stream closes.
    pub async fn next(&mut self) -> Option<Result<IncomingMessage>> {
        loop {
            match self.ws.next().await? {
                Ok(Message::Text(t)) => {
                    let parsed = serde_json::from_str::<IncomingMessage>(&t)
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
        self.ws.close(None).await?;
        Ok(())
    }
}
