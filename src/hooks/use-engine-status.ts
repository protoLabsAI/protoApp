import { useEffect, useState } from "react";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";

export type Engine = "llm" | "stt" | "tts";

/**
 * Exactly mirrors `protolabs-voice-core::engines::events::EngineStatus`.
 * Rust serializes Phase with `#[serde(tag = "phase")]` and flattens it onto
 * the outer struct, so every event is a flat object with `engine` + `phase`
 * + phase-specific fields.
 */
export type EnginePhase =
  | { phase: "loading" }
  | { phase: "downloading"; bytes: number; total?: number }
  | { phase: "ready" }
  | { phase: "error"; message: string };

export type EngineStatus = EnginePhase & { engine: Engine };

/**
 * Subscribe to Tauri `engine-status` events emitted by voice-core. Returns
 * the latest phase for each engine (or `undefined` before the first event).
 */
export function useEngineStatus() {
  const [statuses, setStatuses] = useState<Partial<Record<Engine, EnginePhase>>>({});

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let cancelled = false;

    (async () => {
      const fn = await listen<EngineStatus>("engine-status", (event) => {
        // Strip `engine` off the flat payload; the rest is the discriminated
        // `EnginePhase` union (`phase` + variant-specific fields).
        const { engine, ...rest } = event.payload;
        const phase = rest as EnginePhase;
        setStatuses((prev) => ({ ...prev, [engine]: phase }));
      });
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return statuses;
}

/** Short human-friendly label for a phase, or `null` when ready / not started. */
export function formatStatus(phase: EnginePhase | undefined, label: string): string | null {
  if (!phase || phase.phase === "ready") return null;
  switch (phase.phase) {
    case "loading":
      return `Loading ${label}…`;
    case "downloading": {
      if (phase.total && phase.total > 0) {
        const pct = Math.round((100 * phase.bytes) / phase.total);
        const mb = (phase.bytes / 1_000_000).toFixed(0);
        const totalMb = (phase.total / 1_000_000).toFixed(0);
        return `Downloading ${label}… ${mb} MB / ${totalMb} MB (${pct}%)`;
      }
      const mb = (phase.bytes / 1_000_000).toFixed(0);
      return `Downloading ${label}… ${mb} MB`;
    }
    case "error":
      return `Error loading ${label}: ${phase.message}`;
  }
}
