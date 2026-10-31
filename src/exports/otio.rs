use crate::timeline::Timeline;
use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;

fn rational_time(value: f64, rate: f64) -> Value {
    json!({
        "OTIO_SCHEMA": "RationalTime.1",
        "value": value,
        "rate": rate
    })
}

fn time_range(start_frames: f64, duration_frames: f64, rate: f64) -> Value {
    json!({
        "OTIO_SCHEMA": "TimeRange.1",
        "start_time": rational_time(start_frames, rate),
        "duration": rational_time(duration_frames, rate)
    })
}

fn build_track_children(timeline: &Timeline, rate: f64) -> Vec<Value> {
    let source_path = Path::new(&timeline.source_path);
    let file_name = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("media.mp4");
    let total_dur = timeline.total_frames as f64;

    timeline
        .kept_chunks()
        .iter()
        .enumerate()
        .map(|(idx, chunk)| {
            let start = chunk.start_frame as f64;
            let out_dur = chunk.output_frames() as f64;
            let effects = if (chunk.speed - 1.0).abs() > 1e-4 {
                vec![json!({
                    "OTIO_SCHEMA": "LinearTimeWarp.1",
                    "name": "SpeedModify",
                    "effect_name": "LinearTimeWarp",
                    "time_scalar": chunk.speed,
                    "metadata": {}
                })]
            } else {
                Vec::new()
            };

            json!({
                "OTIO_SCHEMA": "Clip.2",
                "name": format!("{} ({})", file_name, idx + 1),
                "source_range": time_range(start, out_dur, rate),
                "effects": effects,
                "markers": [],
                "enabled": true,
                "metadata": {},
                "media_references": {
                    "DEFAULT_MEDIA": {
                        "OTIO_SCHEMA": "ExternalReference.1",
                        "name": file_name,
                        "target_url": timeline.source_path,
                        "available_range": time_range(0.0, total_dur, rate),
                        "metadata": {}
                    }
                },
                "active_media_reference_key": "DEFAULT_MEDIA"
            })
        })
        .collect()
}

/// Exports timeline to Academy Software Foundation OpenTimelineIO (`.otio`) JSON schema
/// with synchronized `Video 1` and `Audio 1` tracks and `LinearTimeWarp.1` speed effects.
pub fn export_otio(timeline: &Timeline) -> Result<String> {
    let rate = (timeline.timebase.as_f64() * 1000.0).round() / 1000.0;
    let video_clips = build_track_children(timeline, rate);
    let audio_clips = build_track_children(timeline, rate);

    let doc = json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": "Auto-Editor Timeline",
        "global_start_time": rational_time(0.0, rate),
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "name": "tracks",
            "source_range": Value::Null,
            "effects": [],
            "markers": [],
            "enabled": true,
            "metadata": {},
            "children": [
                {
                    "OTIO_SCHEMA": "Track.1",
                    "name": "Video 1",
                    "kind": "Video",
                    "source_range": Value::Null,
                    "effects": [],
                    "markers": [],
                    "enabled": true,
                    "metadata": {},
                    "children": video_clips
                },
                {
                    "OTIO_SCHEMA": "Track.1",
                    "name": "Audio 1",
                    "kind": "Audio",
                    "source_range": Value::Null,
                    "effects": [],
                    "markers": [],
                    "enabled": true,
                    "metadata": {},
                    "children": audio_clips
                }
            ]
        },
        "metadata": {
            "generator": "auto-editor-rs"
        }
    });

    Ok(serde_json::to_string_pretty(&doc)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::Timebase;
    use crate::timeline::Chunk;

    #[test]
    fn test_otio_export_with_speed_warp() {
        let tl = Timeline::new(
            "interview.mp4".to_string(),
            Timebase::fps_30(),
            90,
            vec![
                Chunk::new(0, 30, 1.0),
                Chunk::new(30, 60, 99999.0),
                Chunk::new(60, 90, 2.0),
            ],
        );

        let otio = export_otio(&tl).unwrap();
        assert!(otio.contains("\"OTIO_SCHEMA\": \"Timeline.1\""));
        assert!(otio.contains("\"OTIO_SCHEMA\": \"LinearTimeWarp.1\""));
        assert!(otio.contains("\"time_scalar\": 2.0"));
        assert!(otio.contains("\"kind\": \"Video\""));
        assert!(otio.contains("\"kind\": \"Audio\""));
    }
}
