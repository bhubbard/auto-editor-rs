use crate::labels::Action;
use crate::rational::Timebase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    pub start_frame: i64,
    pub end_frame: i64,
    pub speed: f64,
}

impl Chunk {
    pub fn new(start_frame: i64, end_frame: i64, speed: f64) -> Self {
        Self {
            start_frame,
            end_frame,
            speed,
        }
    }

    pub fn duration_frames(&self) -> i64 {
        self.end_frame - self.start_frame
    }

    pub fn output_frames(&self) -> i64 {
        if self.is_cut() {
            0
        } else {
            (self.duration_frames() as f64 / self.speed).round() as i64
        }
    }

    pub fn is_cut(&self) -> bool {
        self.speed > 1000.0 || self.speed <= 0.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineStats {
    pub original_frames: i64,
    pub edited_frames: i64,
    pub original_duration_sec: f64,
    pub edited_duration_sec: f64,
    pub time_saved_sec: f64,
    pub percent_saved: f64,
    pub chunks_count: usize,
    pub cuts_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub source_path: String,
    pub timebase: Timebase,
    pub total_frames: i64,
    pub chunks: Vec<Chunk>,
}

impl Timeline {
    pub fn new(source_path: String, timebase: Timebase, total_frames: i64, chunks: Vec<Chunk>) -> Self {
        Self {
            source_path,
            timebase,
            total_frames,
            chunks,
        }
    }

    /// Build contiguous chunks from an array of per-frame actions
    pub fn from_actions(source_path: String, timebase: Timebase, actions: &[Action]) -> Self {
        let total_frames = actions.len() as i64;
        if total_frames == 0 {
            return Self::new(source_path, timebase, 0, Vec::new());
        }

        let mut chunks = Vec::new();
        let mut curr_speed = actions[0].speed_multiplier();
        let mut chunk_start = 0i64;

        for (i, action) in actions.iter().enumerate().skip(1) {
            let speed = action.speed_multiplier();
            if (speed - curr_speed).abs() > 1e-6 {
                chunks.push(Chunk::new(chunk_start, i as i64, curr_speed));
                curr_speed = speed;
                chunk_start = i as i64;
            }
        }

        chunks.push(Chunk::new(chunk_start, total_frames, curr_speed));
        Self::new(source_path, timebase, total_frames, chunks)
    }

    /// Filter to only kept chunks (excluding cuts)
    pub fn kept_chunks(&self) -> Vec<Chunk> {
        self.chunks.iter().copied().filter(|c| !c.is_cut()).collect()
    }

    /// Calculate timeline statistics
    pub fn stats(&self) -> TimelineStats {
        let orig_sec = self.timebase.frames_to_seconds(self.total_frames);
        let edited_frames: i64 = self.chunks.iter().map(|c| c.output_frames()).sum();
        let edited_sec = self.timebase.frames_to_seconds(edited_frames);
        let saved_sec = (orig_sec - edited_sec).max(0.0);
        let percent_saved = if orig_sec > 0.0 {
            (saved_sec / orig_sec) * 100.0
        } else {
            0.0
        };

        let cuts_count = self.chunks.iter().filter(|c| c.is_cut()).count();

        TimelineStats {
            original_frames: self.total_frames,
            edited_frames,
            original_duration_sec: orig_sec,
            edited_duration_sec: edited_sec,
            time_saved_sec: saved_sec,
            percent_saved,
            chunks_count: self.chunks.len(),
            cuts_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunks_from_actions() {
        let tb = Timebase::fps_30();
        let actions = vec![
            Action::Cut,
            Action::Cut,
            Action::Keep,
            Action::Keep,
            Action::Keep,
            Action::Speed(2.0),
        ];

        let tl = Timeline::from_actions("test.mp4".to_string(), tb, &actions);
        assert_eq!(tl.chunks.len(), 3);
        assert_eq!(tl.chunks[0], Chunk::new(0, 2, 99999.0));
        assert_eq!(tl.chunks[1], Chunk::new(2, 5, 1.0));
        assert_eq!(tl.chunks[2], Chunk::new(5, 6, 2.0));

        let stats = tl.stats();
        assert_eq!(stats.original_frames, 6);
        // chunk 0: 0 frames, chunk 1: 3 frames, chunk 2: (1/2.0).round() = 1 frame -> 4 frames
        assert_eq!(stats.edited_frames, 4);
    }
}
