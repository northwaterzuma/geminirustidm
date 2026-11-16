//! Real product-code tests for yt-dlp error classification and media format selection.
//! Covers TEST-06F (error categories) and TEST-07D (selection strategy).

use mdm_ipc::{MediaIntent, MediaKind, MediaPreset, SizeKind, SubtitlePolicy};
use mdm_tools::media_policy::{select_formats, FormatChoice, TrackRole};
use mdm_tools::ytdlp_errors::{classify_ytdlp_stderr, YtDlpErrorCategory};

fn mk_format(id: &str, role: TrackRole, h: Option<u32>, vcodec: Option<&str>, acodec: Option<&str>) -> FormatChoice {
    FormatChoice {
        format_id: id.to_string(),
        role,
        protocol: "https".to_string(),
        video_codec: vcodec.map(str::to_string),
        audio_codec: acodec.map(str::to_string),
        container: "mp4".to_string(),
        width: None,
        height: h,
        fps: Some(30),
        dynamic_range: "SDR".to_string(),
        language: Some("original".to_string()),
        is_original: true,
        is_auto_caption: false,
        bitrate_kbps: Some(5000),
        size_bytes: Some(1024 * 1024),
        size_kind: SizeKind::Exact,
    }
}

fn intent() -> MediaIntent {
    MediaIntent {
        version: 1,
        kind: MediaKind::VideoAudio,
        preset: MediaPreset::HighestQuality,
        max_height: None,
        codec_preference: vec!["av1".into(), "vp9".into(), "h264".into()],
        container: "auto".into(),
        hdr: "preserve".into(),
        audio_language: "original".into(),
        subtitles: SubtitlePolicy { languages: vec![], source: "manual_preferred".into(), mode: "sidecar".into(), required: false },
        allow_quality_downgrade: false,
        allow_transcode: false,
        require_exact_format: false,
        selected_format_ids: vec![],
    }
}

// ---------- TEST-06F: error classification must not conflate all 403 ----------

#[test]
fn test_error_classifier_distinguishes_all_categories() {
    let cases: Vec<(&str, YtDlpErrorCategory)> = vec![
        ("HTTP Error 429: Too Many Requests", YtDlpErrorCategory::RateLimited),
        ("Sign in to confirm you're not a bot", YtDlpErrorCategory::BotBlocked),
        ("Your IP is being blocked", YtDlpErrorCategory::IpBlocked),
        ("Video not available in your country", YtDlpErrorCategory::GeoBlocked),
        ("Cookies are no longer valid", YtDlpErrorCategory::AuthRejected),
        ("This video is private", YtDlpErrorCategory::AuthRequired),
        ("Requested format is not available", YtDlpErrorCategory::FormatUnavailable),
        ("Unable to extract video info", YtDlpErrorCategory::ExtractorChanged),
        ("Widevine DRM protected", YtDlpErrorCategory::DrmProtected),
        ("No space left on device", YtDlpErrorCategory::DiskFull),
        ("ERROR: postprocessing: ffmpeg exited with code 1", YtDlpErrorCategory::MergeFailed),
        ("connection reset by peer", YtDlpErrorCategory::Network),
    ];
    for (msg, expected) in cases {
        let (got, _) = classify_ytdlp_stderr(msg);
        assert_eq!(got, expected, "misclassified: {}", msg);
    }
}

#[test]
fn test_only_network_and_rate_limited_are_retryable() {
    for (msg, retryable) in [
        ("HTTP Error 429: Too Many Requests", true),
        ("connection reset by peer", true),
        ("Private video", false),
        ("Widevine DRM protected", false),
        ("Requested format is not available", false),
    ] {
        let (_, detail) = classify_ytdlp_stderr(msg);
        assert_eq!(detail.retryable, retryable, "retryable wrong for: {}", msg);
    }
}

#[test]
fn test_rate_limited_has_retry_after() {
    let (_, detail) = classify_ytdlp_stderr("HTTP Error 429: Too Many Requests");
    assert_eq!(detail.retry_after_ms, Some(60_000));
}

// ---------- TEST-07D: format selection ----------

#[test]
fn test_highest_quality_picks_tallest_available() {
    let mut i = intent();
    i.max_height = Some(1080);
    let available = vec![
        mk_format("720p", TrackRole::VideoOnly, Some(720), Some("av1"), None),
        mk_format("1080p", TrackRole::VideoOnly, Some(1080), Some("av1"), None),
        mk_format("audio", TrackRole::AudioOnly, None, None, Some("aac")),
    ];
    let r = select_formats(&i, &available).unwrap();
    assert_eq!(r.selected_video.unwrap().format_id, "1080p");
    assert_eq!(r.selected_audio.unwrap().format_id, "audio");
    assert!(!r.quality_downgraded);
}

#[test]
fn test_max_height_is_upper_bound_not_guarantee() {
    let mut i = intent();
    i.max_height = Some(2160);
    let available = vec![mk_format("1080p", TrackRole::VideoOnly, Some(1080), Some("h264"), None)];
    let r = select_formats(&i, &available).unwrap();
    // Up to 2160 is allowed; 1080 must be accepted, not rejected as "not 4K".
    assert_eq!(r.selected_video.unwrap().height, Some(1080));
}

#[test]
fn test_exact_missing_format_fails_without_silent_downgrade() {
    let mut i = intent();
    i.require_exact_format = true;
    i.selected_format_ids = vec!["2160p60-hdr".to_string()];
    let available = vec![mk_format("1080p", TrackRole::VideoOnly, Some(1080), Some("h264"), None)];
    let e = select_formats(&i, &available).unwrap_err();
    assert_eq!(e.code, mdm_ipc::ErrorCode::FormatUnavailable);
}

#[test]
fn test_existing_exact_format_is_honored() {
    let mut i = intent();
    i.require_exact_format = true;
    i.selected_format_ids = vec!["1080p".to_string()];
    let available = vec![
        mk_format("720p", TrackRole::VideoOnly, Some(720), Some("h264"), None),
        mk_format("1080p", TrackRole::VideoOnly, Some(1080), Some("h264"), None),
    ];
    let r = select_formats(&i, &available).unwrap();
    assert_eq!(r.selected_video.unwrap().format_id, "1080p");
    assert!(!r.quality_downgraded);
}

#[test]
fn test_incompatible_codec_explicitly_rejected_when_transcode_disallowed() {
    let mut i = intent();
    i.container = "mp4".to_string();
    i.allow_transcode = false;
    // VP8 cannot be muxed into MP4. The product must reject rather than
    // pretend a format change is a conversion.
    let fmt = mk_format("vp8", TrackRole::VideoOnly, Some(1080), Some("vp8"), None);
    let available = vec![fmt];
    let e = select_formats(&i, &available).unwrap_err();
    assert_eq!(e.code, mdm_ipc::ErrorCode::FormatUnavailable);
    assert!(e.message.contains("transcode"), "must explain transcode is required");
}
