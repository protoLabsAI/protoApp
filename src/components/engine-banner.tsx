import { type Engine, formatStatus, useEngineStatus } from "@/hooks/use-engine-status";
import { cn } from "@/lib/utils";

interface EngineBannerProps {
  engine: Engine;
  /** Human name of the model this engine is loading, for the banner text. */
  label: string;
}

/**
 * Inline banner that surfaces voice-core life-cycle events for a given
 * engine: download progress, loading, errors. Renders nothing when the
 * engine is either idle (no event received yet) or ready.
 */
export function EngineBanner({ engine, label }: EngineBannerProps) {
  const statuses = useEngineStatus();
  const phase = statuses[engine];
  const message = formatStatus(phase, label);
  if (!message) return null;

  const isError = phase?.phase === "error";
  const downloading = phase?.phase === "downloading";
  const percent = downloading && phase.total ? (100 * phase.bytes) / phase.total : null;

  return (
    <section
      role={isError ? "alert" : "status"}
      aria-live={isError ? "assertive" : "polite"}
      className={cn(
        "rounded-md border p-3 text-sm",
        isError
          ? "border-destructive bg-destructive/10 text-destructive"
          : "border-border bg-muted/30 text-foreground",
      )}
    >
      <div className="flex items-center gap-2">
        {!isError && (
          <span
            aria-hidden
            className="h-2 w-2 rounded-full bg-primary animate-pulse"
          />
        )}
        <span>{message}</span>
      </div>
      {percent !== null && (
        <div className="mt-2 h-1.5 w-full overflow-hidden rounded bg-muted">
          <div
            className="h-full bg-primary transition-[width] duration-500 ease-out"
            style={{ width: `${Math.min(100, Math.max(0, percent))}%` }}
          />
        </div>
      )}
    </section>
  );
}
