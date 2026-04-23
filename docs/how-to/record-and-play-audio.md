# Record and play audio

The **Transcribe** and **Speak** tabs in the built-in UI use the OpenAI
SDK's `audio.transcriptions.create()` and `audio.speech.create()`
methods against our local `/v1/*` server. You can do the same thing
from any React component.

## Microphone → transcription

```tsx
import { useTranscription } from "@/hooks/use-transcription";

function MyRecorder() {
  const { phase, text, error, start, stop } = useTranscription();
  return (
    <>
      {phase === "recording"
        ? <button onClick={stop}>Stop</button>
        : <button onClick={start}>Record</button>}
      {phase === "transcribing" && <span>…</span>}
      {error && <p role="alert">{error.message}</p>}
      <p aria-live="polite">{text}</p>
    </>
  );
}
```

The hook:

1. Calls `navigator.mediaDevices.getUserMedia({ audio: true })`.
2. Pipes the stream through `MediaRecorder`, collecting chunks.
3. On stop, wraps the chunks in a `File` and POSTs to
   `/v1/audio/transcriptions` via the OpenAI SDK.
4. Releases the mic tracks.

The browser hands back webm/opus by default. Whisper (and our
transcription stub today) accepts the raw bytes — no client-side
conversion required.

## Text → speech

```tsx
import { useSpeech } from "@/hooks/use-speech";

function MyNarrator() {
  const { phase, speak, audioUrl } = useSpeech();
  return (
    <>
      <button onClick={() => speak("Hello, world.")}>Speak</button>
      {audioUrl && <audio controls src={audioUrl} />}
    </>
  );
}
```

The hook:

1. Calls `openai.audio.speech.create({ model, input, voice, response_format: "wav" })`.
2. Reads the response `Blob` and wraps it in an `ObjectURL`.
3. Plays it via a `new Audio(url)` instance, then exposes the URL for
   an inline `<audio controls>` element so the user can scrub / replay.
4. Revokes the URL on reset to avoid a Blob leak.

## Caveats

- **Stub behavior**: until `--features stt` and `--features tts` ship real engines, transcription returns byte counts and speech returns 1 s of silent WAV. The UI wiring is correct; only the content is filler.
- **Voice strings**: our server accepts any of Kokoro's 50+ voices. The OpenAI SDK's `voice` field is a narrow enum, so the `useSpeech` hook casts through `unknown` to pass custom voice ids. Swap to a tighter type once you've pinned your voice set.
- **`response_format` == "mp3"**: the server still returns WAV and advertises this in an `x-protoapp-note` header. Don't rely on the MIME extension matching the bytes until the real TTS engine lands.
- **CORS**: browser calls only succeed from loopback origins and the Tauri webview schemes — see [reference/openai-api.md](../reference/openai-api.md#cors). A browser tab at `example.com` cannot reach your local server.
