import OpenAI from "openai";
import { invoke } from "@tauri-apps/api/core";

let clientPromise: Promise<OpenAI> | null = null;
let cachedBaseUrl: string | null = null;

/**
 * Lazy-instantiate an OpenAI client pointed at the in-process Rust server.
 * The base URL is resolved from the Tauri `get_api_base_url` command the first
 * time the client is used, then cached for the session.
 */
export function getOpenAI(): Promise<OpenAI> {
  if (!clientPromise) {
    clientPromise = (async () => {
      const base = await invoke<string>("get_api_base_url");
      cachedBaseUrl = base;
      return new OpenAI({
        baseURL: `${base}/v1`,
        apiKey: "local", // our server ignores the key; OpenAI SDK requires one
        dangerouslyAllowBrowser: true,
      });
    })();
  }
  return clientPromise;
}

/**
 * Synchronous accessor for the cached base URL — useful when you only need
 * it for display or deep-linking and don't want to block on the Tauri
 * command.
 *
 * Returns `null` until {@link getOpenAI} has resolved at least once. If you
 * need the URL before then, `await getOpenAI()` first:
 *
 * ```ts
 * await getOpenAI();
 * const base = getBaseUrl(); // now guaranteed non-null
 * ```
 */
export function getBaseUrl(): string | null {
  return cachedBaseUrl;
}
