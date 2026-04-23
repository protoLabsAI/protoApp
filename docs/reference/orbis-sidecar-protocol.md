# orbis-sidecar protocol

JSON-over-WebSocket, tagged by a `type` field. Defined in
`crates/orbis-sidecar/src/protocol.rs`.

## Transport

- WebSocket (RFC 6455). `ws://` or `wss://`.
- Text frames only. Each frame is one complete JSON message — no framing or length prefixes.
- Connection direction: the Rust host dials; the Python sidecar listens.

## Readiness handshake

Before the WebSocket is available, the sidecar prints **exactly one**
line to stdout:

```
ORBIS_READY ws://127.0.0.1:<port>/<any-path>
```

The Rust spawner parses the URL and opens a WebSocket to it. Anything
before this line is forwarded to tracing at `INFO`. Anything after is
treated as log noise.

## Messages — host → sidecar (`OutgoingMessage`)

```json
{ "type": "user", "text": "What's the weather?" }
```

```json
{ "type": "interrupt" }
```

Cancel the current agent turn. The sidecar should truncate pending
context and stop generating.

```json
{ "type": "context", "key": "user_timezone", "value": "America/Chicago" }
```

Free-form side-channel for settings the agent needs but shouldn't
reply to. `value` is any JSON.

```json
{ "type": "ping", "id": "req-123" }
```

Liveness probe.

## Messages — sidecar → host (`IncomingMessage`)

```json
{ "type": "token", "text": "The " }
```

One streamed token (or any text shard). Feed directly to TTS.

```json
{
  "type": "tool_call",
  "name": "search_weather",
  "args": { "location": "Chicago" },
  "id": "call-456"
}
```

Structured request for the host to execute something (search, memory
lookup, file read, another LLM call). The host replies however makes
sense in your stack — it's not part of this protocol.

```json
{ "type": "turn_end", "finish_reason": "stop" }
```

Agent is done for this turn. Flush TTS, re-open the microphone.
`finish_reason` mirrors OpenAI's: `"stop"`, `"length"`, `"tool_calls"`, etc.

```json
{ "type": "pong", "id": "req-123" }
```

Reply to a `ping`.

```json
{ "type": "error", "message": "timeout calling tool search_weather" }
```

Non-fatal. The host decides whether to surface or retry.

## Extending

Append-only evolution: add new variants, never remove or rename. Note
that the types **are not** `#[non_exhaustive]` today — serde's tagged
enum deserializer will hard-error on an unknown `type` tag. An older
host talking to a newer sidecar will see this as an
`IncomingMessage` parse error bubbling up through `Client::next()`.

If you need fail-soft compatibility across versions, either:
- bump the major version on both sides at the same time, or
- open a PR to add an explicit `Unknown { type: String, raw: Value }`
  variant backed by a custom deserializer, and mark the enums
  `#[non_exhaustive]`.

## Reference implementation (Rust)

See `crates/orbis-sidecar/tests/roundtrip.rs` for an axum-based
reference server that implements enough of this protocol to smoke-test
the client end-to-end.
