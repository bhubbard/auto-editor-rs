use crate::timeline::Timeline;
use std::path::Path;

pub fn export_kdenlive(timeline: &Timeline) -> String {
    let source_path = Path::new(&timeline.source_path);
    let file_path = source_path.display().to_string();
    let fps_f64 = timeline.timebase.as_f64();

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str(
        "<mlt version=\"7.1.0\" title=\"Auto-Editor Kdenlive Export\" producer=\"main_bin\">\n",
    );
    xml.push_str(&format!(
        "  <profile description=\"HD 1080p\" width=\"1920\" height=\"1080\" progressive=\"1\" frame_rate_num=\"{}\" frame_rate_den=\"1000\" />\n",
        (fps_f64 * 1000.0).round() as i64
    ));
    xml.push_str(&format!(
        "  <producer id=\"producer0\" in=\"0\" out=\"{}\">\n",
        timeline.total_frames
    ));
    xml.push_str(&format!(
        "    <property name=\"resource\">{}</property>\n",
        file_path
    ));
    xml.push_str("    <property name=\"kdenlive:id\">1</property>\n");
    xml.push_str("  </producer>\n");
    xml.push_str("  <playlist id=\"playlist_track1\">\n");
    xml.push_str("    <property name=\"kdenlive:track_name\">Video 1</property>\n");

    for chunk in timeline.kept_chunks() {
        let in_frame = chunk.start_frame;
        let out_frame = chunk.end_frame - 1;
        xml.push_str(&format!(
            "    <entry producer=\"producer0\" in=\"{}\" out=\"{}\" />\n",
            in_frame, out_frame
        ));
    }

    xml.push_str("  </playlist>\n");
    xml.push_str("  <tractor id=\"maintractor\">\n");
    xml.push_str("    <track producer=\"playlist_track1\" />\n");
    xml.push_str("  </tractor>\n");
    xml.push_str("</mlt>\n");

    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::Timebase;
    use crate::timeline::Chunk;

    #[test]
    fn test_kdenlive_export() {
        let tl = Timeline::new(
            "/path/to/media.mp4".to_string(),
            Timebase::fps_30(),
            100,
            vec![Chunk::new(0, 50, 1.0)],
        );

        let xml = export_kdenlive(&tl);
        assert!(xml.contains("kdenlive:track_name"));
        assert!(xml.contains("<entry producer=\"producer0\" in=\"0\" out=\"49\" />"));
    }
}
