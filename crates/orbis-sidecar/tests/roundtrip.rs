//! Round-trip the protocol against an in-test axum WebSocket server that
//! stands in for the real ORBIS Python sidecar. Verifies:
//!   * Client sends `OutgoingMessage::User` as proper tagged JSON
//!   * Client parses `IncomingMessage::Token` / `TurnEnd` chunks
//!   * `connect()` succeeds against a live ws:// endpoint

use std::net::SocketAddr;

use axum::Router;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::{Message as AxMsg, WebSocket};
use axum::response::IntoResponse;
use axum::routing::get;
use orbis_sidecar::{Client, IncomingMessage, OutgoingMessage};

async fn mock_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let app = Router::new().route("/ws", get(ws_handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (addr, handle)
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    // Echo-reply pattern: whatever user text arrives, reply with 3 tokens then turn_end.
    while let Some(Ok(msg)) = socket.recv().await {
        if let AxMsg::Text(t) = msg {
            let parsed: OutgoingMessage = serde_json::from_str(&t).unwrap();
            if let OutgoingMessage::User { text } = parsed {
                for word in ["got:", text.as_str(), "done."].iter() {
                    let reply = IncomingMessage::Token {
                        text: format!("{word} "),
                    };
                    let s = serde_json::to_string(&reply).unwrap();
                    socket.send(AxMsg::Text(s.into())).await.unwrap();
                }
                let end = IncomingMessage::TurnEnd {
                    finish_reason: Some("stop".into()),
                };
                socket
                    .send(AxMsg::Text(serde_json::to_string(&end).unwrap().into()))
                    .await
                    .unwrap();
            }
        }
    }
}

#[tokio::test]
async fn client_roundtrip_user_to_turn_end() {
    let (addr, _server) = mock_server().await;
    let url = format!("ws://{}/ws", addr);

    let mut client = Client::connect(&url).await.expect("connect");
    client
        .send(OutgoingMessage::User {
            text: "hello".into(),
        })
        .await
        .expect("send");

    let mut tokens: Vec<String> = Vec::new();
    let mut got_turn_end = false;
    while let Some(msg) = client.next().await {
        match msg.expect("decode") {
            IncomingMessage::Token { text } => tokens.push(text),
            IncomingMessage::TurnEnd { finish_reason } => {
                assert_eq!(finish_reason.as_deref(), Some("stop"));
                got_turn_end = true;
                break;
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    assert!(got_turn_end);
    let joined = tokens.concat();
    assert!(
        joined.contains("got:") && joined.contains("hello") && joined.contains("done."),
        "expected echo of 'hello', got: {joined:?}"
    );
}
