import { useCallback, useState } from "react";
import { getOpenAI } from "@/services/openai";

type Phase = "idle" | "synthesizing" | "playing" | "done" | "error";

export interface UseSpeechOptions {
  model?: string;
  voice?: string;
}

/**
 * Text → POST /v1/audio/speech → Blob → `<audio>` playback.
 *
 * Server always returns WAV today (mp3 transcoding is a future extension —
 * the advisory `x-protoapp-note` header flags this for clients).
 */
export function useSpeech({
  model = "kokoro-82m",
  voice = "af_heart",
}: UseSpeechOptions = {}) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState<Error | null>(null);
  const [audioUrl, setAudioUrl] = useState<string | null>(null);

  const speak = useCallback(
    async (text: string) => {
      if (!text.trim() || phase === "synthesizing") return;
      setError(null);

      if (audioUrl) {
        URL.revokeObjectURL(audioUrl);
        setAudioUrl(null);
      }

      setPhase("synthesizing");
      try {
        const openai = await getOpenAI();
        // OpenAI SDK expects a "voice" but ours allows any of Kokoro's 50+
        // voices; cast through any to pass arbitrary strings without
        // fighting the narrow SDK enum.
        const resp = await openai.audio.speech.create({
          model,
          input: text,
          voice: voice as unknown as "alloy",
          response_format: "wav",
        });
        const blob = await resp.blob();
        const url = URL.createObjectURL(blob);
        setAudioUrl(url);
        setPhase("playing");
        const audio = new Audio(url);
        audio.onended = () => setPhase("done");
        audio.onerror = () => setPhase("error");
        await audio.play();
      } catch (e) {
        setError(e instanceof Error ? e : new Error(String(e)));
        setPhase("error");
      }
    },
    [model, voice, phase, audioUrl],
  );

  const reset = useCallback(() => {
    if (audioUrl) {
      URL.revokeObjectURL(audioUrl);
      setAudioUrl(null);
    }
    setError(null);
    setPhase("idle");
  }, [audioUrl]);

  return { phase, error, audioUrl, speak, reset };
}
