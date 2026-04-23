//! Kokoro TTS via the `kokoros` git crate.
//!
//! First call downloads the ONNX model (`kokoro-v1.0.onnx`, ~310 MB) and
//! the combined voice pack (`voices-v1.0.bin`, ~27 MB) into
//! `~/.cache/protoapp/kokoro/`. Override with `PROTOAPP_KOKORO_MODEL_PATH`
//! and `PROTOAPP_KOKORO_VOICES_PATH`.
//!
//! We return 24 kHz mono PCM16 WAV, matching what the stub emits so the
//! client contract doesn't change.

use std::io::Cursor;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use kokoros::tts::koko::TTSKoko;
use tokio::sync::OnceCell;

pub const OUTPUT_SAMPLE_RATE: u32 = 24_000;

static ENGINE: OnceCell<TTSKoko> = OnceCell::const_new();

fn cache_dir() -> Result<PathBuf> {
    let root = dirs::cache_dir()
        .ok_or_else(|| anyhow!("no cache dir available on this platform"))?;
    let dir = root.join("protoapp").join("kokoro");
    std::fs::create_dir_all(&dir).context("create kokoro cache dir")?;
    Ok(dir)
}

fn model_path() -> Result<String> {
    if let Ok(p) = std::env::var("PROTOAPP_KOKORO_MODEL_PATH") {
        return Ok(p);
    }
    Ok(cache_dir()?.join("kokoro-v1.0.onnx").to_string_lossy().into_owned())
}

fn voices_path() -> Result<String> {
    if let Ok(p) = std::env::var("PROTOAPP_KOKORO_VOICES_PATH") {
        return Ok(p);
    }
    Ok(cache_dir()?.join("voices-v1.0.bin").to_string_lossy().into_owned())
}

async fn engine() -> Result<&'static TTSKoko> {
    ENGINE
        .get_or_try_init(|| async {
            tracing::info!("loading Kokoro TTS (first run downloads ~340 MB)");
            let model = model_path()?;
            let voices = voices_path()?;
            // TTSKoko::new downloads the files on first use via the kokoros
            // InitConfig defaults. That's the cache-warm cost we pay once.
            let engine = TTSKoko::new(&model, &voices).await;
            tracing::info!("Kokoro TTS ready");
            anyhow::Ok(engine)
        })
        .await
}

/// Synthesize `input` in `voice`. Returns 24 kHz mono WAV bytes.
pub async fn synthesize_wav(input: &str, voice: &str) -> Result<Vec<u8>> {
    let engine = engine().await?;
    let voice = voice.to_string();
    let input = input.to_string();

    // Kokoros is CPU-bound and not async internally — run it off the tokio
    // pool so other requests (chat streaming, etc.) aren't starved.
    let samples: Vec<f32> = tokio::task::spawn_blocking(move || -> Result<Vec<f32>> {
        engine
            .tts_raw_audio(&input, "en-us", &voice, 1.0, None, None, None, None)
            .map_err(|e| anyhow!("kokoros tts_raw_audio failed: {e}"))
    })
    .await
    .context("join kokoros worker")??;

    if samples.is_empty() {
        bail!("kokoros returned no samples (empty synthesis)");
    }

    Ok(f32_samples_to_wav(&samples, OUTPUT_SAMPLE_RATE))
}

/// Encode a mono f32 buffer as a 16-bit PCM WAV. We pick PCM16 (not f32 WAV)
/// because every browser `<audio>` element can decode it with no extra
/// client-side work.
fn f32_samples_to_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut buf: Vec<u8> = Vec::with_capacity(44 + samples.len() * 2);
    {
        let cursor = Cursor::new(&mut buf);
        let mut writer = hound::WavWriter::new(cursor, spec).expect("hound spec is valid");
        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let i = (clamped * i16::MAX as f32) as i16;
            // Writing into an in-memory Cursor<Vec<u8>> only fails on
            // OOM / integer overflow; use expect() with a helpful message
            // so we don't silently drop samples on a real failure.
            writer
                .write_sample(i)
                .expect("hound write_sample to in-memory Vec<u8>");
        }
        writer.finalize().expect("hound finalize");
    }
    buf
}
