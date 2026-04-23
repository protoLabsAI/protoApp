//! End-to-end smoke test for the OpenAI-compatible server.
//!
//! Spawns the Axum server on an ephemeral port, hits `/v1/models` for a
//! quick sanity check, then POSTs a streaming chat completion and verifies
//! we receive SSE frames ending in the literal `[DONE]` sentinel.
//!
//! Run with: `cargo test -p protolabs-voice-core --test api_smoke`

use futures::StreamExt;
use protolabs_voice_core::api;
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

    // Guard the stream with both a byte cap and a wall-clock deadline so a
    // missing [DONE] or a server bug can't hang CI.
    const MAX_BYTES: usize = 64 * 1024;
    const DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let read_loop = async {
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.expect("chunk");
            buf.push_str(&String::from_utf8_lossy(&bytes));
            if buf.contains("[DONE]") || buf.len() >= MAX_BYTES {
                break;
            }
        }
    };
    tokio::time::timeout(DEADLINE, read_loop)
        .await
        .expect("timed out waiting for [DONE]");

    assert!(buf.len() < MAX_BYTES, "read cap hit before [DONE]: {buf:?}");
    assert!(buf.contains("data: "), "expected SSE data: frames, got {buf:?}");
    assert!(buf.contains("chat.completion.chunk"), "expected object label");
    assert!(buf.contains("[DONE]"), "expected terminator");
}

#[tokio::test]
async fn transcriptions_accepts_multipart_and_returns_json() {
    let (addr, fut) = api::bind().await.expect("bind");
    tokio::spawn(fut);

    let fake_audio = vec![0u8; 1024];
    let form = reqwest::multipart::Form::new()
        .part(
            "file",
            reqwest::multipart::Part::bytes(fake_audio)
                .file_name("clip.wav")
                .mime_str("audio/wav")
                .unwrap(),
        )
        .text("model", "whisper-large-v3-turbo");

    let body: serde_json::Value = reqwest::Client::new()
        .post(format!("http://{addr}/v1/audio/transcriptions"))
        .multipart(form)
        .send()
        .await
        .expect("post")
        .json()
        .await
        .expect("json");

    let text = body["text"].as_str().expect("text field");
    assert!(!text.is_empty());
    assert!(
        text.contains("1024 bytes") || text.contains("stub") || text.contains("pending"),
        "stub should mention byte count, got: {text}"
    );
}

#[tokio::test]
async fn speech_returns_audio_bytes() {
    let (addr, fut) = api::bind().await.expect("bind");
    tokio::spawn(fut);

    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/v1/audio/speech"))
        .json(&json!({
            "model": "kokoro-82m",
            "input": "Hello from the local server.",
            "voice": "af_heart"
        }))
        .send()
        .await
        .expect("post");

    assert!(resp.status().is_success());
    let bytes = resp.bytes().await.expect("bytes");
    assert!(bytes.len() > 44, "must include at least a WAV header");
    assert_eq!(&bytes[0..4], b"RIFF", "expected a WAV file");
    assert_eq!(&bytes[8..12], b"WAVE");
}

#[tokio::test]
async fn speech_rejects_empty_input() {
    let (addr, fut) = api::bind().await.expect("bind");
    tokio::spawn(fut);

    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/v1/audio/speech"))
        .json(&json!({ "input": "" }))
        .send()
        .await
        .expect("post");
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn chat_completions_rejects_unknown_model() {
    let (addr, fut) = api::bind().await.expect("bind");
    tokio::spawn(fut);

    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/v1/chat/completions"))
        .json(&json!({
            "model": "some-model-we-dont-serve",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .send()
        .await
        .expect("post");
    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["error"]["code"], "model_not_found");
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
