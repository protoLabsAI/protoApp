import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { useTranscription } from "@/hooks/use-transcription";

export function Transcribe() {
  const { phase, text, error, start, stop, reset } = useTranscription();

  const primary = (() => {
    switch (phase) {
      case "idle":
      case "done":
      case "error":
        return (
          <Button type="button" onClick={start}>
            Record
          </Button>
        );
      case "recording":
        return (
          <Button type="button" variant="destructive" onClick={stop}>
            Stop
          </Button>
        );
      case "transcribing":
        return (
          <Button type="button" disabled>
            Transcribing…
          </Button>
        );
    }
  })();

  return (
    <Card className="w-full max-w-2xl">
      <CardHeader>
        <CardTitle>Transcribe</CardTitle>
        <CardDescription>
          Record from your mic and send to <code>/v1/audio/transcriptions</code>.
          Returns stubbed text today; real Whisper kicks in with the{" "}
          <code>stt</code> cargo feature.
        </CardDescription>
      </CardHeader>
      <Separator />
      <CardContent className="pt-4 space-y-3">
        <div className="flex items-center gap-2">
          {primary}
          <Button type="button" variant="ghost" onClick={reset} disabled={phase === "recording" || phase === "transcribing"}>
            Clear
          </Button>
          <span className="text-muted-foreground text-xs">
            {phase === "recording" && "Listening…"}
            {phase === "transcribing" && "Sending to server…"}
            {phase === "done" && "Done"}
          </span>
        </div>

        {error && (
          <p role="alert" aria-live="assertive" className="text-destructive text-sm">
            {error.message}
          </p>
        )}

        <section
          aria-live="polite"
          aria-label="Transcription result"
          className="min-h-16 rounded-md border bg-muted/30 p-3 text-sm whitespace-pre-wrap"
        >
          {text || (
            <span className="text-muted-foreground">
              Transcript will appear here after the first recording.
            </span>
          )}
        </section>
      </CardContent>
    </Card>
  );
}
