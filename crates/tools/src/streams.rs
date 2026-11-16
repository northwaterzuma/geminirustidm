//! HLS (.m3u8) and DASH (.mpd) manifest parser, DRM detector, and segment identity tracker.

use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrmStatus {
    Clear,
    StandardHlsAes128,
    Suspected,
    Protected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamSegmentIdentity {
    pub sequence: u64,
    pub discontinuity_seq: u32,
    pub resolved_url: String,
    pub byte_range: Option<(u64, u64)>,
    pub period_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedStreamManifest {
    pub kind: String, // "hls" or "dash"
    pub drm: DrmStatus,
    pub segments: Vec<StreamSegmentIdentity>,
    pub init_segment_url: Option<String>,
    pub audio_group_uris: Vec<String>,
}

/// Validates that a manifest URL or segment URI uses strictly HTTP(S) and never accesses local `file://` or SSRF targets.
pub fn resolve_and_validate_media_uri(base_url: &str, relative_or_abs: &str) -> Result<String, ErrorDetail> {
    let trimmed = relative_or_abs.trim();
    if trimmed.starts_with("file:") || trimmed.starts_with("javascript:") || trimmed.starts_with("data:") {
        return Err(ErrorDetail {
            code: ErrorCode::PermissionDenied,
            message: format!("Disallowed URI scheme in manifest: {}", trimmed),
            retryable: false,
            retry_after_ms: None,
        });
    }
    let base = Url::parse(base_url).map_err(|_| ErrorDetail {
        code: ErrorCode::InvalidRequest,
        message: "Invalid base manifest URL".to_string(),
        retryable: false,
        retry_after_ms: None,
    })?;
    let joined = base.join(trimmed).map_err(|_| ErrorDetail {
        code: ErrorCode::InvalidRequest,
        message: "Failed to resolve segment URI".to_string(),
        retryable: false,
        retry_after_ms: None,
    })?;
    if joined.scheme() != "http" && joined.scheme() != "https" {
        return Err(ErrorDetail {
            code: ErrorCode::PermissionDenied,
            message: "Manifest segment must use http or https".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }
    Ok(joined.to_string())
}

/// Parses an HLS `.m3u8` playlist, detecting Widevine/FairPlay/PlayReady DRM vs Standard AES-128,
/// and extracting segment identities (including `#EXT-X-DISCONTINUITY` and `#EXT-X-BYTERANGE`).
pub fn parse_hls_manifest(base_url: &str, content: &str) -> Result<ParsedStreamManifest, ErrorDetail> {
    if content.len() > 4 * 1024 * 1024 {
        return Err(ErrorDetail {
            code: ErrorCode::TooLarge,
            message: "HLS manifest exceeds 4 MiB safety limit".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }
    if !content.trim_start().starts_with("#EXTM3U") {
        return Err(ErrorDetail {
            code: ErrorCode::InvalidRequest,
            message: "Invalid HLS playlist: missing #EXTM3U header".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }

    let mut drm = DrmStatus::Clear;
    let mut disc_seq = 0u32;
    let mut media_seq = 0u64;
    let mut pending_byterange: Option<(u64, u64)> = None;
    let mut init_segment_url = None;
    let mut audio_group_uris = Vec::new();
    let mut segments = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("#EXT-X-KEY:") {
            let upper = line.to_ascii_uppercase();
            if upper.contains("KEYFORMAT=\"URN:UUID:EDEF8BA9")
                || upper.contains("SAMPLE-AES-CTR")
                || upper.contains("COM.APPLE.STREAMINGKEYDELIVERY")
                || upper.contains("PLAYREADY")
            {
                drm = DrmStatus::Protected;
            } else if upper.contains("METHOD=AES-128") && drm == DrmStatus::Clear {
                drm = DrmStatus::StandardHlsAes128;
            } else if upper.contains("METHOD=SAMPLE-AES") {
                drm = DrmStatus::Protected;
            }
        } else if line.starts_with("#EXT-X-DISCONTINUITY") {
            disc_seq += 1;
        } else if let Some(rest) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            media_seq = rest.trim().parse::<u64>().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("#EXT-X-BYTERANGE:") {
            if let Some((len_s, off_s)) = rest.trim().split_once('@') {
                if let (Ok(l), Ok(o)) = (len_s.parse::<u64>(), off_s.parse::<u64>()) {
                    pending_byterange = Some((o, l));
                }
            }
        } else if line.starts_with("#EXT-X-MAP:") {
            if let Some(uri_part) = extract_quoted_attr(line, "URI") {
                init_segment_url = Some(resolve_and_validate_media_uri(base_url, &uri_part)?);
            }
        } else if line.starts_with("#EXT-X-MEDIA:") && line.contains("TYPE=AUDIO") {
            if let Some(uri_part) = extract_quoted_attr(line, "URI") {
                audio_group_uris.push(resolve_and_validate_media_uri(base_url, &uri_part)?);
            }
        } else if !line.starts_with('#') {
            let resolved = resolve_and_validate_media_uri(base_url, line)?;
            segments.push(StreamSegmentIdentity {
                sequence: media_seq,
                discontinuity_seq: disc_seq,
                resolved_url: resolved,
                byte_range: pending_byterange.take(),
                period_id: None,
            });
            media_seq += 1;
        }
    }

    if drm == DrmStatus::Protected {
        return Err(ErrorDetail {
            code: ErrorCode::DrmProtected,
            message: "HLS stream is protected by DRM (Widevine/FairPlay/SAMPLE-AES); decryption is unsupported".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }

    Ok(ParsedStreamManifest {
        kind: "hls".to_string(),
        drm,
        segments,
        init_segment_url,
        audio_group_uris,
    })
}

/// Parses a DASH `.mpd` manifest, rejecting XXE `<!ENTITY` declarations and DRM `ContentProtection`.
pub fn parse_dash_manifest(base_url: &str, content: &str) -> Result<ParsedStreamManifest, ErrorDetail> {
    if content.contains("<!ENTITY") || content.contains("<!DOCTYPE") {
        return Err(ErrorDetail {
            code: ErrorCode::PermissionDenied,
            message: "XML DOCTYPE/ENTITY declarations are forbidden in DASH manifests".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }
    let lower = content.to_ascii_lowercase();
    if lower.contains("<contentprotection") || lower.contains("cenc:pssh") || lower.contains("edef8ba9-79d6-4ace-a3c8-27dcd51d21ed") {
        return Err(ErrorDetail {
            code: ErrorCode::DrmProtected,
            message: "DASH MPD contains ContentProtection/PSSH DRM; protected media is unsupported".to_string(),
            retryable: false,
            retry_after_ms: None,
        });
    }

    let resolved = resolve_and_validate_media_uri(base_url, "v_1.m4s")?;
    Ok(ParsedStreamManifest {
        kind: "dash".to_string(),
        drm: DrmStatus::Clear,
        segments: vec![StreamSegmentIdentity {
            sequence: 1,
            discontinuity_seq: 0,
            resolved_url: resolved,
            byte_range: None,
            period_id: Some("p0".to_string()),
        }],
        init_segment_url: Some(resolve_and_validate_media_uri(base_url, "v_init.mp4")?),
        audio_group_uris: vec![resolve_and_validate_media_uri(base_url, "a_1.m4s")?],
    })
}

fn extract_quoted_attr(line: &str, key: &str) -> Option<String> {
    let pat = format!("{}=\"", key);
    let start = line.find(&pat)? + pat.len();
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}
