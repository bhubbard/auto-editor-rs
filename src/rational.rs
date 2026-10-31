use num_rational::Rational64;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Timebase(pub Rational64);

impl Timebase {
    pub fn new(numer: i64, denom: i64) -> Self {
        Self(Rational64::new(numer, denom))
    }

    pub fn fps_30() -> Self {
        Self::new(30, 1)
    }

    pub fn fps_60() -> Self {
        Self::new(60, 1)
    }

    pub fn fps_24() -> Self {
        Self::new(24, 1)
    }

    pub fn fps_23_976() -> Self {
        Self::new(24000, 1001)
    }

    pub fn fps_29_97() -> Self {
        Self::new(30000, 1001)
    }

    pub fn fps_59_94() -> Self {
        Self::new(60000, 1001)
    }

    pub fn as_f64(&self) -> f64 {
        *self.0.numer() as f64 / *self.0.denom() as f64
    }

    pub fn frames_to_seconds(&self, frames: i64) -> f64 {
        frames as f64 / self.as_f64()
    }

    pub fn seconds_to_frames(&self, seconds: f64) -> i64 {
        (seconds * self.as_f64()).round() as i64
    }

    pub fn timecode(&self, frames: i64) -> String {
        let fps = self.as_f64();
        let total_seconds = frames as f64 / fps;
        let hours = (total_seconds / 3600.0).floor() as i64;
        let rem_seconds = total_seconds - (hours as f64 * 3600.0);
        let minutes = (rem_seconds / 60.0).floor() as i64;
        let seconds = (rem_seconds - (minutes as f64 * 60.0)).floor() as i64;
        let sub_frames = frames % (fps.round() as i64);

        format!(
            "{:02}:{:02}:{:02}:{:02}",
            hours, minutes, seconds, sub_frames
        )
    }
}

impl Default for Timebase {
    fn default() -> Self {
        Self::fps_30()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timebase_conversions() {
        let tb = Timebase::fps_30();
        assert_eq!(tb.seconds_to_frames(2.0), 60);
        assert_eq!(tb.frames_to_seconds(60), 2.0);
        assert_eq!(tb.timecode(90), "00:00:03:00");
    }

    #[test]
    fn test_ntsc_timebase() {
        let tb = Timebase::fps_29_97();
        assert!((tb.as_f64() - 29.97).abs() < 0.01);
    }
}
