//! # auto-editor (Rust Port) 🦀🎬
//!
//! A high-performance, pure Rust port of [Auto-Editor](https://github.com/wyattblue/auto-editor) —
//! automatic video and audio editing by acoustic loudness, motion analysis, and NLE timeline export.

pub mod audio;
pub mod exports;
pub mod expr;
pub mod labels;
pub mod margin;
pub mod motion;
pub mod rational;
pub mod render;
pub mod timeline;

pub use audio::{
    compute_lufs, compute_lufs_integrated, compute_peak, compute_rms, compute_vad_probability,
    is_audio_active, parse_wav_pcm, AudioThreshold, BiquadFilter, KWeightFilter, WavAudio,
};
pub use exports::{export_timeline, ExportFormat};
pub use expr::{EditExpr, EditMethod};
pub use labels::{Action, LabelManager};
pub use margin::Margin;
pub use motion::{compute_motion_difference, MotionThreshold};
pub use rational::Timebase;
pub use render::{RenderEngine, RenderOptions};
pub use timeline::{
    score_takes, select_best_take, Chunk, TakeCandidate, TakeScore, Timeline, TimelineStats,
};
