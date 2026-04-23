# OpenAI-compatible API

Served by `protolabs-voice-core` at `http://127.0.0.1:<ephemeral>/v1/*`.
The port is chosen at startup and surfaced to the frontend via the
[`get_api_base_url`](./tauri-commands.md#get_api_base_url) Tauri command.

Request/response shapes match OpenAI's specification exactly so any
OpenAI SDK works unchanged.

## Endpoints

| Method | Path | Purpose | Status |
|---|---|---|---|
| `GET` | `/v1/models` | List locally advertised models | real |
| `POST` | `/v1/chat/completions` | Chat completion (JSON or SSE) | real (with `--features llm`); stub otherwise |
| `POST` | `/v1/audio/transcriptions` | STT (multipart) | stub (see [status](./cargo-features.md#stt)) |
| `POST` | `/v1/audio/speech` | TTS (JSON → audio bytes) | stub (see [status](./cargo-features.md#tts)) |
| `GET` | `/healthz` | Liveness | real |

## `GET /v1/models`

**Response** (`application/json`):

```json
{
  "object": "list",
  "data": [
    { "id": "gemma-4-e2b", "object": "model", "created": 0, "owned_by": "google" },
    { "id": "gemma-4-e4b", "object": "model", "created": 0, "owned_by": "google" },
    { "id": "whisper-large-v3-turbo", "object": "model", "created": 0, "owned_by": "openai" },
    { "id": "kokoro-82m", "object": "model", "created": 0, "owned_by": "hexgrad" }
  ]
}
```

## `POST /v1/chat/completions`

**Request** (`application/json`):

```json
{
  "model": "gemma-4-e2b",
  "messages": [
    { "role": "system", "content": "You are helpful." },
    { "role": "user",   "content": "hello" }
  ],
  "stream": true,
  "temperature": 0.7,
  "max_tokens": 512
}
```

**Non-streaming response** (`stream: false`):

```json
{
  "id": "chatcmpl-<uuid>",
  "object": "chat.completion",
  "created": 1745355600,
  "model": "gemma-4-e2b",
  "choices": [
    {
      "index": 0,
      "message": { "role": "assistant", "content": "Hi there." },
      "finish_reason": "stop"
    }
  ]
}
```

**Streaming response** (`stream: true`) — `text/event-stream`:

```
data: {"id":"...","object":"chat.completion.chunk","created":...,"model":"gemma-4-e2b","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"...","object":"chat.completion.chunk",...,"choices":[{"index":0,"delta":{"content":"Hi "},"finish_reason":null}]}

data: {"id":"...","object":"chat.completion.chunk",...,"choices":[{"index":0,"delta":{"content":"there."},"finish_reason":null}]}

data: {"id":"...","object":"chat.completion.chunk",...,"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]

```

## `POST /v1/audio/transcriptions`

**Request** (`multipart/form-data`):

| Field | Required | Notes |
|---|---|---|
| `file` | yes | Audio bytes (wav/mp3/webm) |
| `model` | no | Default: `whisper-large-v3-turbo` |
| `response_format` | no | `json` (default) or `text` |

**Response** (`response_format=json`):

```json
{ "text": "Hello world." }
```

## `POST /v1/audio/speech`

**Request** (`application/json`):

```json
{
  "model": "kokoro-82m",
  "input": "Hello world.",
  "voice": "af_heart",
  "response_format": "wav"
}
```

**Response**: raw audio bytes. `Content-Type: audio/wav`. Stub always
returns 1 s of silence at 24 kHz mono f32.

## `GET /healthz`

```
200 OK
Content-Type: text/plain

ok
```

## Errors

4xx: the endpoint rejects the request (missing `file`, empty `input`,
bad JSON). Body is a plain text reason.

5xx: real engine failure. Stub fallback emits a best-effort response
rather than a 500 — check logs for the underlying error.

## CORS

Everything is allowed (`Any`/`Any`/`Any`). Safe because the server
binds to `127.0.0.1` only; no remote origin can reach it.
