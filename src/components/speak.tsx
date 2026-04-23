import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { useSpeech } from "@/hooks/use-speech";

export function Speak() {
  const [text, setText] = useState("Hello from the local server.");
  const { phase, error, audioUrl, speak, reset } = useSpeech();

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <CardTitle>Speak</CardTitle>
        <CardDescription>
          Send text to <code>/v1/audio/speech</code> and play the response.
          Returns 1s of silent WAV today; real Kokoro kicks in with the{" "}
          <code>tts</code> cargo feature.
        </CardDescription>
      </CardHeader>
      <Separator />
      <CardContent className="pt-4 space-y-3">
        <label htmlFor="speak-input" className="sr-only">
          Text to speak
        </label>
        <textarea
          id="speak-input"
          className="w-full min-h-20 rounded-md border bg-background p-3 text-sm font-sans resize-y"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Type something to synthesize…"
          disabled={phase === "synthesizing"}
        />

        <div className="flex items-center gap-2">
          <Button
            type="button"
            disabled={!text.trim() || phase === "synthesizing"}
            onClick={() => speak(text)}
          >
            {phase === "synthesizing" ? "Synthesizing…" : "Speak"}
          </Button>
          <Button
            type="button"
            variant="ghost"
            onClick={reset}
            disabled={phase === "synthesizing"}
          >
            Clear
          </Button>
          <span className="text-muted-foreground text-xs">
            {phase === "playing" && "Playing…"}
            {phase === "done" && "Done"}
          </span>
        </div>

        {error && (
          <p role="alert" aria-live="assertive" className="text-destructive text-sm">
            {error.message}
          </p>
        )}

        {audioUrl && (
          <audio
            className="w-full"
            controls
            src={audioUrl}
            aria-label="Synthesized speech"
          >
            <track kind="captions" />
          </audio>
        )}
      </CardContent>
    </Card>
  );
}
