# Use the OpenAI SDK from React

The whole point of the `/v1/*` surface is that the standard
[`openai`](https://www.npmjs.com/package/openai) npm package works
unchanged — you only rewrite the `baseURL`.

## Minimal example

```ts
import OpenAI from "openai";
import { invoke } from "@tauri-apps/api/core";

const base = await invoke<string>("get_api_base_url");
const client = new OpenAI({
  baseURL: `${base}/v1`,
  apiKey: "local",                    // server ignores, SDK requires something
  dangerouslyAllowBrowser: true,      // we're same-origin to localhost
});

const stream = await client.chat.completions.create({
  model: "gemma-4-e2b",
  messages: [{ role: "user", content: "hello" }],
  stream: true,
});

for await (const chunk of stream) {
  process.stdout.write(chunk.choices[0]?.delta?.content ?? "");
}
```

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
- `dangerouslyAllowBrowser: true` is required because OpenAI's SDK defends against exposing a real API key in the browser. We don't have a real key — the check is a formality for our localhost case.
- CORS is already wide-open in the router (`Any`/`Any`/`Any`). Same-origin to `http://127.0.0.1` doesn't even need it, but cross-origin tools (curl in DevTools, LangChain.js on a different page) work too.
- Audio endpoints work the same way: `client.audio.transcriptions.create(...)` and `client.audio.speech.create(...)`.
