// Re-export from auto-generated bindings (produced by tauri-specta on debug build).
// Until bindings.ts exists, this manual fallback keeps TS happy.
// After running `pnpm tauri dev` once, bindings.ts will be generated and this
// file can simply re-export from there.

import { invoke } from "@tauri-apps/api/core";

export interface GreetResponse {
  message: string;
  version: string;
}

export const greet = (name: string): Promise<GreetResponse> => invoke("greet", { name });

/**
 * Drive one in-process agent turn (zeroclaw, embedded in the Rust process).
 * `baseUrl` is voice-core's server root from {@link getBaseUrl}; the agent's
 * LLM provider is wired to `{baseUrl}/v1`, so it runs on the same local model
 * as the Chat tab. Rejects with the Rust error string on failure.
 */
export const agentAsk = (
  baseUrl: string,
  message: string,
  opts?: { model?: string; sessionId?: string },
): Promise<string> =>
  invoke("agent_ask", {
    baseUrl,
    model: opts?.model ?? null,
    message,
    sessionId: opts?.sessionId ?? null,
  });
