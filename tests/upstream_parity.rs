use auto_editor::{
    audio::{compute_peak, compute_rms, is_audio_active, AudioThreshold},
    exports::{
        clip_sequence::{export_csv, export_edl},
        export_timeline,
        fcpxml::export_fcpxml,
        json::export_json,
        kdenlive::export_kdenlive,
        premiere::export_premiere,
        resolve::export_resolve,
        shotcut::export_shotcut,
        ExportFormat,
    },
    expr::{EditExpr, EditMethod},
    labels::{Action, LabelManager},
    margin::Margin,
    motion::{compute_motion_difference, MotionThreshold},
    rational::Timebase,
    render::{RenderEngine, RenderOptions},
    timeline::{Chunk, Timeline},
};

#[test]
fn test_upstream_audio_threshold_decibel_linear_parity() {
    // Upstream auto-editor test: dB to linear amplitude conversions
    // 0dB -> 1.0
    let t0 = AudioThreshold::parse("0dB").unwrap();
    assert!((t0.linear - 1.0).abs() < 1e-4);
    assert!((t0.to_db() - 0.0).abs() < 1e-4);

    // -6dB -> ~0.501
    let t6 = AudioThreshold::parse("-6dB").unwrap();
    assert!((t6.linear - 0.501187).abs() < 1e-3);

    // -20dB -> 0.1
    let t20 = AudioThreshold::parse("-20dB").unwrap();
    assert!((t20.linear - 0.1).abs() < 1e-4);
    assert!((t20.to_db() - (-20.0)).abs() < 1e-4);

    // Percentages: 4% -> 0.04
    let tp = AudioThreshold::parse("4%").unwrap();
    assert!((tp.linear - 0.04).abs() < 1e-6);

    // Float: 0.04
    let tf = AudioThreshold::parse("0.04").unwrap();
    assert!((tf.linear - 0.04).abs() < 1e-6);
}

#[test]
fn test_upstream_timebase_conversions() {
    // 30fps
    let tb30 = Timebase::fps_30();
    assert_eq!(tb30.seconds_to_frames(1.0), 30);
    assert_eq!(tb30.seconds_to_frames(2.5), 75);
    assert_eq!(tb30.frames_to_seconds(90), 3.0);
    assert_eq!(tb30.timecode(3600), "00:02:00:00");

    // 24fps film
    let tb24 = Timebase::fps_24();
    assert_eq!(tb24.seconds_to_frames(1.0), 24);
    assert_eq!(tb24.frames_to_seconds(48), 2.0);

    // NTSC 29.97 (30000/1001)
    let tb_ntsc = Timebase::fps_29_97();
    assert!((tb_ntsc.as_f64() - 29.97).abs() < 0.01);
}

#[test]
fn test_upstream_margin_behavior() {
    let tb = Timebase::fps_30();

    // Default 0.2s margin -> 6 frames
    let def_margin = Margin::default_for_tb(&tb);
    assert_eq!(def_margin.start_frames, 6);
    assert_eq!(def_margin.end_frames, 6);

    // Asymmetric margin: 0.1s before, 0.4s after
    let asym = Margin::parse("0.1s,0.4s", &tb).unwrap();
    assert_eq!(asym.start_frames, 3);
    assert_eq!(asym.end_frames, 12);

    // Exact frame margins: 5,10
    let f_margin = Margin::parse("5,10", &tb).unwrap();
    assert_eq!(f_margin.start_frames, 5);
    assert_eq!(f_margin.end_frames, 10);

    // Test margin expansion on active array
    // [F, F, T, F, F] with margin=1 -> [F, T, T, T, F]
    let m1 = Margin::new(1, 1);
    let frames = vec![false, false, true, false, false];
    let expanded = m1.apply(&frames);
    assert_eq!(expanded, vec![false, true, true, true, false]);

    // Test gap bridging
    let mut gap_frames = vec![true, false, false, true];
    Margin::bridge_gaps(&mut gap_frames, 2);
    assert_eq!(gap_frames, vec![true, true, true, true]);
}

#[test]
fn test_upstream_edit_expression_ast() {
    // 1. Simple audio method
    let expr1 = EditExpr::parse("audio:0.04").unwrap();
    assert_eq!(
        expr1,
        EditExpr::Method(EditMethod::Audio {
            threshold: AudioThreshold::from_linear(0.04),
            stream: 0,
        })
    );

    // 2. Simple motion method
    let expr2 = EditExpr::parse("motion:0.02").unwrap();
    assert_eq!(
        expr2,
        EditExpr::Method(EditMethod::Motion {
            threshold: MotionThreshold::new(0.02),
            stream: 0,
        })
    );

    // 3. Constant expressions
    let e_all = EditExpr::parse("all").unwrap();
    assert_eq!(e_all, EditExpr::Method(EditMethod::Const(true)));
    let e_none = EditExpr::parse("none").unwrap();
    assert_eq!(e_none, EditExpr::Method(EditMethod::Const(false)));

    // 4. Boolean OR expression: (or audio:0.05 motion:0.03)
    let e_or = EditExpr::parse("(or audio:0.05 motion:0.03)").unwrap();
    assert!(e_or.evaluate(&[0.10], 0.01)); // Audio passes
    assert!(e_or.evaluate(&[0.01], 0.05)); // Motion passes
    assert!(!e_or.evaluate(&[0.01], 0.01)); // Neither passes

    // 5. Boolean AND expression: (and audio:0.05 motion:0.03)
    let e_and = EditExpr::parse("(and audio:0.05 motion:0.03)").unwrap();
    assert!(e_and.evaluate(&[0.10], 0.05)); // Both pass
    assert!(!e_and.evaluate(&[0.10], 0.01)); // Motion fails

    // 6. Boolean NOT expression: (not audio:0.05)
    let e_not = EditExpr::parse("(not audio:0.05)").unwrap();
    assert!(e_not.evaluate(&[0.01], 0.0)); // Audio quiet -> NOT is true
    assert!(!e_not.evaluate(&[0.10], 0.0)); // Audio loud -> NOT is false
}

#[test]
fn test_upstream_labels_and_actions() {
    let mut lm = LabelManager::new();
    // Default: 0 is Cut, 1 is Keep
    assert_eq!(lm.get_action(0), Action::Cut);
    assert_eq!(lm.get_action(1), Action::Keep);

    // Custom speed label: 2 is Speed(2.5)
    lm.set_action(2, Action::Speed(2.5));
    assert_eq!(lm.get_action(2), Action::Speed(2.5));

    // Resolve sequence of frame labels
    let labels = vec![0, 0, 1, 1, 2];
    let resolved = lm.resolve_layers(&labels);
    assert_eq!(
        resolved,
        vec![
            Action::Cut,
            Action::Cut,
            Action::Keep,
            Action::Keep,
            Action::Speed(2.5)
        ]
    );
}

#[test]
fn test_upstream_timeline_and_chunks() {
    let tb = Timebase::fps_30();
    let actions = vec![
        Action::Cut,
        Action::Cut,
        Action::Keep,
        Action::Keep,
        Action::Keep,
        Action::Keep,
        Action::Speed(2.0),
        Action::Speed(2.0),
    ];

    let tl = Timeline::from_actions("source.mp4".to_string(), tb, &actions);
    assert_eq!(tl.chunks.len(), 3);
    assert_eq!(tl.chunks[0], Chunk::new(0, 2, 99999.0));
    assert_eq!(tl.chunks[1], Chunk::new(2, 6, 1.0));
    assert_eq!(tl.chunks[2], Chunk::new(6, 8, 2.0));

    let stats = tl.stats();
    assert_eq!(stats.original_frames, 8);
    // kept: chunk 1 (4 frames @ 1.0x = 4), chunk 2 (2 frames @ 2.0x = 1) -> 5 frames
    assert_eq!(stats.edited_frames, 5);
    assert_eq!(stats.cuts_count, 1);
    assert!(stats.time_saved_sec > 0.0);
    assert!(stats.percent_saved > 0.0);
}

#[test]
fn test_upstream_nle_interchange_exports() {
    let tl = Timeline::new(
        "/media/test.mp4".to_string(),
        Timebase::fps_30(),
        120,
        vec![
            Chunk::new(0, 30, 1.0),
            Chunk::new(30, 60, 99999.0), // cut
            Chunk::new(60, 90, 2.0),     // speedup
            Chunk::new(90, 120, 1.0),
        ],
    );

    // 1. Final Cut Pro XML (FCPXML)
    let fcpxml = export_fcpxml(&tl);
    assert!(fcpxml.contains("<fcpxml version=\"1.9\">"));
    assert!(fcpxml.contains("<asset-clip ref=\"r2\""));
    assert!(fcpxml.contains("<timeMap>"));

    // 2. Adobe Premiere Pro XML
    let premiere = export_premiere(&tl);
    assert!(premiere.contains("<xmeml version=\"4\">"));
    assert!(premiere.contains("<sequence>"));
    assert!(premiere.contains("<clipitem id=\"clipitem-1\">"));

    // 3. DaVinci Resolve XML
    let resolve = export_resolve(&tl);
    assert!(resolve.contains("<xmeml version=\"5\">"));
    assert!(resolve.contains("DaVinci Resolve"));

    // 4. Shotcut MLT
    let shotcut = export_shotcut(&tl);
    assert!(shotcut.contains("<mlt version=\"7.1.0\""));
    assert!(shotcut.contains("<producer id=\"producer0\""));
    assert!(shotcut.contains("<tractor id=\"tractor0\""));

    // 5. Kdenlive
    let kdenlive = export_kdenlive(&tl);
    assert!(kdenlive.contains("kdenlive:track_name"));

    // 6. CMX 3600 EDL
    let edl = export_edl(&tl);
    assert!(edl.contains("TITLE: AUTO-EDITOR SEQUENCE"));
    assert!(edl.contains("001  AX       V     C"));

    // 7. CSV Cut List
    let csv = export_csv(&tl);
    assert!(csv.contains("clip_index,start_frame,end_frame"));

    // 8. JSON
    let json = export_json(&tl).unwrap();
    assert!(json.contains("\"source_path\": \"/media/test.mp4\""));

    // 9. Dispatcher test
    let res_fcp = export_timeline(&tl, ExportFormat::FinalCutPro).unwrap();
    assert_eq!(res_fcp, fcpxml);
}

#[test]
fn test_upstream_render_command_builder() {
    let tl = Timeline::new(
        "sample.mp4".to_string(),
        Timebase::fps_30(),
        60,
        vec![
            Chunk::new(0, 30, 1.0),
            Chunk::new(30, 60, 99999.0), // cut
        ],
    );

    let opts = RenderOptions::default();
    let cmd = RenderEngine::build_ffmpeg_render_command(&tl, &opts).unwrap();
    assert_eq!(cmd[0], "ffmpeg");
    assert!(cmd.iter().any(|arg| arg.contains("concat=n=1")));

    let demuxer = RenderEngine::build_concat_demuxer_script(&tl);
    assert!(demuxer.contains("file 'sample.mp4'"));
    assert!(demuxer.contains("inpoint 0.0000"));
    assert!(demuxer.contains("outpoint 1.0000"));
}

#[test]
fn test_upstream_motion_difference_calculation() {
    // Identical frames -> 0 motion
    let f1 = vec![128u8; 100];
    let f2 = vec![128u8; 100];
    assert_eq!(compute_motion_difference(&f1, &f2), 0.0);

    // Inverted frames -> 1.0 motion
    let black = vec![0u8; 100];
    let white = vec![255u8; 100];
    assert!((compute_motion_difference(&black, &white) - 1.0).abs() < 1e-6);
}

#[test]
fn test_upstream_audio_rms_and_peak() {
    let pcm = vec![0.0f32, 0.5, -0.5, 0.0];
    let rms = compute_rms(&pcm);
    assert!((rms - (0.5f64 / 2.0f64.sqrt())).abs() < 1e-3);

    let peak = compute_peak(&pcm);
    assert!((peak - 0.5).abs() < 1e-6);

    let threshold = AudioThreshold::from_linear(0.2);
    assert!(is_audio_active(peak, threshold));
    assert!(!is_audio_active(0.1, threshold));
}

#[test]
fn test_upstream_lufs_vad_otio_and_take_scoring() {
    use auto_editor::{
        compute_lufs, compute_lufs_integrated, compute_vad_probability, parse_wav_pcm, score_takes,
        select_best_take, TakeCandidate,
    };

    // 1. ITU-R BS.1770-4 LUFS & VAD expression evaluation
    let expr = EditExpr::parse("(and audio:-24LUFS vad:0.6)").unwrap();
    assert!(expr.evaluate_with_vad(&[0.12], 0.0, 0.8));
    assert!(!expr.evaluate_with_vad(&[0.12], 0.0, 0.3));

    let sr = 48_000u32;
    let tone: Vec<f32> = (0..24_000)
        .map(|n| (0.5 * (2.0 * std::f64::consts::PI * 440.0 * n as f64 / sr as f64).sin()) as f32)
        .collect();
    let lufs = compute_lufs(&tone, sr);
    let lufs_int = compute_lufs_integrated(&tone, sr);
    assert!(lufs > -12.0 && lufs < -6.0);
    assert!((lufs - lufs_int).abs() < 0.5);
    assert!(compute_vad_probability(&tone[0..1024], sr) > 0.7);

    // 2. RIFF WAV PCM envelope extraction
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    let num_samples = 1600usize; // 0.1s at 16kHz -> 3 frames at 30fps
    let data_bytes = (num_samples * 2) as u32;
    wav.extend_from_slice(&(36u32 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&16000u32.to_le_bytes());
    wav.extend_from_slice(&32000u32.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for i in 0..num_samples {
        let val =
            ((2.0 * std::f64::consts::PI * 300.0 * i as f64 / 16000.0).sin() * 12000.0) as i16;
        wav.extend_from_slice(&val.to_le_bytes());
    }
    let decoded = parse_wav_pcm(&wav).unwrap();
    let tb = Timebase::fps_30();
    let rms_env = decoded.frame_rms_envelope(&tb);
    let vad_env = decoded.frame_vad_probabilities(&tb);
    assert_eq!(rms_env.len(), 3);
    assert_eq!(vad_env.len(), 3);
    assert!(rms_env[0] > 0.15);

    // 3. OpenTimelineIO (.otio) export with dynamic speed-up zone (1.75x)
    let tl = Timeline::new(
        "podcast.mp4".to_string(),
        tb,
        90,
        vec![Chunk::new(0, 45, 1.0), Chunk::new(45, 90, 1.75)],
    );
    let otio_str = export_timeline(&tl, ExportFormat::Otio).unwrap();
    assert!(otio_str.contains("\"OTIO_SCHEMA\": \"Timeline.1\""));
    assert!(otio_str.contains("\"OTIO_SCHEMA\": \"LinearTimeWarp.1\""));
    assert!(otio_str.contains("\"time_scalar\": 1.75"));

    // 4. Multi-take scoring & best take selection
    let takes = vec![
        TakeCandidate {
            start_frame: 0,
            end_frame: 60,
            transcript: "Welcome back— scratch that, let me try that again".to_string(),
            rms_levels: vec![0.15, 0.14, 0.12],
        },
        TakeCandidate {
            start_frame: 60,
            end_frame: 120,
            transcript: "Welcome back to the channel!".to_string(),
            rms_levels: vec![0.18, 0.20, 0.19],
        },
    ];
    let scores = score_takes(&takes, 0.04);
    assert!(scores[0].has_restart_marker);
    assert_eq!(select_best_take(&takes, 0.04), Some(1));
}
