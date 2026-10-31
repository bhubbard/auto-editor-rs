pub mod clip_sequence;
pub mod fcpxml;
pub mod json;
pub mod kdenlive;
pub mod otio;
pub mod premiere;
pub mod resolve;
pub mod shotcut;

use crate::timeline::Timeline;
use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Premiere,
    Resolve,
    FinalCutPro,
    Shotcut,
    Kdenlive,
    Otio,
    Edl,
    Csv,
    Json,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Result<Self> {
        let s_lower = s.trim().to_lowercase();
        match s_lower.as_str() {
            "premiere" => Ok(ExportFormat::Premiere),
            "resolve" => Ok(ExportFormat::Resolve),
            "final-cut-pro" | "fcpxml" => Ok(ExportFormat::FinalCutPro),
            "shotcut" => Ok(ExportFormat::Shotcut),
            "kdenlive" => Ok(ExportFormat::Kdenlive),
            "otio" | "opentimelineio" => Ok(ExportFormat::Otio),
            "clip-sequence" | "edl" => Ok(ExportFormat::Edl),
            "csv" => Ok(ExportFormat::Csv),
            "json" => Ok(ExportFormat::Json),
            other => bail!("Unknown export format: '{}'. Supported: premiere, resolve, final-cut-pro, shotcut, kdenlive, otio, clip-sequence, csv, json", other),
        }
    }

    pub fn default_extension(&self) -> &'static str {
        match self {
            ExportFormat::Premiere | ExportFormat::Resolve => "xml",
            ExportFormat::FinalCutPro => "fcpxml",
            ExportFormat::Shotcut | ExportFormat::Kdenlive => "mlt",
            ExportFormat::Otio => "otio",
            ExportFormat::Edl => "edl",
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
        }
    }
}

pub fn export_timeline(timeline: &Timeline, format: ExportFormat) -> Result<String> {
    match format {
        ExportFormat::Premiere => Ok(premiere::export_premiere(timeline)),
        ExportFormat::Resolve => Ok(resolve::export_resolve(timeline)),
        ExportFormat::FinalCutPro => Ok(fcpxml::export_fcpxml(timeline)),
        ExportFormat::Shotcut => Ok(shotcut::export_shotcut(timeline)),
        ExportFormat::Kdenlive => Ok(kdenlive::export_kdenlive(timeline)),
        ExportFormat::Otio => otio::export_otio(timeline),
        ExportFormat::Edl => Ok(clip_sequence::export_edl(timeline)),
        ExportFormat::Csv => Ok(clip_sequence::export_csv(timeline)),
        ExportFormat::Json => json::export_json(timeline),
    }
}
