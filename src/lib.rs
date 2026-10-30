//! # auto-editor (Rust Port) 🦀🎬
//!
//! A high-performance, pure Rust port of [Auto-Editor](https://github.com/wyattblue/auto-editor) —
//! automatic video and audio editing by acoustic loudness, motion analysis, and NLE timeline export.

pub mod audio;
pub mod expr;
pub mod exports;
pub mod labels;
pub mod margin;
pub mod motion;
pub mod rational;
pub mod render;
pub mod timeline;

pub use audio::{AudioThreshold, compute_peak, compute_rms, is_audio_active};
pub use expr::{EditExpr, EditMethod};
pub use exports::{ExportFormat, export_timeline};
pub use labels::{Action, LabelManager};
pub use margin::Margin;
pub use motion::{MotionThreshold, compute_motion_difference};
pub use rational::Timebase;
pub use render::{RenderEngine, RenderOptions};
pub use timeline::{Chunk, Timeline, TimelineStats};
