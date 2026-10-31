use crate::timeline::Timeline;
use std::path::Path;

pub fn export_premiere(timeline: &Timeline) -> String {
    let source_path = Path::new(&timeline.source_path);
    let file_name = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("media.mp4");
    let file_path = source_path.display().to_string();
    let timebase_int = timeline.timebase.as_f64().round() as i64;
    let total_output_frames: i64 = timeline
        .kept_chunks()
        .iter()
        .map(|c| c.output_frames())
        .sum();

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<!DOCTYPE xmeml>\n");
    xml.push_str("<xmeml version=\"4\">\n");
    xml.push_str("  <sequence>\n");
    xml.push_str(&format!("    <name>Auto-Editor: {}</name>\n", file_name));
    xml.push_str(&format!(
        "    <duration>{}</duration>\n",
        total_output_frames
    ));
    xml.push_str("    <rate>\n");
    xml.push_str(&format!("      <timebase>{}</timebase>\n", timebase_int));
    xml.push_str("      <ntsc>FALSE</ntsc>\n");
    xml.push_str("    </rate>\n");
    xml.push_str("    <media>\n");
    xml.push_str("      <video>\n");
    xml.push_str("        <format>\n");
    xml.push_str("          <samplecharacteristics>\n");
    xml.push_str(&format!(
        "            <rate><timebase>{}</timebase></rate>\n",
        timebase_int
    ));
    xml.push_str("            <width>1920</width>\n");
    xml.push_str("            <height>1080</height>\n");
    xml.push_str("          </samplecharacteristics>\n");
    xml.push_str("        </format>\n");
    xml.push_str("        <track>\n");

    let mut timeline_cursor = 0i64;
    for (i, chunk) in timeline.kept_chunks().iter().enumerate() {
        let in_frame = chunk.start_frame;
        let out_frame = chunk.end_frame;
        let dur = chunk.output_frames();

        xml.push_str("          <clipitem id=\"clipitem-");
        xml.push_str(&(i + 1).to_string());
        xml.push_str("\">\n");
        xml.push_str(&format!(
            "            <name>{} [{}]</name>\n",
            file_name,
            i + 1
        ));
        xml.push_str(&format!("            <duration>{}</duration>\n", dur));
        xml.push_str(&format!("            <start>{}</start>\n", timeline_cursor));
        xml.push_str(&format!(
            "            <end>{}</end>\n",
            timeline_cursor + dur
        ));
        xml.push_str(&format!("            <in>{}</in>\n", in_frame));
        xml.push_str(&format!("            <out>{}</out>\n", out_frame));
        xml.push_str("            <file id=\"file-1\">\n");
        xml.push_str(&format!("              <name>{}</name>\n", file_name));
        xml.push_str(&format!(
            "              <pathurl>file://{}</pathurl>\n",
            file_path
        ));
        xml.push_str("              <rate>\n");
        xml.push_str(&format!(
            "                <timebase>{}</timebase>\n",
            timebase_int
        ));
        xml.push_str("              </rate>\n");
        xml.push_str("            </file>\n");
        xml.push_str("          </clipitem>\n");

        timeline_cursor += dur;
    }

    xml.push_str("        </track>\n");
    xml.push_str("      </video>\n");
    xml.push_str("    </media>\n");
    xml.push_str("  </sequence>\n");
    xml.push_str("</xmeml>\n");

    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::Timebase;
    use crate::timeline::Chunk;

    #[test]
    fn test_premiere_export() {
        let tl = Timeline::new(
            "/path/to/source.mp4".to_string(),
            Timebase::fps_30(),
            120,
            vec![
                Chunk::new(0, 30, 1.0),
                Chunk::new(30, 90, 99999.0),
                Chunk::new(90, 120, 1.0),
            ],
        );

        let xml = export_premiere(&tl);
        assert!(xml.contains("<xmeml version=\"4\">"));
        assert!(xml.contains("<clipitem id=\"clipitem-1\">"));
        assert!(xml.contains("<clipitem id=\"clipitem-2\">"));
        assert_eq!(tl.kept_chunks().len(), 2);
    }
}
