import { useCallback, useRef, useState } from "react";
import { getOpenAI } from "@/services/openai";

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface UseChatOptions {
  model?: string;
  systemPrompt?: string;
}

export function useChat({
  model = "gemma-4-e2b",
  systemPrompt = "You are a helpful assistant running entirely locally.",
}: UseChatOptions = {}) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  // Synchronous guard — React state updates are async, so two rapid send()
  // calls could both pass `!isStreaming` before either render lands. This
  // ref is set before any await and checked before we commit to a run.
  const isStreamingRef = useRef(false);

  const stop = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    isStreamingRef.current = false;
  }, []);

  const send = useCallback(
    async (input: string) => {
      if (!input.trim() || isStreamingRef.current) return;
      isStreamingRef.current = true;

      setError(null);
      setIsStreaming(true);

      const userMsg: ChatMessage = { role: "user", content: input };
      const history = [...messages, userMsg];
      setMessages([...history, { role: "assistant", content: "" }]);

      const abort = new AbortController();
      abortRef.current = abort;

      try {
        const openai = await getOpenAI();
        const stream = await openai.chat.completions.create(
          {
            model,
            messages: [{ role: "system", content: systemPrompt }, ...history],
            stream: true,
          },
          { signal: abort.signal },
        );

        for await (const chunk of stream) {
          const delta = chunk.choices?.[0]?.delta?.content;
          if (!delta) continue;
          setMessages((prev) => {
            const copy = [...prev];
            const last = copy[copy.length - 1];
            if (last?.role === "assistant") {
              copy[copy.length - 1] = { ...last, content: last.content + delta };
            }
            return copy;
          });
        }
      } catch (e) {
        if ((e as { name?: string })?.name !== "AbortError") {
          setError(e instanceof Error ? e : new Error(String(e)));
        }
      } finally {
        setIsStreaming(false);
        isStreamingRef.current = false;
        abortRef.current = null;
      }
    },
    [messages, model, systemPrompt],
  );

  const clear = useCallback(() => {
    // Abort any in-flight request before wiping state so tokens from a
    // stale stream can't mutate the freshly-cleared message list.
    stop();
    setMessages([]);
    setError(null);
  }, [stop]);

  return { messages, send, stop, clear, isStreaming, error };
}
