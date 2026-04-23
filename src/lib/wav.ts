/**
 * Downmix and resample an `AudioBuffer` to 16 kHz mono PCM16, then wrap in
 * a RIFF/WAVE header. Output is what whisper.cpp expects as input — no
 * server-side audio codec stack needed.
 *
 * Uses linear interpolation for the resample; for speech it's fine.
 */
export function encodeMono16kWav(buffer: AudioBuffer): ArrayBuffer {
  const targetRate = 16_000;
  const mono = downmixToMono(buffer);
  const resampled =
    buffer.sampleRate === targetRate ? mono : linearResample(mono, buffer.sampleRate, targetRate);
  return encodePcm16Wav(resampled, targetRate);
}

function downmixToMono(buffer: AudioBuffer): Float32Array {
  const ch = buffer.numberOfChannels;
  const len = buffer.length;
  if (ch === 1) return buffer.getChannelData(0);
  const out = new Float32Array(len);
  const channels: Float32Array[] = [];
  for (let c = 0; c < ch; c++) channels.push(buffer.getChannelData(c));
  for (let i = 0; i < len; i++) {
    let sum = 0;
    for (let c = 0; c < ch; c++) sum += channels[c]?.[i] ?? 0;
    out[i] = sum / ch;
  }
  return out;
}

function linearResample(input: Float32Array, from: number, to: number): Float32Array {
  if (from === to) return input;
  const ratio = from / to;
  const outLen = Math.round(input.length / ratio);
  const out = new Float32Array(outLen);
  for (let i = 0; i < outLen; i++) {
    const src = i * ratio;
    const s0 = Math.floor(src);
    const s1 = Math.min(s0 + 1, input.length - 1);
    const t = src - s0;
    const a = input[s0] ?? 0;
    const b = input[s1] ?? a;
    out[i] = a * (1 - t) + b * t;
  }
  return out;
}

function encodePcm16Wav(samples: Float32Array, sampleRate: number): ArrayBuffer {
  const byteRate = sampleRate * 2;
  const dataSize = samples.length * 2;
  const buf = new ArrayBuffer(44 + dataSize);
  const v = new DataView(buf);
  const writeAscii = (offset: number, s: string) => {
    for (let i = 0; i < s.length; i++) v.setUint8(offset + i, s.charCodeAt(i));
  };
  writeAscii(0, "RIFF");
  v.setUint32(4, 36 + dataSize, true);
  writeAscii(8, "WAVE");
  writeAscii(12, "fmt ");
  v.setUint32(16, 16, true);
  v.setUint16(20, 1, true);
  v.setUint16(22, 1, true);
  v.setUint32(24, sampleRate, true);
  v.setUint32(28, byteRate, true);
  v.setUint16(32, 2, true);
  v.setUint16(34, 16, true);
  writeAscii(36, "data");
  v.setUint32(40, dataSize, true);
  let off = 44;
  for (let i = 0; i < samples.length; i++) {
    const raw = samples[i] ?? 0;
    const s = Math.max(-1, Math.min(1, raw));
    const int16 = s < 0 ? Math.round(s * 0x8000) : Math.round(s * 0x7fff);
    v.setInt16(off, int16, true);
    off += 2;
  }
  return buf;
}
