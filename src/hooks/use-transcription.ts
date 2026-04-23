import { useCallback, useRef, useState } from "react";
import { getOpenAI } from "@/services/openai";

type Phase = "idle" | "recording" | "transcribing" | "done" | "error";

/**
 * Browser mic → MediaRecorder → POST /v1/audio/transcriptions → text.
 *
 * The server round-trips any bytes we send; in stub mode it echoes the byte
 * count so the UI can be exercised end-to-end. Real Whisper transcription
 * kicks in once the `stt` cargo feature lands.
 */
export function useTranscription(model = "whisper-large-v3-turbo") {
  const [phase, setPhase] = useState<Phase>("idle");
  const [text, setText] = useState("");
  const [error, setError] = useState<Error | null>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const streamRef = useRef<MediaStream | null>(null);

  const start = useCallback(async () => {
    if (phase === "recording" || phase === "transcribing") return;
    setError(null);
    setText("");

    let stream: MediaStream;
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    } catch (e) {
      setError(e instanceof Error ? e : new Error(String(e)));
      setPhase("error");
      return;
    }
    streamRef.current = stream;

    const rec = new MediaRecorder(stream);
    mediaRecorderRef.current = rec;
    chunksRef.current = [];
    rec.ondataavailable = (e) => {
      if (e.data.size > 0) chunksRef.current.push(e.data);
    };
    rec.onstop = async () => {
      for (const t of streamRef.current?.getTracks() ?? []) {
        t.stop();
      }
      streamRef.current = null;

      const blob = new Blob(chunksRef.current, {
        type: rec.mimeType || "audio/webm",
      });
      setPhase("transcribing");

      try {
        const openai = await getOpenAI();
        const result = await openai.audio.transcriptions.create({
          file: new File([blob], "clip.webm", { type: blob.type }),
          model,
        });
        setText(result.text ?? "");
        setPhase("done");
      } catch (e) {
        setError(e instanceof Error ? e : new Error(String(e)));
        setPhase("error");
      }
    };
    rec.start();
    setPhase("recording");
  }, [phase, model]);

  const stop = useCallback(() => {
    mediaRecorderRef.current?.stop();
  }, []);

  const reset = useCallback(() => {
    setText("");
    setError(null);
    setPhase("idle");
  }, []);

  return { phase, text, error, start, stop, reset };
}
