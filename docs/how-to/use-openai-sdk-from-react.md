# Use the OpenAI SDK from React

The whole point of the `/v1/*` surface is that the standard
[`openai`](https://www.npmjs.com/package/openai) npm package works
unchanged — you only rewrite the `baseURL`.

## Minimal example

```ts
import OpenAI from "openai";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

const base = await invoke<string>("get_api_base_url");
const client = new OpenAI({
  baseURL: `${base}/v1`,
  apiKey: "local",                    // server ignores, SDK requires something
  dangerouslyAllowBrowser: true,      // calling a trusted loopback server from the Tauri webview
});

function Example() {
  const [reply, setReply] = useState("");

  async function ask() {
    const stream = await client.chat.completions.create({
      model: "gemma-4-e2b",
      messages: [{ role: "user", content: "hello" }],
      stream: true,
    });
    for await (const chunk of stream) {
      const delta = chunk.choices[0]?.delta?.content ?? "";
      setReply((prev) => prev + delta);  // browser-friendly sink
    }
  }
  // ...
}
```

(The webview has no `process.stdout` — use React state, `console.log`,
or any DOM-aware sink instead.)

## The pre-built hook

protoApp already ships `src/hooks/use-chat.ts` wrapping this pattern
with React state, abort, and clear. Use it directly:

```tsx
import { useChat } from "@/hooks/use-chat";

function MyChat() {
  const { messages, send, isStreaming, stop } = useChat({
    model: "gemma-4-e2b",
    systemPrompt: "Be concise.",
  });
  return (/* render messages, onSubmit → send(input) */);
}
```

## Notes

- The `baseURL` includes the `/v1` prefix. OpenAI's SDK appends paths like `/chat/completions`, so we get `http://127.0.0.1:<port>/v1/chat/completions` as expected.
- `dangerouslyAllowBrowser: true` is required because OpenAI's SDK defends against exposing a real API key in the browser. We don't have a real key — the check is a formality for a trusted loopback target.
- The Tauri webview is cross-origin to `http://127.0.0.1:<port>`, so browser calls rely on the server's loopback-only CORS allowlist. CLI tools like `curl` aren't affected by CORS; browser access from origins outside the allowlist is denied by design.
- Audio endpoints work the same way: `client.audio.transcriptions.create(...)` and `client.audio.speech.create(...)`.
