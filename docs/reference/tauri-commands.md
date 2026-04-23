# Tauri commands

Rust handlers invokable from the frontend via
[`@tauri-apps/api/core`](https://v2.tauri.app/reference/javascript/api/core/)'s
`invoke`. Signatures are auto-exported to
[`src/bindings.ts`](../../src/bindings.ts) by tauri-specta on debug
builds.

## `get_api_base_url`

**Signature**

```rust
#[tauri::command]
#[specta::specta]
pub fn get_api_base_url(server: State<'_, ApiServer>) -> String
```

**Returns**: `http://127.0.0.1:<ephemeral>` — the base URL of the local
OpenAI-compatible server. Append `/v1` when configuring the OpenAI SDK.

**Frontend**

```ts
import { invoke } from "@tauri-apps/api/core";
const base = await invoke<string>("get_api_base_url");
```

## `greet`

**Signature**

```rust
#[tauri::command]
#[specta::specta]
pub fn greet(name: String) -> GreetResponse
```

**Returns**:

```ts
{ message: string; version: string }
```

Demo command from the starter template; kept for reference.
