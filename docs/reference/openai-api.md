# OpenAI-compatible API

Served by `protolabs-voice-core` at `http://127.0.0.1:<ephemeral>/v1/*`.
The port is chosen at startup and surfaced to the frontend via the
[`get_api_base_url`](./tauri-commands.md#get_api_base_url) Tauri command.

Request/response shapes are OpenAI-compatible — compatible enough that
the standard OpenAI SDKs work without changes — but not a 100 % replica.
Known deviations: some endpoints are stubbed (see "Status" column below),
error bodies mix JSON and plain text per endpoint, and `audio/speech`
always returns WAV even when the client asks for mp3 (the advisory
`x-protoapp-note` header flags this).

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
    { "id": "qwen3-4b-instruct-2507", "object": "model", "created": 0, "owned_by": "qwen" },
    { "id": "whisper-large-v3-turbo", "object": "model", "created": 0, "owned_by": "openai" },
    { "id": "kokoro-82m", "object": "model", "created": 0, "owned_by": "hexgrad" }
  ]
}
```

## `POST /v1/chat/completions`

**Request** (`application/json`):

```json
{
  "model": "qwen3-4b-instruct-2507",
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
  "model": "qwen3-4b-instruct-2507",
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
data: {"id":"...","object":"chat.completion.chunk","created":...,"model":"qwen3-4b-instruct-2507","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

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

4xx response bodies are **mixed**:

- `POST /v1/chat/completions` returns OpenAI-style JSON:
  ```json
  {
    "error": {
      "message": "model `foo` not found",
      "type": "invalid_request_error",
      "param": "model",
      "code": "model_not_found"
    }
  }
  ```
- `POST /v1/audio/transcriptions` returns plain text reasons for validation failures (e.g. `` `input` must not be empty ``, `unsupported response_format`, `invalid multipart body`). No model-lookup JSON yet.
- `POST /v1/audio/speech` returns plain text reasons for simple validation failures and OpenAI-style JSON for model lookup failures (unknown TTS model → 404 with `"code": "model_not_found"`).

5xx: real engine failure. `/v1/chat/completions` returns the same
OpenAI JSON shape with `"code": "backend_failure"`. The stub fallback
emits a best-effort 200 rather than a 5xx — check the logs for the
underlying error if you see suspicious stub-like replies.

Either way, the `Content-Type` header distinguishes: `application/json`
for the structured shape, `text/plain` otherwise.

## CORS

The server binds to `127.0.0.1`, which keeps the socket off the network
but **not** off other pages in the user's browser — any site a user
visits can issue `fetch("http://127.0.0.1:<port>/...")`. The router
therefore enforces a loopback-only CORS allowlist:

- `allow_origin`: loopback IPs (`127.0.0.0/8`, `::1`), `localhost`, and the Tauri webview schemes (`tauri://localhost` on macOS/Linux, `http[s]://tauri.localhost` on Windows). All other origins are rejected at preflight.
- `allow_methods`: `GET`, `POST`, `OPTIONS`.
- `allow_headers`: any header. Loosely set because the allow-origin
  narrowing already prevents an attacker from exercising it — and
  because the OpenAI JS SDK sends a handful of `x-stainless-*` telemetry
  headers we don't want to enumerate by hand.

If you expose this surface beyond loopback (via a reverse proxy,
ngrok, etc.) add your own authentication layer — the CORS allowlist
does nothing for non-browser callers.
