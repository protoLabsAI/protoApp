import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { useChat } from "@/hooks/use-chat";
import { cn } from "@/lib/utils";

export function Chat() {
  const [input, setInput] = useState("");
  const { messages, send, stop, clear, isStreaming, error } = useChat();
  const scrollRef = useRef<HTMLDivElement | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: scroll to bottom when a new message or token arrives
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messages]);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const text = input.trim();
    if (!text) return;
    setInput("");
    await send(text);
  };

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <CardTitle>Local chat</CardTitle>
        <CardDescription>
          Talking to the in-process OpenAI-compatible server (model: gemma-4-e2b)
        </CardDescription>
      </CardHeader>
      <Separator />
      <CardContent className="pt-4 space-y-3">
        <div
          ref={scrollRef}
          role="log"
          aria-live="polite"
          aria-atomic="false"
          aria-label="Chat transcript"
          className="h-80 overflow-y-auto rounded-md border bg-muted/30 p-3 space-y-2 text-sm"
        >
          {messages.length === 0 && (
            <p className="text-muted-foreground">
              Say hi to start. Default reply is a stub until the <code>llm</code> cargo
              feature is built (e.g. <code>--features "llm metal"</code>).
            </p>
          )}
          {messages.map((m, i) => (
            <div
              // biome-ignore lint/suspicious/noArrayIndexKey: message order is stable within a session
              key={i}
              className={cn(
                "rounded-md px-3 py-2 whitespace-pre-wrap",
                m.role === "user"
                  ? "bg-primary/10 ml-8"
                  : m.role === "assistant"
                    ? "bg-background mr-8 border"
                    : "bg-muted text-muted-foreground",
              )}
            >
              <div className="text-[10px] uppercase tracking-wide text-muted-foreground mb-1">
                {m.role}
              </div>
              {m.content || (isStreaming && i === messages.length - 1 ? "…" : "")}
            </div>
          ))}
        </div>

        {error && <p className="text-destructive text-sm">{error.message}</p>}

        <form onSubmit={onSubmit} className="flex gap-2">
          <label htmlFor="chat-input" className="sr-only">
            Message
          </label>
          <Input
            id="chat-input"
            aria-label="Message"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={isStreaming ? "Streaming…" : "Type a message"}
            disabled={isStreaming}
          />
          {isStreaming ? (
            <Button type="button" variant="outline" onClick={stop}>
              Stop
            </Button>
          ) : (
            <Button type="submit" disabled={!input.trim()}>
              Send
            </Button>
          )}
          <Button type="button" variant="ghost" onClick={clear} disabled={isStreaming}>
            Clear
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
