use crate::timeline::Timeline;
use anyhow::Result;

pub fn export_json(timeline: &Timeline) -> Result<String> {
    let json = serde_json::to_string_pretty(timeline)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rational::Timebase;
    use crate::timeline::Chunk;

    #[test]
    fn test_json_export() {
        let tl = Timeline::new(
            "sample.mp4".to_string(),
            Timebase::fps_30(),
            60,
            vec![Chunk::new(0, 60, 1.0)],
        );

        let json = export_json(&tl).unwrap();
        assert!(json.contains("\"source_path\": \"sample.mp4\""));
        assert!(json.contains("\"chunks\""));
    }
}
