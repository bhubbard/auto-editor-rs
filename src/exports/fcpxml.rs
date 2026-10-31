use crate::timeline::Timeline;
use std::path::Path;

pub fn export_fcpxml(timeline: &Timeline) -> String {
    let source_path = Path::new(&timeline.source_path);
    let file_name = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("media.mp4");
    let file_url = format!("file://{}", source_path.display());
    let _fps_ratio = format!(
        "{}/1001s",
        (timeline.timebase.as_f64() * 1001.0).round() as i64
    );
    let frame_dur = if (timeline.timebase.as_f64() - 29.97).abs() < 0.05 {
        "1001/30000s".to_string()
    } else if (timeline.timebase.as_f64() - 23.976).abs() < 0.05 {
        "1001/24000s".to_string()
    } else {
        format!("1/{}s", timeline.timebase.as_f64().round() as i64)
    };

    let total_dur_sec = timeline.timebase.frames_to_seconds(timeline.total_frames);
    let total_dur_str = format!("{:.3}s", total_dur_sec);

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<!DOCTYPE fcpxml>\n");
    xml.push_str("<fcpxml version=\"1.9\">\n");
    xml.push_str("  <resources>\n");
    xml.push_str(&format!(
        "    <format id=\"r1\" name=\"FFVideoFormat1080p\" frameDuration=\"{}\" width=\"1920\" height=\"1080\" />\n",
        frame_dur
    ));
    xml.push_str(&format!(
        "    <asset id=\"r2\" name=\"{}\" src=\"{}\" start=\"0s\" duration=\"{}\" hasVideo=\"1\" hasAudio=\"1\" format=\"r1\" />\n",
        file_name, file_url, total_dur_str
    ));
    xml.push_str("  </resources>\n");
    xml.push_str("  <library>\n");
    xml.push_str("    <event name=\"Auto-Editor Output\">\n");
    xml.push_str(&format!("      <project name=\"{}\">\n", file_name));
    xml.push_str(&format!(
        "        <sequence format=\"r1\" duration=\"{}\">\n",
        total_dur_str
    ));
    xml.push_str("          <spine>\n");

    let mut out_offset_frames = 0i64;
    for (i, chunk) in timeline.kept_chunks().iter().enumerate() {
        let in_sec = timeline.timebase.frames_to_seconds(chunk.start_frame);
        let dur_sec = timeline.timebase.frames_to_seconds(chunk.duration_frames());
        let offset_sec = timeline.timebase.frames_to_seconds(out_offset_frames);

        let speed = chunk.speed;
        let effective_dur_sec = dur_sec / speed;

        xml.push_str(&format!(
            "            <asset-clip ref=\"r2\" name=\"Clip {}\" offset=\"{:.4}s\" start=\"{:.4}s\" duration=\"{:.4}s\" format=\"r1\">\n",
            i + 1,
            offset_sec,
            in_sec,
            effective_dur_sec
        ));

        if (speed - 1.0).abs() > 1e-4 {
            xml.push_str(&format!(
                "              <timeMap><timept time=\"0s\" value=\"0s\" interp=\"smooth2\" /><timept time=\"{:.4}s\" value=\"{:.4}s\" interp=\"smooth2\" /></timeMap>\n",
                effective_dur_sec, dur_sec
            ));
        }

        xml.push_str("            </asset-clip>\n");
        out_offset_frames += chunk.output_frames();
    }

    xml.push_str("          </spine>\n");
    xml.push_str("        </sequence>\n");
    xml.push_str("      </project>\n");
    xml.push_str("    </event>\n");
    xml.push_str("  </library>\n");
    xml.push_str("</fcpxml>\n");

    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::Timebase;
    use crate::timeline::Chunk;

    #[test]
    fn test_fcpxml_export() {
        let tl = Timeline::new(
            "/path/to/video.mp4".to_string(),
            Timebase::fps_30(),
            90,
            vec![
                Chunk::new(0, 30, 1.0),
                Chunk::new(30, 60, 99999.0), // cut
                Chunk::new(60, 90, 1.0),
            ],
        );

        let xml = export_fcpxml(&tl);
        assert!(xml.contains("<fcpxml version=\"1.9\">"));
        assert!(xml.contains("Clip 1"));
        assert!(xml.contains("Clip 2"));
        assert!(!xml.contains("Clip 3")); // cut chunk not exported in spine
    }
}
