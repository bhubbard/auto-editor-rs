use crate::rational::Timebase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Margin {
    pub start_frames: i64,
    pub end_frames: i64,
}

impl Margin {
    pub fn new(start_frames: i64, end_frames: i64) -> Self {
        Self {
            start_frames: start_frames.max(0),
            end_frames: end_frames.max(0),
        }
    }

    /// Default margin: 0.2 seconds before and after
    pub fn default_for_tb(tb: &Timebase) -> Self {
        let frames = tb.seconds_to_frames(0.2);
        Self::new(frames, frames)
    }

    /// Parse margin string such as "0.2s", "0.2s,1.5s", "6", "6,12"
    pub fn parse(s: &str, tb: &Timebase) -> Result<Self, String> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.is_empty() {
            return Ok(Self::default_for_tb(tb));
        }

        let parse_part = |p: &str| -> Result<i64, String> {
            let trimmed = p.trim();
            if let Some(sec_str) = trimmed
                .strip_suffix("sec")
                .or_else(|| trimmed.strip_suffix('s'))
            {
                let secs = sec_str.parse::<f64>().map_err(|e| format!("Invalid seconds in margin '{}': {}", sec_str, e))?;
                Ok(tb.seconds_to_frames(secs))
            } else {
                let frames = trimmed.parse::<i64>().map_err(|e| format!("Invalid frame count in margin '{}': {}", trimmed, e))?;
                Ok(frames)
            }
        };

        if parts.len() == 1 {
            let f = parse_part(parts[0])?;
            Ok(Self::new(f, f))
        } else if parts.len() == 2 {
            let start = parse_part(parts[0])?;
            let end = parse_part(parts[1])?;
            Ok(Self::new(start, end))
        } else {
            Err(format!("Too many parts in margin specification: '{}'", s))
        }
    }

    /// Apply margin expansion to a boolean array representing active frames.
    pub fn apply(&self, active_frames: &[bool]) -> Vec<bool> {
        let n = active_frames.len();
        if n == 0 {
            return Vec::new();
        }

        let mut expanded = active_frames.to_vec();

        for (i, &is_active) in active_frames.iter().enumerate() {
            if is_active {
                let left = (i as i64 - self.start_frames).max(0) as usize;
                let right = ((i as i64 + self.end_frames) as usize).min(n - 1);
                expanded[left..=right].fill(true);
            }
        }

        expanded
    }

    /// Bridge short gaps (silences) that are shorter than `min_gap` frames
    pub fn bridge_gaps(frames: &mut [bool], min_gap: usize) {
        let n = frames.len();
        if n == 0 || min_gap == 0 {
            return;
        }

        let mut i = 0;
        while i < n {
            if !frames[i] {
                let start = i;
                while i < n && !frames[i] {
                    i += 1;
                }
                let gap_len = i - start;
                // If gap is bounded on both sides by active frames and is shorter than min_gap
                if start > 0 && i < n && gap_len <= min_gap {
                    frames[start..i].fill(true);
                }
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margin_parsing() {
        let tb = Timebase::fps_30();
        let m = Margin::parse("0.2s", &tb).unwrap();
        assert_eq!(m.start_frames, 6);
        assert_eq!(m.end_frames, 6);

        let m2 = Margin::parse("0.1s,0.5s", &tb).unwrap();
        assert_eq!(m2.start_frames, 3);
        assert_eq!(m2.end_frames, 15);
    }

    #[test]
    fn test_margin_expansion() {
        let _tb = Timebase::fps_30();
        let margin = Margin::new(2, 2);
        let mut input = vec![false; 10];
        input[4] = true; // active at frame 4

        let expanded = margin.apply(&input);
        // frames 2, 3, 4, 5, 6 should now be true
        assert!(!expanded[1]);
        assert!(expanded[2]);
        assert!(expanded[3]);
        assert!(expanded[4]);
        assert!(expanded[5]);
        assert!(expanded[6]);
        assert!(!expanded[7]);
    }

    #[test]
    fn test_bridge_gaps() {
        let mut frames = vec![true, false, false, true, true];
        Margin::bridge_gaps(&mut frames, 2);
        assert_eq!(frames, vec![true, true, true, true, true]);
    }
}
