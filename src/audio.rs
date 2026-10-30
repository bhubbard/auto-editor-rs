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

    /// Converts percentage string/number (e.g. 4.0 for 4% -> 0.04)
    pub fn from_percentage(pct: f64) -> Self {
        Self::from_linear(pct / 100.0)
    }

    /// Parse a string value such as "-19dB", "4%", "0.04"
    pub fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        if let Some(db_str) = trimmed.strip_suffix("dB").or_else(|| trimmed.strip_suffix("db")) {
            let val = db_str.parse::<f64>().map_err(|e| format!("Invalid dB value '{}': {}", db_str, e))?;
            Ok(Self::from_db(val))
        } else if let Some(pct_str) = trimmed.strip_suffix('%') {
            let val = pct_str.parse::<f64>().map_err(|e| format!("Invalid percentage value '{}': {}", pct_str, e))?;
            Ok(Self::from_percentage(val))
        } else {
            let val = trimmed.parse::<f64>().map_err(|e| format!("Invalid linear audio threshold '{}': {}", trimmed, e))?;
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
}

impl Default for AudioThreshold {
    fn default() -> Self {
        Self::from_linear(0.04) // Auto-Editor default is 0.04
    }
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
}
