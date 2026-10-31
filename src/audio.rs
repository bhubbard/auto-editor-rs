use crate::rational::Timebase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioThreshold {
    /// Linear amplitude threshold in [0.0, 1.0]
    pub linear: f64,
}

impl AudioThreshold {
    pub fn from_linear(linear: f64) -> Self {
        Self {
            linear: linear.clamp(0.0, 1.0),
        }
    }

    /// Converts decibels to linear amplitude: linear = 10^(dB / 20)
    pub fn from_db(db: f64) -> Self {
        let linear = 10.0f64.powf(db / 20.0);
        Self::from_linear(linear)
    }

    /// Converts ITU-R BS.1770-4 LUFS to equivalent linear RMS amplitude:
    /// Since LUFS = -0.691 + 20 * log10(rms), rms = 10^((LUFS + 0.691) / 20)
    pub fn from_lufs(lufs: f64) -> Self {
        let linear = 10.0f64.powf((lufs + 0.691) / 20.0);
        Self::from_linear(linear)
    }

    /// Converts percentage string/number (e.g. 4.0 for 4% -> 0.04)
    pub fn from_percentage(pct: f64) -> Self {
        Self::from_linear(pct / 100.0)
    }

    /// Parse a string value such as "-19dB", "-23LUFS", "4%", "0.04"
    pub fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        let lower = trimmed.to_lowercase();
        if let Some(lufs_str) = lower
            .strip_suffix("lufs")
            .or_else(|| lower.strip_suffix("lkfs"))
        {
            let val = lufs_str
                .trim()
                .parse::<f64>()
                .map_err(|e| format!("Invalid LUFS value '{}': {}", lufs_str, e))?;
            Ok(Self::from_lufs(val))
        } else if let Some(db_str) = trimmed
            .strip_suffix("dB")
            .or_else(|| trimmed.strip_suffix("db"))
        {
            let val = db_str
                .trim()
                .parse::<f64>()
                .map_err(|e| format!("Invalid dB value '{}': {}", db_str, e))?;
            Ok(Self::from_db(val))
        } else if let Some(pct_str) = trimmed.strip_suffix('%') {
            let val = pct_str
                .trim()
                .parse::<f64>()
                .map_err(|e| format!("Invalid percentage value '{}': {}", pct_str, e))?;
            Ok(Self::from_percentage(val))
        } else {
            let val = trimmed
                .parse::<f64>()
                .map_err(|e| format!("Invalid linear audio threshold '{}': {}", trimmed, e))?;
            Ok(Self::from_linear(val))
        }
    }

    /// Returns the equivalent decibel value: dB = 20 * log10(linear)
    pub fn to_db(&self) -> f64 {
        if self.linear <= 1e-9 {
            -180.0
        } else {
            20.0 * self.linear.log10()
        }
    }

    /// Returns the equivalent ITU-R BS.1770-4 LUFS level: -0.691 + 20 * log10(linear)
    pub fn to_lufs(&self) -> f64 {
        if self.linear <= 1e-9 {
            -180.0
        } else {
            -0.691 + 20.0 * self.linear.log10()
        }
    }
}

impl Default for AudioThreshold {
    fn default() -> Self {
        Self::from_linear(0.04) // Auto-Editor default is 0.04
    }
}

/// Second-order IIR Biquad filter (Direct Form I) for ITU-R BS.1770-4 K-weighting
#[derive(Debug, Clone, Copy)]
pub struct BiquadFilter {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl BiquadFilter {
    pub fn new(b0: f64, b1: f64, b2: f64, a1: f64, a2: f64) -> Self {
        Self {
            b0,
            b1,
            b2,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// ITU-R BS.1770-4 Two-Stage K-Weighting Filter (Pre-filter High-Shelf + RLB High-Pass)
#[derive(Debug, Clone)]
pub struct KWeightFilter {
    stage1_shelf: BiquadFilter,
    stage2_rlb: BiquadFilter,
}

impl KWeightFilter {
    /// Exact ITU-R BS.1770-4 Table 1 & Table 2 coefficients at 48 kHz
    pub fn new_48k() -> Self {
        Self {
            stage1_shelf: BiquadFilter::new(
                1.53512485958697,
                -2.69169618940638,
                1.19839281085285,
                -1.69065929318241,
                0.73248077421585,
            ),
            stage2_rlb: BiquadFilter::new(1.0, -2.0, 1.0, -1.99004745483398, 0.99007225036621),
        }
    }

    /// Bilinear-transformed K-weighting coefficients for arbitrary sample rates
    pub fn new_for_rate(sample_rate: u32) -> Self {
        if sample_rate == 48_000 || sample_rate == 0 {
            return Self::new_48k();
        }
        let fs = sample_rate as f64;

        // Stage 1: High-shelf (+4 dB above ~1681.97 Hz)
        let f0_s1 = 1681.974450955533;
        let g_db = 3.999843853973347;
        let q_s1 = 0.7071752369554196;
        let k1 = (std::f64::consts::PI * f0_s1 / fs).tan();
        let vh = 10.0f64.powf(g_db / 20.0);
        let vb = vh.powf(0.4996667741545416);
        let a0_s1 = 1.0 + k1 / q_s1 + k1 * k1;
        let b0_s1 = (vh + vb * k1 / q_s1 + k1 * k1) / a0_s1;
        let b1_s1 = 2.0 * (k1 * k1 - vh) / a0_s1;
        let b2_s1 = (vh - vb * k1 / q_s1 + k1 * k1) / a0_s1;
        let a1_s1 = 2.0 * (k1 * k1 - 1.0) / a0_s1;
        let a2_s1 = (1.0 - k1 / q_s1 + k1 * k1) / a0_s1;

        // Stage 2: RLB high-pass (~38.135 Hz)
        let f0_s2 = 38.13547087602444;
        let q_s2 = 0.5003270373238773;
        let k2 = (std::f64::consts::PI * f0_s2 / fs).tan();
        let a0_s2 = 1.0 + k2 / q_s2 + k2 * k2;
        let b0_s2 = 1.0 / a0_s2;
        let b1_s2 = -2.0 / a0_s2;
        let b2_s2 = 1.0 / a0_s2;
        let a1_s2 = 2.0 * (k2 * k2 - 1.0) / a0_s2;
        let a2_s2 = (1.0 - k2 / q_s2 + k2 * k2) / a0_s2;

        Self {
            stage1_shelf: BiquadFilter::new(b0_s1, b1_s1, b2_s1, a1_s1, a2_s1),
            stage2_rlb: BiquadFilter::new(b0_s2, b1_s2, b2_s2, a1_s2, a2_s2),
        }
    }

    #[inline]
    pub fn process_sample(&mut self, sample: f32) -> f64 {
        let s1 = self.stage1_shelf.process(sample as f64);
        self.stage2_rlb.process(s1)
    }
}

/// Compute ITU-R BS.1770-4 K-weighted loudness (LUFS) for a block of PCM samples
pub fn compute_lufs(samples: &[f32], sample_rate: u32) -> f64 {
    if samples.is_empty() {
        return -180.0;
    }
    let mut filter = KWeightFilter::new_for_rate(sample_rate);
    let mut mean_sq = 0.0f64;
    for &s in samples {
        let y = filter.process_sample(s);
        mean_sq += y * y;
    }
    mean_sq /= samples.len() as f64;
    if mean_sq <= 1e-18 {
        -180.0
    } else {
        -0.691 + 10.0 * mean_sq.log10()
    }
}

/// Compute ITU-R BS.1770-4 gated integrated loudness (LUFS) using 400ms gating blocks
/// with 75% overlap (100ms step), `-70.0 LUFS` absolute gate, and `-10.0 LU` relative gate.
pub fn compute_lufs_integrated(samples: &[f32], sample_rate: u32) -> f64 {
    if samples.is_empty() || sample_rate == 0 {
        return -180.0;
    }
    let mut filter = KWeightFilter::new_for_rate(sample_rate);
    let weighted_sq: Vec<f64> = samples
        .iter()
        .map(|&s| {
            let y = filter.process_sample(s);
            y * y
        })
        .collect();

    let block_len = ((sample_rate as usize) * 4 / 10).max(1); // 400ms
    let hop_len = ((sample_rate as usize) / 10).max(1); // 100ms

    if weighted_sq.len() < block_len {
        let mean_sq = weighted_sq.iter().sum::<f64>() / weighted_sq.len() as f64;
        return if mean_sq <= 1e-18 {
            -180.0
        } else {
            -0.691 + 10.0 * mean_sq.log10()
        };
    }

    let mut block_energies = Vec::new();
    let mut start = 0;
    while start + block_len <= weighted_sq.len() {
        let z: f64 = weighted_sq[start..start + block_len].iter().sum::<f64>() / block_len as f64;
        let l = if z <= 1e-18 {
            -180.0
        } else {
            -0.691 + 10.0 * z.log10()
        };
        if l > -70.0 {
            block_energies.push(z);
        }
        start += hop_len;
    }

    if block_energies.is_empty() {
        return -180.0;
    }

    // Relative gate threshold: -10 LU below ungated mean above -70 LUFS
    let abs_gated_mean = block_energies.iter().sum::<f64>() / block_energies.len() as f64;
    let rel_gate_lufs = (-0.691 + 10.0 * abs_gated_mean.log10()) - 10.0;

    let rel_gated: Vec<f64> = block_energies
        .into_iter()
        .filter(|&z| (-0.691 + 10.0 * z.log10()) > rel_gate_lufs)
        .collect();

    if rel_gated.is_empty() {
        return -180.0;
    }

    let final_mean_sq = rel_gated.iter().sum::<f64>() / rel_gated.len() as f64;
    -0.691 + 10.0 * final_mean_sq.log10()
}

/// Estimate Voice Activity Detection (VAD) speech probability in `[0.0, 1.0]` for an audio frame.
/// Combines K-weighted vocal energy, zero-crossing rate (voiced human speech `80 Hz..3200 Hz`),
/// and crest-factor impulse rejection (distinguishing sustained speech from clicks/mic bumps).
pub fn compute_vad_probability(samples: &[f32], sample_rate: u32) -> f64 {
    if samples.len() < 4 || sample_rate == 0 {
        return 0.0;
    }

    let rms = compute_rms(samples);
    let peak = compute_peak(samples);
    if rms < 1e-4 {
        return 0.0;
    }

    // 1. K-weighted vocal energy score (sigmoid centered around -32 LUFS)
    let lufs = compute_lufs(samples, sample_rate);
    let energy_score = 1.0 / (1.0 + (-(lufs + 32.0) * 0.35).exp());

    // 2. Zero-crossing rate in Hz: each sign change corresponds to half a cycle
    let mut crossings = 0usize;
    for pair in samples.windows(2) {
        if (pair[0] >= 0.0 && pair[1] < 0.0) || (pair[0] < 0.0 && pair[1] >= 0.0) {
            crossings += 1;
        }
    }
    let duration_sec = samples.len() as f64 / sample_rate as f64;
    let est_freq_hz = (crossings as f64 * 0.5) / duration_sec.max(1e-6);

    // Human speech fundamentals and formants predominantly fall between 60 Hz and 3500 Hz
    let freq_score = if (60.0..=3500.0).contains(&est_freq_hz) {
        1.0
    } else if !(30.0..=6000.0).contains(&est_freq_hz) {
        0.15
    } else {
        0.6
    };

    // 3. Crest-factor check: impulsive clicks have very high peak-to-RMS ratio (> 6.0)
    let crest = peak / rms.max(1e-6);
    let crest_penalty = if crest > 5.5 { 0.35 } else { 1.0 };

    (energy_score * freq_score * crest_penalty).clamp(0.0, 1.0)
}

/// Decoded PCM audio from a RIFF `.wav` container
#[derive(Debug, Clone)]
pub struct WavAudio {
    pub sample_rate: u32,
    pub channels: u16,
    /// Mono-mixed normalized floating-point samples in `[-1.0, 1.0]`
    pub mono_samples: Vec<f32>,
}

impl WavAudio {
    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.mono_samples.len() as f64 / self.sample_rate as f64
        }
    }

    /// Compute per-video-frame RMS loudness envelope aligned to `Timebase`
    pub fn frame_rms_envelope(&self, tb: &Timebase) -> Vec<f64> {
        let total_frames = tb.seconds_to_frames(self.duration_seconds()).max(0) as usize;
        if total_frames == 0 || self.mono_samples.is_empty() {
            return Vec::new();
        }
        let samples_per_frame = (self.sample_rate as f64 / tb.as_f64()).round().max(1.0) as usize;
        let mut out = Vec::with_capacity(total_frames);
        for f in 0..total_frames {
            let start = (f * samples_per_frame).min(self.mono_samples.len());
            let end = ((f + 1) * samples_per_frame).min(self.mono_samples.len());
            out.push(compute_rms(&self.mono_samples[start..end]));
        }
        out
    }

    /// Compute per-video-frame VAD speech probabilities aligned to `Timebase`
    pub fn frame_vad_probabilities(&self, tb: &Timebase) -> Vec<f64> {
        let total_frames = tb.seconds_to_frames(self.duration_seconds()).max(0) as usize;
        if total_frames == 0 || self.mono_samples.is_empty() {
            return Vec::new();
        }
        let samples_per_frame = (self.sample_rate as f64 / tb.as_f64()).round().max(1.0) as usize;
        let mut out = Vec::with_capacity(total_frames);
        for f in 0..total_frames {
            let start = (f * samples_per_frame).min(self.mono_samples.len());
            let end = ((f + 1) * samples_per_frame).min(self.mono_samples.len());
            out.push(compute_vad_probability(
                &self.mono_samples[start..end],
                self.sample_rate,
            ));
        }
        out
    }
}

/// Parse a RIFF WAVE file (`.wav`) from bytes (supports 16-bit PCM, 24-bit PCM, 32-bit PCM, and 32-bit IEEE float)
pub fn parse_wav_pcm(bytes: &[u8]) -> Result<WavAudio, String> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("Invalid RIFF WAVE header".to_string());
    }

    let mut pos = 12usize;
    let mut audio_format = 1u16;
    let mut channels = 1u16;
    let mut sample_rate = 48_000u32;
    let mut bits_per_sample = 16u16;
    let mut data_slice: Option<&[u8]> = None;

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let chunk_start = pos + 8;
        let chunk_end = (chunk_start + chunk_size).min(bytes.len());

        if chunk_id == b"fmt " && chunk_end - chunk_start >= 16 {
            let fmt = &bytes[chunk_start..chunk_end];
            audio_format = u16::from_le_bytes([fmt[0], fmt[1]]);
            channels = u16::from_le_bytes([fmt[2], fmt[3]]).max(1);
            sample_rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
            bits_per_sample = u16::from_le_bytes([fmt[14], fmt[15]]);
        } else if chunk_id == b"data" {
            data_slice = Some(&bytes[chunk_start..chunk_end]);
            break;
        }

        // Advance to next word-aligned chunk
        pos = chunk_start + chunk_size + (chunk_size & 1);
    }

    let raw = data_slice.ok_or_else(|| "Missing 'data' chunk in WAV file".to_string())?;
    let bytes_per_sample = (bits_per_sample / 8).max(1) as usize;
    let frame_stride = bytes_per_sample * (channels as usize);
    if frame_stride == 0 {
        return Err("Invalid WAV frame stride".to_string());
    }

    let num_frames = raw.len() / frame_stride;
    let mut mono_samples = Vec::with_capacity(num_frames);

    for f in 0..num_frames {
        let base = f * frame_stride;
        let mut sum = 0.0f32;
        for c in 0..(channels as usize) {
            let off = base + c * bytes_per_sample;
            let s = match (audio_format, bits_per_sample) {
                (1, 16) => {
                    let v = i16::from_le_bytes([raw[off], raw[off + 1]]);
                    v as f32 / 32768.0
                }
                (1, 24) => {
                    let b0 = raw[off] as i32;
                    let b1 = (raw[off + 1] as i32) << 8;
                    let b2 = ((raw[off + 2] as i8) as i32) << 16;
                    (b0 | b1 | b2) as f32 / 8_388_608.0
                }
                (1, 32) => {
                    let v = i32::from_le_bytes(raw[off..off + 4].try_into().unwrap());
                    v as f32 / 2_147_483_648.0
                }
                (3, 32) => f32::from_le_bytes(raw[off..off + 4].try_into().unwrap()),
                _ => {
                    return Err(format!(
                        "Unsupported WAV format={} bits={}",
                        audio_format, bits_per_sample
                    ))
                }
            };
            sum += s;
        }
        mono_samples.push((sum / channels as f32).clamp(-1.0, 1.0));
    }

    Ok(WavAudio {
        sample_rate,
        channels,
        mono_samples,
    })
}

/// Compute Root-Mean-Square (RMS) amplitude from a slice of floating point PCM samples [-1.0, 1.0]
pub fn compute_rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / samples.len() as f64).sqrt()
}

/// Compute peak amplitude from a slice of PCM samples
pub fn compute_peak(samples: &[f32]) -> f64 {
    samples
        .iter()
        .map(|&s| s.abs() as f64)
        .fold(0.0f64, |acc, val| acc.max(val))
}

/// Evaluate audio frame loudness against a threshold. Returns true if active (loud), false if silent.
pub fn is_audio_active(level: f64, threshold: AudioThreshold) -> bool {
    level >= threshold.linear
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_threshold_parsing() {
        let t1 = AudioThreshold::parse("0.04").unwrap();
        assert!((t1.linear - 0.04).abs() < 1e-6);

        let t2 = AudioThreshold::parse("-20dB").unwrap();
        assert!((t2.linear - 0.1).abs() < 1e-4);

        let t3 = AudioThreshold::parse("5%").unwrap();
        assert!((t3.linear - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_rms_and_peak() {
        let samples = vec![0.5f32, -0.5, 0.5, -0.5];
        let rms = compute_rms(&samples);
        assert!((rms - 0.5).abs() < 1e-6);

        let peak = compute_peak(&samples);
        assert!((peak - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_lufs_k_weighting_and_threshold() {
        let t_lufs = AudioThreshold::parse("-23LUFS").unwrap();
        assert!((t_lufs.to_lufs() - (-23.0)).abs() < 1e-4);

        // 1 kHz full-scale sine wave at 48 kHz should measure near -3.01 LUFS
        let sr = 48_000u32;
        let samples: Vec<f32> = (0..48_000)
            .map(|n| (2.0 * std::f64::consts::PI * 997.0 * (n as f64) / (sr as f64)).sin() as f32)
            .collect();
        let lufs = compute_lufs(&samples, sr);
        assert!(
            (lufs - (-3.01)).abs() < 0.25,
            "Expected ~ -3.01 LUFS for 997 Hz full-scale sine, got {}",
            lufs
        );

        let integrated = compute_lufs_integrated(&samples, sr);
        assert!((integrated - lufs).abs() < 0.25);
    }

    #[test]
    fn test_vad_speech_vs_silence_and_impulse() {
        let sr = 16_000u32;
        let silence = vec![0.0f32; 512];
        assert_eq!(compute_vad_probability(&silence, sr), 0.0);

        // Voiced speech fundamental + harmonic (~220 Hz + 660 Hz)
        let speech: Vec<f32> = (0..512)
            .map(|n| {
                let t = n as f64 / sr as f64;
                (0.25 * (2.0 * std::f64::consts::PI * 220.0 * t).sin()
                    + 0.15 * (2.0 * std::f64::consts::PI * 660.0 * t).sin()) as f32
            })
            .collect();
        let speech_prob = compute_vad_probability(&speech, sr);
        assert!(
            speech_prob > 0.65,
            "Expected high VAD prob, got {}",
            speech_prob
        );

        // Single-sample impulse click (high crest factor) should be penalized compared to speech
        let mut click = vec![0.0f32; 512];
        click[10] = 0.95;
        let click_prob = compute_vad_probability(&click, sr);
        assert!(click_prob < speech_prob);
    }

    #[test]
    fn test_parse_wav_pcm_16bit() {
        // Synthesize minimal 16-bit mono WAV at 16000 Hz with 4 samples
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36u32 + 8).to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // 1 channel
        wav.extend_from_slice(&16000u32.to_le_bytes());
        wav.extend_from_slice(&32000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&8u32.to_le_bytes());
        for s in [16384i16, -16384, 16384, -16384] {
            wav.extend_from_slice(&s.to_le_bytes());
        }

        let parsed = parse_wav_pcm(&wav).unwrap();
        assert_eq!(parsed.sample_rate, 16000);
        assert_eq!(parsed.channels, 1);
        assert_eq!(parsed.mono_samples.len(), 4);
        assert!((parsed.mono_samples[0] - 0.5).abs() < 1e-3);
    }
}
