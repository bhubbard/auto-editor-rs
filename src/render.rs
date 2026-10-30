use crate::timeline::Timeline;
use anyhow::{bail, Result};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub output_path: PathBuf,
    pub video_codec: String,
    pub audio_codec: String,
    pub crf: u32,
    pub preset: String,
    pub faststart: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("output.mp4"),
            video_codec: "libx264".to_string(),
            audio_codec: "aac".to_string(),
            crf: 18,
            preset: "fast".to_string(),
            faststart: true,
        }
    }
}

pub struct RenderEngine;

impl RenderEngine {
    /// Builds an FFmpeg command with a concat filter or filter_complex that applies
    /// cuts and speeds across all kept chunks.
    pub fn build_ffmpeg_render_command(
        timeline: &Timeline,
        opts: &RenderOptions,
    ) -> Result<Vec<String>> {
        let kept = timeline.kept_chunks();
        if kept.is_empty() {
            bail!("No active chunks to render! Entire media was cut.");
        }

        let tb = &timeline.timebase;
        let mut cmd = vec!["ffmpeg".to_string(), "-y".to_string()];

        // Input
        cmd.push("-i".to_string());
        cmd.push(timeline.source_path.clone());

        let mut filter_complex = String::new();
        let mut concat_inputs = String::new();

        for (i, chunk) in kept.iter().enumerate() {
            let start_sec = tb.frames_to_seconds(chunk.start_frame);
            let end_sec = tb.frames_to_seconds(chunk.end_frame);
            let speed = chunk.speed;

            // Video trim & PTS
            let v_pts_mult = 1.0 / speed;
            let v_filter = format!(
                "[0:v]trim=start={:.4}:end={:.4},setpts=PTS-STARTPTS,setpts={:.4}*PTS[v{}]",
                start_sec, end_sec, v_pts_mult, i
            );

            // Audio trim & PTS / tempo
            let a_filter = if (speed - 1.0).abs() < 1e-4 {
                format!(
                    "[0:a]atrim=start={:.4}:end={:.4},asetpts=PTS-STARTPTS[a{}]",
                    start_sec, end_sec, i
                )
            } else {
                format!(
                    "[0:a]atrim=start={:.4}:end={:.4},asetpts=PTS-STARTPTS,atempo={:.4}[a{}]",
                    start_sec, end_sec, speed, i
                )
            };

            filter_complex.push_str(&format!("{};{};", v_filter, a_filter));
            concat_inputs.push_str(&format!("[v{}][a{}]", i, i));
        }

        filter_complex.push_str(&format!(
            "{}concat=n={}:v=1:a=1[outv][outa]",
            concat_inputs,
            kept.len()
        ));

        cmd.push("-filter_complex".to_string());
        cmd.push(filter_complex);

        cmd.push("-map".to_string());
        cmd.push("[outv]".to_string());
        cmd.push("-map".to_string());
        cmd.push("[outa]".to_string());

        cmd.push("-c:v".to_string());
        cmd.push(opts.video_codec.clone());
        cmd.push("-preset".to_string());
        cmd.push(opts.preset.clone());
        cmd.push("-crf".to_string());
        cmd.push(opts.crf.to_string());

        cmd.push("-c:a".to_string());
        cmd.push(opts.audio_codec.clone());
        cmd.push("-b:a".to_string());
        cmd.push("192k".to_string());

        if opts.faststart {
            cmd.push("-movflags".to_string());
            cmd.push("+faststart".to_string());
        }

        cmd.push(opts.output_path.to_string_lossy().to_string());

        Ok(cmd)
    }

    /// Build a concat demuxer cut list (fastest lossless cut if FFmpeg is installed)
    pub fn build_concat_demuxer_script(timeline: &Timeline) -> String {
        let mut script = String::new();
        let tb = &timeline.timebase;
        let source_escaped = timeline.source_path.replace('\'', "'\\''");

        for chunk in timeline.kept_chunks() {
            let in_sec = tb.frames_to_seconds(chunk.start_frame);
            let out_sec = tb.frames_to_seconds(chunk.end_frame);
            let dur_sec = out_sec - in_sec;

            script.push_str(&format!("file '{}'\n", source_escaped));
            script.push_str(&format!("inpoint {:.4}\n", in_sec));
            script.push_str(&format!("outpoint {:.4}\n", out_sec));
            script.push_str(&format!("duration {:.4}\n", dur_sec));
        }

        script
    }

    /// Execute FFmpeg command directly
    pub fn run_ffmpeg(args: &[String]) -> Result<()> {
        if args.is_empty() {
            bail!("Empty FFmpeg command");
        }

        let status = Command::new(&args[0])
            .args(&args[1..])
            .status()?;

        if !status.success() {
            bail!("FFmpeg exited with status: {}", status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::Timebase;
    use crate::timeline::Chunk;

    #[test]
    fn test_render_cmd_generation() {
        let tl = Timeline::new(
            "input.mp4".to_string(),
            Timebase::fps_30(),
            60,
            vec![
                Chunk::new(0, 30, 1.0),
                Chunk::new(30, 60, 2.0),
            ],
        );

        let opts = RenderOptions::default();
        let cmd = RenderEngine::build_ffmpeg_render_command(&tl, &opts).unwrap();

        assert_eq!(cmd[0], "ffmpeg");
        let filter_idx = cmd.iter().position(|r| r == "-filter_complex").unwrap();
        let filter = &cmd[filter_idx + 1];
        assert!(filter.contains("concat=n=2:v=1:a=1"));
        assert!(filter.contains("trim=start=0.0000:end=1.0000"));
    }

    #[test]
    fn test_concat_demuxer_script() {
        let tl = Timeline::new(
            "input.mp4".to_string(),
            Timebase::fps_30(),
            60,
            vec![Chunk::new(0, 30, 1.0)],
        );

        let script = RenderEngine::build_concat_demuxer_script(&tl);
        assert!(script.contains("file 'input.mp4'"));
        assert!(script.contains("inpoint 0.0000"));
        assert!(script.contains("outpoint 1.0000"));
    }
}
