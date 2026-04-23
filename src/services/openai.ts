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

export function getBaseUrl(): string | null {
  return cachedBaseUrl;
}
