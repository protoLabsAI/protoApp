//! End-to-end smoke test for the OpenAI-compatible server.
//!
//! Spawns the Axum server on an ephemeral port, hits `/v1/models` for a
//! quick sanity check, then POSTs a streaming chat completion and verifies
//! we receive SSE frames ending in the literal `[DONE]` sentinel.
//!
//! Run with: `cargo test -p protoapp --test api_smoke`

use futures::StreamExt;
use protoapp_lib::api;
use serde_json::json;

#[tokio::test]
async fn models_endpoint_returns_defaults() {
    let (addr, fut) = api::bind().await.expect("bind");
    tokio::spawn(fut);

    let body: serde_json::Value = reqwest::get(format!("http://{addr}/v1/models"))
        .await
        .expect("request")
        .json()
        .await
        .expect("json");

    assert_eq!(body["object"], "list");
    let ids: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["id"].as_str())
        .collect();
    assert!(ids.contains(&"gemma-4-e2b"), "missing gemma-4-e2b in {ids:?}");
}

#[tokio::test]
async fn chat_completions_streams_sse_and_terminates() {
    let (addr, fut) = api::bind().await.expect("bind");
    tokio::spawn(fut);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{addr}/v1/chat/completions"))
        .json(&json!({
            "model": "gemma-4-e2b",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true
        }))
        .send()
        .await
        .expect("post");
    assert!(resp.status().is_success());

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.expect("chunk");
        buf.push_str(&String::from_utf8_lossy(&bytes));
        if buf.contains("[DONE]") {
            break;
        }
    }

    assert!(buf.contains("data: "), "expected SSE data: frames, got {buf:?}");
    assert!(buf.contains("chat.completion.chunk"), "expected object label");
    assert!(buf.contains("[DONE]"), "expected terminator");
}

#[tokio::test]
async fn chat_completions_json_mode_returns_body() {
    let (addr, fut) = api::bind().await.expect("bind");
    tokio::spawn(fut);

    let client = reqwest::Client::new();
    let body: serde_json::Value = client
        .post(format!("http://{addr}/v1/chat/completions"))
        .json(&json!({
            "model": "gemma-4-e2b",
            "messages": [{"role": "user", "content": "ping"}]
        }))
        .send()
        .await
        .expect("post")
        .json()
        .await
        .expect("json");

    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], "gemma-4-e2b");
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .expect("content");
    assert!(content.contains("ping"), "stub should echo user msg: {content}");
}
