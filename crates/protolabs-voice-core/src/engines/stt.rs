//! Whisper-backed speech-to-text.
//!
//! The endpoint handler in `api/transcriptions.rs` receives a WAV blob
//! produced by the browser (`useTranscription` downmixes and resamples to
//! 16 kHz mono PCM16 before upload, so no server-side audio codec stack is
//! needed). We parse the WAV, run whisper.cpp via `whisper-rs`, and
//! concatenate the segment text.
//!
//! Model is cached under `~/.cache/protoapp/whisper/<filename>` and only
//! downloaded on first use. Override with `PROTOAPP_WHISPER_MODEL_PATH`.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use tokio::sync::OnceCell;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const DEFAULT_MODEL_REPO: &str = "ggerganov/whisper.cpp";
const DEFAULT_MODEL_FILE: &str = "ggml-base.en-q5_1.bin";
const DEFAULT_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en-q5_1.bin";

/// Hard cap on model download.
/// Tunable via `PROTOAPP_WHISPER_DOWNLOAD_TIMEOUT_SECS`.
fn download_timeout() -> Duration {
    std::env::var("PROTOAPP_WHISPER_DOWNLOAD_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| Duration::from_secs(10 * 60))
}

/// Lazily-initialized whisper context. Loading the model compiles its
/// tensors for the current backend, which takes seconds — we do it once
/// per process and reuse for every request.
static CTX: OnceCell<WhisperContext> = OnceCell::const_new();

/// Locate the cached model file, downloading it from Hugging Face on
/// first use.
pub async fn ensure_model() -> Result<PathBuf> {
    if let Ok(override_path) = std::env::var("PROTOAPP_WHISPER_MODEL_PATH") {
        let p = PathBuf::from(override_path);
        if p.exists() {
            return Ok(p);
        }
        bail!("PROTOAPP_WHISPER_MODEL_PATH={} does not exist", p.display());
    }

    let cache_root = dirs::cache_dir()
        .ok_or_else(|| anyhow!("no cache dir available on this platform"))?;
    let dir = cache_root.join("protoapp").join("whisper");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(DEFAULT_MODEL_FILE);

    if path.exists() {
        return Ok(path);
    }

    tracing::info!(
        url = DEFAULT_MODEL_URL,
        to = %path.display(),
        "downloading whisper model (first run only)"
    );

    // Stream download into a temp file, then rename on completion so a
    // partial download can't masquerade as a complete one.
    let tmp = path.with_extension("bin.partial");
    download_streaming(DEFAULT_MODEL_URL, &tmp).await?;
    tokio::fs::rename(&tmp, &path)
        .await
        .context("rename whisper model into place")?;
    Ok(path)
}

async fn download_streaming(url: &str, dst: &Path) -> Result<()> {
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    let timeout = download_timeout();
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .context("build reqwest client")?;

    let resp = client.get(url).send().await.context("GET model")?;
    if !resp.status().is_success() {
        bail!("model download failed: HTTP {}", resp.status());
    }
    let total = resp.content_length();
    let mut file = tokio::fs::File::create(dst).await?;
    let mut stream = resp.bytes_stream();
    let mut bytes_written: u64 = 0;
    let mut last_log = std::time::Instant::now();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("read chunk")?;
        file.write_all(&chunk).await?;
        bytes_written += chunk.len() as u64;
        if last_log.elapsed() > Duration::from_secs(5) {
            if let Some(total) = total {
                tracing::info!(
                    percent = (bytes_written * 100 / total.max(1)),
                    "whisper model download progress"
                );
            } else {
                tracing::info!(bytes = bytes_written, "whisper model download progress");
            }
            last_log = std::time::Instant::now();
        }
    }
    file.flush().await?;
    Ok(())
}

/// Lazily build and return the shared whisper context.
async fn context() -> Result<&'static WhisperContext> {
    CTX.get_or_try_init(|| async {
        let model_path = ensure_model().await?;
        tracing::info!(
            repo = DEFAULT_MODEL_REPO,
            file = DEFAULT_MODEL_FILE,
            "loading whisper model"
        );
        let ctx = WhisperContext::new_with_params(
            model_path
                .to_str()
                .ok_or_else(|| anyhow!("non-utf8 whisper model path"))?,
            WhisperContextParameters::default(),
        )
        .context("WhisperContext::new_with_params")?;
        anyhow::Ok(ctx)
    })
    .await
}

/// Transcribe the WAV-encoded audio bytes that came in as the `file` field
/// of a `POST /v1/audio/transcriptions` request. The frontend's
/// `useTranscription` hook guarantees 16 kHz mono PCM16 WAV; older clients
/// that send a different WAV format are converted on the fly.
pub async fn transcribe(wav_bytes: &[u8]) -> Result<String> {
    let (samples, source_rate, source_channels) = read_wav_to_f32_mono(wav_bytes)
        .context("decode incoming WAV")?;

    let samples = if source_rate == 16_000 {
        samples
    } else {
        tracing::warn!(
            source_rate,
            "client sent non-16kHz audio; naive-resampling on the server"
        );
        naive_resample(&samples, source_rate, 16_000)
    };
    tracing::debug!(
        samples = samples.len(),
        channels = source_channels,
        "running whisper"
    );

    let ctx = context().await?;
    // Whisper calls into CPU/GPU-bound C++ that holds its own locks; keep it
    // off the tokio pool so async tasks don't starve.
    let text = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let mut state = ctx
            .create_state()
            .context("WhisperContext::create_state")?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        state.full(params, &samples).context("state.full")?;

        let mut out = String::new();
        for segment in state.as_iter() {
            if let Ok(text) = segment.to_str_lossy() {
                out.push_str(&text);
            }
        }
        Ok(out.trim().to_string())
    })
    .await
    .context("join whisper worker")??;

    Ok(text)
}

fn read_wav_to_f32_mono(bytes: &[u8]) -> Result<(Vec<f32>, u32, u16)> {
    let cursor = Cursor::new(bytes);
    let mut reader = hound::WavReader::new(cursor).context("open WAV reader")?;
    let spec = reader.spec();
    let channels = spec.channels.max(1);
    let sample_rate = spec.sample_rate;

    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<_, _>>()
            .context("read f32 PCM")?,
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<std::result::Result<_, _>>()
                .context("read int PCM")?
        }
    };

    if channels == 1 {
        return Ok((interleaved, sample_rate, 1));
    }
    // Downmix to mono.
    let mut mono = Vec::with_capacity(interleaved.len() / channels as usize);
    let ch = channels as usize;
    for frame in interleaved.chunks_exact(ch) {
        let sum: f32 = frame.iter().sum();
        mono.push(sum / ch as f32);
    }
    Ok((mono, sample_rate, channels))
}

/// Linear-interpolation resampler. Quality is fine for speech, and we don't
/// want to pay for the `rubato` dep on the default path — anything sent by
/// our own frontend already arrives at 16 kHz so this is just a fallback
/// for ad-hoc curl uploads at a different rate.
fn naive_resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if input.is_empty() || from == to {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let out_len = ((input.len() as f64) / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = (i as f64) * ratio;
        let s0 = src.floor() as usize;
        let s1 = (s0 + 1).min(input.len() - 1);
        let t = (src - s0 as f64) as f32;
        out.push(input[s0] * (1.0 - t) + input[s1] * t);
    }
    out
}
