import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { agentAsk } from "@/services/commands";

type Turn = { role: "user" | "agent"; content: string };

/**
 * Minimal panel for the in-process agent (zeroclaw, embedded in the Rust
 * process — the iPad-safe replacement for the ORBIS sidecar). Each turn calls
 * the `agent_ask` Tauri command, which runs the agent against voice-core's
 * local server. A stable session id threads the agent's memory across turns.
 */
export function Agent() {
  const [input, setInput] = useState("");
  const [turns, setTurns] = useState<Turn[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sessionId = useRef(crypto.randomUUID());

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const text = input.trim();
    if (!text || busy) return;
    setError(null);
    setBusy(true);
    setTurns((t) => [...t, { role: "user", content: text }]);
    setInput("");
    try {
      const baseUrl = await invoke<string>("get_api_base_url");
      const reply = await agentAsk(baseUrl, text, { sessionId: sessionId.current });
      setTurns((t) => [...t, { role: "agent", content: reply }]);
    } catch (err) {
      setError(typeof err === "string" ? err : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <CardTitle>Agent</CardTitle>
        <CardDescription>
          In-process zeroclaw agent, wired to the local server. Replaces the ORBIS sidecar.
        </CardDescription>
      </CardHeader>
      <Separator />
      <CardContent className="pt-4 space-y-3">
        <div
          role="log"
          aria-live="polite"
          aria-label="Agent transcript"
          className="h-80 overflow-y-auto rounded-md border bg-muted/30 p-3 space-y-2 text-sm"
        >
          {turns.length === 0 && (
            <p className="text-muted-foreground">
              Ask the agent something. It runs in-process and reasons over tools and memory,
              using the same local model as the Chat tab. Build with the <code>llm</code> feature
              for real replies.
            </p>
          )}
          {turns.map((t, i) => (
            <div
              // biome-ignore lint/suspicious/noArrayIndexKey: turn order is stable within a session
              key={i}
              className={cnRole(t.role)}
            >
              <div className="text-[10px] uppercase tracking-wide text-muted-foreground mb-1">
                {t.role}
              </div>
              {t.content}
            </div>
          ))}
          {busy && <p className="text-muted-foreground">Thinking…</p>}
        </div>

        {error && (
          <p role="alert" aria-live="assertive" className="text-destructive text-sm">
            {error}
          </p>
        )}

        <form onSubmit={onSubmit} className="flex gap-2">
          <label htmlFor="agent-input" className="sr-only">
            Message
          </label>
          <Input
            id="agent-input"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={busy ? "Working…" : "Ask the agent"}
            disabled={busy}
          />
          <Button type="submit" disabled={busy || !input.trim()}>
            Ask
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}

function cnRole(role: Turn["role"]): string {
  const base = "rounded-md px-3 py-2 whitespace-pre-wrap";
  return role === "user" ? `${base} bg-primary/10 ml-8` : `${base} bg-background mr-8 border`;
}
