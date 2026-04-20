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
