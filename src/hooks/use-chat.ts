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

  const send = useCallback(
    async (input: string) => {
      if (!input.trim() || isStreaming) return;

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
        abortRef.current = null;
      }
    },
    [messages, isStreaming, model, systemPrompt],
  );

  const stop = useCallback(() => {
    abortRef.current?.abort();
  }, []);

  const clear = useCallback(() => {
    setMessages([]);
    setError(null);
  }, []);

  return { messages, send, stop, clear, isStreaming, error };
}
