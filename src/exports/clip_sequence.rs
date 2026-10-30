use crate::timeline::Timeline;

/// Exports CMX 3600 Edit Decision List (EDL)
pub fn export_edl(timeline: &Timeline) -> String {
    let mut edl = String::new();
    edl.push_str("TITLE: AUTO-EDITOR SEQUENCE\n");
    edl.push_str("FCM: NON-DROP FRAME\n\n");

    let tb = &timeline.timebase;
    let mut record_cursor = 0i64;

    for (i, chunk) in timeline.kept_chunks().iter().enumerate() {
        let src_in = tb.timecode(chunk.start_frame);
        let src_out = tb.timecode(chunk.end_frame);
        let dur = chunk.output_frames();
        let rec_in = tb.timecode(record_cursor);
        let rec_out = tb.timecode(record_cursor + dur);

        edl.push_str(&format!(
            "{:03}  AX       V     C        {} {} {} {}\n",
            i + 1,
            src_in,
            src_out,
            rec_in,
            rec_out
        ));
        edl.push_str(&format!("* FROM CLIP: {}\n\n", timeline.source_path));

        record_cursor += dur;
    }

    edl
}

/// Exports CSV cut list
pub fn export_csv(timeline: &Timeline) -> String {
    let mut csv = String::new();
    csv.push_str("clip_index,start_frame,end_frame,start_sec,end_sec,duration_sec,speed\n");

    for (i, chunk) in timeline.kept_chunks().iter().enumerate() {
        let start_sec = timeline.timebase.frames_to_seconds(chunk.start_frame);
        let end_sec = timeline.timebase.frames_to_seconds(chunk.end_frame);
        let dur_sec = end_sec - start_sec;

        csv.push_str(&format!(
            "{},{},{},{:.4},{:.4},{:.4},{:.2}\n",
            i + 1,
            chunk.start_frame,
            chunk.end_frame,
            start_sec,
            end_sec,
            dur_sec,
            chunk.speed
        ));
    }

    csv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::Timebase;
    use crate::timeline::Chunk;

    #[test]
    fn test_edl_export() {
        let tl = Timeline::new(
            "source.mp4".to_string(),
            Timebase::fps_30(),
            60,
            vec![Chunk::new(0, 30, 1.0)],
        );

        let edl = export_edl(&tl);
        assert!(edl.contains("TITLE: AUTO-EDITOR SEQUENCE"));
        assert!(edl.contains("001  AX       V     C"));
    }

    #[test]
    fn test_csv_export() {
        let tl = Timeline::new(
            "source.mp4".to_string(),
            Timebase::fps_30(),
            60,
            vec![Chunk::new(0, 30, 1.0)],
        );

        let csv = export_csv(&tl);
        assert!(csv.contains("clip_index,start_frame"));
        assert!(csv.contains("1,0,30,0.0000,1.0000,1.0000,1.00"));
    }
}
