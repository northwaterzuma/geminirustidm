//! Real product-code tests for HLS/DASH parsing safety and yt-dlp argv hardening.
//! Covers TEST-05B (manifest safety) and TEST-06/12A (no shell, locked paths).

use mdm_ipc::ErrorCode;
use mdm_tools::streams::DrmStatus;
use mdm_tools::proc::build_ytdlp_command;
use mdm_tools::streams::{parse_dash_manifest, parse_hls_manifest, resolve_and_validate_media_uri};
use mdm_tools::video::extract_youtube_identity;
use std::path::Path;

const BASE: &str = "https://cdn.example.invalid/hls/master.m3u8";

// ---------- TEST-05B: manifest URI validation (SSRF / local file) ----------

#[test]
fn test_manifest_uri_rejects_file_javascript_data_and_non_http() {
    for bad in ["file:///etc/passwd", "javascript:alert(1)", "data:text/plain,x", "ftp://host/a"] {
        assert!(resolve_and_validate_media_uri(BASE, bad).is_err(), "must reject {}", bad);
    }
}

#[test]
fn test_manifest_uri_resolves_relative_segments() {
    let r = resolve_and_validate_media_uri(BASE, "seg001.ts").unwrap();
    assert_eq!(r, "https://cdn.example.invalid/hls/seg001.ts");
    let r2 = resolve_and_validate_media_uri(BASE, "/absolute/seg.ts").unwrap();
    assert_eq!(r2, "https://cdn.example.invalid/absolute/seg.ts");
}

// ---------- TEST-05B: HLS DRM classification and discontinuity ----------

#[test]
fn test_hls_aes128_is_standard_not_drm() {
    let m3u8 = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\nseg1.ts\n";
    let p = parse_hls_manifest(BASE, m3u8).unwrap();
    assert_eq!(p.drm, DrmStatus::StandardHlsAes128);
    assert_eq!(p.segments.len(), 1);
}

#[test]
fn test_hls_fairplay_and_widevine_keyformats_are_protected() {
    for key_line in [
        "#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://x\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\"\n",
        "#EXT-X-KEY:METHOD=SAMPLE-AES,KEYFORMAT=\"com.apple.streamingkeydelivery\"\n",
    ] {
        let m3u8 = format!("#EXTM3U\n{}seg.ts\n", key_line);
        let e = parse_hls_manifest(BASE, &m3u8).unwrap_err();
        assert_eq!(e.code, ErrorCode::DrmProtected);
    }
}

#[test]
fn test_hls_discontinuity_and_byterange_identity() {
    let m3u8 = "#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:5\nseg0.ts\n#EXT-X-DISCONTINUITY\n#EXT-X-BYTERANGE:1024@0\nseg1.ts\n";
    let p = parse_hls_manifest(BASE, m3u8).unwrap();
    assert_eq!(p.segments[0].sequence, 5);
    assert_eq!(p.segments[0].discontinuity_seq, 0);
    assert_eq!(p.segments[1].sequence, 6);
    assert_eq!(p.segments[1].discontinuity_seq, 1);
    assert_eq!(p.segments[1].byte_range, Some((0, 1024)));
}

// ---------- TEST-05B: DASH XXE + DRM ----------

#[test]
fn test_dash_rejects_xxe_and_content_protection() {
    let xxe = "<?xml version=\"1.0\"?><!DOCTYPE mpd [<!ENTITY xxe SYSTEM \"file:///etc/passwd\">]><MPD/>";
    assert_eq!(parse_dash_manifest("https://cdn.example.invalid/a.mpd", xxe).unwrap_err().code, ErrorCode::PermissionDenied);
    let drm = "<MPD><Period><ContentProtection cenc:pssh=\"AAA\"/></Period></MPD>";
    assert_eq!(parse_dash_manifest("https://cdn.example.invalid/a.mpd", drm).unwrap_err().code, ErrorCode::DrmProtected);
}

// ---------- TEST-06 / TEST-12A: subprocess argv hardening ----------

#[test]
fn test_ytdlp_command_uses_locked_paths_and_no_shell_injection() {
    let spec = build_ytdlp_command(
        Path::new("C:/Program Files/RustCatch/components/yt-dlp.exe"),
        Path::new("C:/Program Files/RustCatch/components/ffmpeg"),
        Path::new("C:/Program Files/RustCatch/components/deno.exe"),
        "http://127.0.0.1:19876",
        "bv*+ba/b",
        "%(id)s.%(ext)s",
        "https://www.youtube.com/watch?v=abc123",
        99, // clamp to <=8
    ).unwrap();
    assert!(spec.executable.ends_with("yt-dlp.exe"));
    assert!(spec.args.contains(&"--ignore-config".to_string()));
    assert!(spec.args.contains(&"--no-exec".to_string()));
    assert!(spec.args.contains(&"--keep-video".to_string()));
    // fragment concurrency clamped
    assert!(spec.args.contains(&"8".to_string()));
    assert!(!spec.args.contains(&"99".to_string()));
    // page URL passed as a discrete argv after "--", never shell-joined
    let dash = spec.args.iter().position(|a| a == "--").unwrap();
    assert_eq!(spec.args[dash + 1], "https://www.youtube.com/watch?v=abc123");
    // no shell metacharacters are introduced into a standalone "command" string
    assert!(!spec.args.iter().any(|a| a.contains("&&") || a.contains("; ")));
}

#[test]
fn test_ytdlp_command_rejects_unsafe_page_url() {
    for bad in ["-evil-flag", "file:///etc/passwd", "javascript:alert(1)", "not-a-url"] {
        let r = build_ytdlp_command(
            Path::new("yt-dlp.exe"), Path::new("ffmpeg"), Path::new("deno.exe"),
            "http://127.0.0.1:1", "bv", "%(id)s", bad, 4,
        );
        assert!(r.is_err(), "must reject unsafe URL {}", bad);
    }
}

// ---------- TEST-07A: YouTube identity normalization ----------

#[test]
fn test_youtube_watch_shorts_and_you_be_identity() {
    assert_eq!(extract_youtube_identity("https://www.youtube.com/watch?v=abc123"), Some(("youtube".into(), "abc123".into())));
    assert_eq!(extract_youtube_identity("https://www.youtube.com/shorts/xyz789"), Some(("youtube".into(), "xyz789".into())));
    assert_eq!(extract_youtube_identity("https://youtu.be/shortid"), Some(("youtube".into(), "shortid".into())));
    assert_eq!(extract_youtube_identity("https://example.invalid/watch?v=x"), None);
}
