use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MotionThreshold {
    /// Normalized motion intensity threshold in [0.0, 1.0]
    pub threshold: f64,
}

impl MotionThreshold {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        if let Some(pct) = trimmed.strip_suffix('%') {
            let val = pct
                .parse::<f64>()
                .map_err(|e| format!("Invalid motion %: {}", e))?;
            Ok(Self::new(val / 100.0))
        } else if let Some(db) = trimmed
            .strip_suffix("dB")
            .or_else(|| trimmed.strip_suffix("db"))
        {
            let val = db
                .parse::<f64>()
                .map_err(|e| format!("Invalid motion dB: {}", e))?;
            let linear = 10.0f64.powf(val / 20.0);
            Ok(Self::new(linear))
        } else {
            let val = trimmed
                .parse::<f64>()
                .map_err(|e| format!("Invalid motion threshold: {}", e))?;
            Ok(Self::new(val))
        }
    }
}

impl Default for MotionThreshold {
    fn default() -> Self {
        Self::new(0.02)
    }
}

/// Computes normalized pixel difference between two grayscale or flat RGB frame buffers of equal size.
pub fn compute_motion_difference(frame_a: &[u8], frame_b: &[u8]) -> f64 {
    if frame_a.is_empty() || frame_a.len() != frame_b.len() {
        return 0.0;
    }

    let diff_sum: u64 = frame_a
        .iter()
        .zip(frame_b.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).unsigned_abs() as u64)
        .sum();

    let max_diff = frame_a.len() as f64 * 255.0;
    diff_sum as f64 / max_diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_threshold_parsing() {
        let m1 = MotionThreshold::parse("0.02").unwrap();
        assert!((m1.threshold - 0.02).abs() < 1e-6);

        let m2 = MotionThreshold::parse("2%").unwrap();
        assert!((m2.threshold - 0.02).abs() < 1e-6);
    }

    #[test]
    fn test_motion_difference() {
        let f1 = vec![0u8, 0, 0, 0];
        let f2 = vec![255u8, 255, 255, 255];
        let diff = compute_motion_difference(&f1, &f2);
        assert!((diff - 1.0).abs() < 1e-6);

        let f3 = vec![0u8, 0, 0, 0];
        let diff_zero = compute_motion_difference(&f1, &f3);
        assert_eq!(diff_zero, 0.0);
    }
}
