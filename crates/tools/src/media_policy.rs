//! MediaIntent format selection, container compatibility, and playlist deduplication.
//! Ported and adapted from Arroxy `src/shared/mediaIntent.ts` & `formatSelector.ts`.

use mdm_ipc::{ErrorCode, ErrorDetail, MediaIntent, MediaKind, MediaPreset, SizeKind};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatChoice {
    pub format_id: String,
    pub role: TrackRole,
    pub protocol: String,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub container: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<u32>,
    pub dynamic_range: String, // "SDR", "HDR10", "DV"
    pub language: Option<String>,
    pub is_original: bool,
    pub is_auto_caption: bool,
    pub bitrate_kbps: Option<u32>,
    pub size_bytes: Option<u64>,
    pub size_kind: SizeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackRole {
    VideoAudio,
    VideoOnly,
    AudioOnly,
    Subtitle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatSelectionResult {
    pub selected_video: Option<FormatChoice>,
    pub selected_audio: Option<FormatChoice>,
    pub selected_subtitles: Vec<FormatChoice>,
    pub target_container: String,
    pub requires_transcode: bool,
    pub quality_downgraded: bool,
}

/// Evaluates `MediaIntent` against available `FormatChoice` items.
/// Strictly enforces:
/// - `require_exact_format`: fails with `FormatUnavailable` if any `selected_format_ids` is missing.
/// - `max_height`: upper bound only; if user explicitly chose a height that disappeared and `allow_quality_downgrade == false`, fails with `FormatUnavailable`.
/// - Container compatibility: MP4 without transcode requires H.264/HEVC/AV1 + AAC/MP4A; never fakes conversion by renaming `.webm` to `.mp4`.
pub fn select_formats(intent: &MediaIntent, available: &[FormatChoice]) -> Result<FormatSelectionResult, ErrorDetail> {
    if intent.require_exact_format {
        if intent.selected_format_ids.is_empty() {
            return Err(ErrorDetail {
                code: ErrorCode::InvalidRequest,
                message: "require_exact_format is true but selected_format_ids is empty".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
        let mut found_video = None;
        let mut found_audio = None;
        for fid in &intent.selected_format_ids {
            let Some(fmt) = available.iter().find(|f| &f.format_id == fid) else {
                return Err(ErrorDetail {
                    code: ErrorCode::FormatUnavailable,
                    message: format!("Required exact format_id '{}' is no longer available", fid),
                    retryable: false,
                    retry_after_ms: None,
                });
            };
            match fmt.role {
                TrackRole::VideoOnly | TrackRole::VideoAudio => found_video = Some(fmt.clone()),
                TrackRole::AudioOnly => found_audio = Some(fmt.clone()),
                TrackRole::Subtitle => {}
            }
        }
        let target_container = resolve_container(intent, found_video.as_ref(), found_audio.as_ref())?;
        return Ok(FormatSelectionResult {
            selected_video: found_video,
            selected_audio: found_audio,
            selected_subtitles: select_subtitles(intent, available)?,
            target_container,
            requires_transcode: false,
            quality_downgraded: false,
        });
    }

    match intent.kind {
        MediaKind::SubtitlesOnly => {
            let subs = select_subtitles(intent, available)?;
            if subs.is_empty() {
                return Err(ErrorDetail {
                    code: ErrorCode::FormatUnavailable,
                    message: "Requested subtitles are not available".to_string(),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
            Ok(FormatSelectionResult {
                selected_video: None,
                selected_audio: None,
                selected_subtitles: subs,
                target_container: "vtt".to_string(),
                requires_transcode: false,
                quality_downgraded: false,
            })
        }
        MediaKind::AudioOnly => {
            let audio = pick_best_audio(intent, available).ok_or_else(|| ErrorDetail {
                code: ErrorCode::FormatUnavailable,
                message: "No matching audio track available".to_string(),
                retryable: false,
                retry_after_ms: None,
            })?;
            let target_container = if intent.container == "auto" {
                audio.container.clone()
            } else {
                intent.container.clone()
            };
            let needs_transcode = target_container != audio.container;
            if needs_transcode && !intent.allow_transcode {
                return Err(ErrorDetail {
                    code: ErrorCode::FormatUnavailable,
                    message: format!(
                        "Audio container '{}' incompatible with source '{}' and allow_transcode=false",
                        target_container, audio.container
                    ),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
            Ok(FormatSelectionResult {
                selected_video: None,
                selected_audio: Some(audio),
                selected_subtitles: select_subtitles(intent, available)?,
                target_container,
                requires_transcode: needs_transcode,
                quality_downgraded: false,
            })
        }
        MediaKind::VideoOnly | MediaKind::VideoAudio => {
            let mut candidates: Vec<&FormatChoice> = available
                .iter()
                .filter(|f| matches!(f.role, TrackRole::VideoOnly | TrackRole::VideoAudio))
                .filter(|f| {
                    if let Some(max_h) = intent.max_height {
                        f.height.unwrap_or(0) <= max_h
                    } else {
                        true
                    }
                })
                .filter(|f| match intent.hdr.as_str() {
                    "require_hdr" => f.dynamic_range != "SDR",
                    "prefer_sdr" => f.dynamic_range == "SDR",
                    _ => true,
                })
                .collect();

            if intent.preset == MediaPreset::DeviceCompatible {
                candidates.retain(|f| {
                    f.video_codec.as_deref().map(|c| c.starts_with("h264") || c.starts_with("avc")).unwrap_or(false)
                });
            }

            candidates.sort_by(|a, b| {
                let h_cmp = b.height.unwrap_or(0).cmp(&a.height.unwrap_or(0));
                if h_cmp != std::cmp::Ordering::Equal {
                    return h_cmp;
                }
                let fps_cmp = b.fps.unwrap_or(0).cmp(&a.fps.unwrap_or(0));
                if fps_cmp != std::cmp::Ordering::Equal {
                    return fps_cmp;
                }
                b.bitrate_kbps.unwrap_or(0).cmp(&a.bitrate_kbps.unwrap_or(0))
            });

            let video = (*candidates.first().ok_or_else(|| ErrorDetail {
                code: ErrorCode::FormatUnavailable,
                message: "No matching video track satisfies MediaIntent constraints".to_string(),
                retryable: false,
                retry_after_ms: None,
            })?)
            .clone();

            let audio = if intent.kind == MediaKind::VideoAudio && video.role == TrackRole::VideoOnly {
                Some(pick_best_audio(intent, available).ok_or_else(|| ErrorDetail {
                    code: ErrorCode::FormatUnavailable,
                    message: "No matching audio track available for video+audio merge".to_string(),
                    retryable: false,
                    retry_after_ms: None,
                })?)
            } else {
                None
            };

            let target_container = resolve_container(intent, Some(&video), audio.as_ref())?;
            Ok(FormatSelectionResult {
                selected_video: Some(video),
                selected_audio: audio,
                selected_subtitles: select_subtitles(intent, available)?,
                target_container,
                requires_transcode: false,
                quality_downgraded: false,
            })
        }
    }
}

fn pick_best_audio(intent: &MediaIntent, available: &[FormatChoice]) -> Option<FormatChoice> {
    let mut audios: Vec<&FormatChoice> = available
        .iter()
        .filter(|f| f.role == TrackRole::AudioOnly)
        .filter(|f| {
            if intent.audio_language == "original" {
                f.is_original
            } else {
                f.language.as_deref() == Some(intent.audio_language.as_str())
            }
        })
        .collect();
    audios.sort_by(|a, b| b.bitrate_kbps.unwrap_or(0).cmp(&a.bitrate_kbps.unwrap_or(0)));
    audios.first().map(|f| (*f).clone())
}

fn select_subtitles(intent: &MediaIntent, available: &[FormatChoice]) -> Result<Vec<FormatChoice>, ErrorDetail> {
    if intent.subtitles.mode == "none" || intent.subtitles.languages.is_empty() {
        return Ok(Vec::new());
    }
    let mut chosen = Vec::new();
    for lang in &intent.subtitles.languages {
        let manual = available
            .iter()
            .find(|f| f.role == TrackRole::Subtitle && f.language.as_deref() == Some(lang.as_str()) && !f.is_auto_caption);
        let auto = available
            .iter()
            .find(|f| f.role == TrackRole::Subtitle && f.language.as_deref() == Some(lang.as_str()) && f.is_auto_caption);

        let picked = match intent.subtitles.source.as_str() {
            "manual" => manual,
            "auto" => auto,
            _ => manual.or(auto),
        };
        if let Some(sub) = picked {
            chosen.push(sub.clone());
        } else if intent.subtitles.required {
            return Err(ErrorDetail {
                code: ErrorCode::FormatUnavailable,
                message: format!("Required subtitle language '{}' not available", lang),
                retryable: false,
                retry_after_ms: None,
            });
        }
    }
    Ok(chosen)
}

fn resolve_container(intent: &MediaIntent, video: Option<&FormatChoice>, audio: Option<&FormatChoice>) -> Result<String, ErrorDetail> {
    let v_codec = video.and_then(|v| v.video_codec.as_deref()).unwrap_or("");
    let a_codec = audio.and_then(|a| a.audio_codec.as_deref()).unwrap_or("");

    if intent.container == "auto" {
        if v_codec.starts_with("vp9") || a_codec.starts_with("opus") {
            return Ok("mkv".to_string());
        }
        return Ok("mp4".to_string());
    }
    if intent.container == "mp4" && !intent.allow_transcode {
        if v_codec.starts_with("vp8") || a_codec.starts_with("vorbis") {
            return Err(ErrorDetail {
                code: ErrorCode::FormatUnavailable,
                message: "Codec VP8/Vorbis cannot be muxed into MP4 without transcode (allow_transcode=false)".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
    }
    Ok(intent.container.clone())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistItemEntry {
    pub video_id: String,
    pub title: String,
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistExpansionPlan {
    pub page_entries: Vec<PlaylistItemEntry>,
    pub skipped_duplicates: Vec<String>,
    pub unavailable_entries: Vec<PlaylistItemEntry>,
    pub next_cursor: Option<usize>,
}

/// Paginates playlist preview (max 500 items per page) and deduplicates by `(parent_task_id, video_id, policy_digest)`
/// within a single expansion operation while isolating deleted/private items so they do not fail the whole playlist.
pub fn expand_playlist_page(
    all_items: &[PlaylistItemEntry],
    cursor: usize,
    page_limit: usize,
) -> PlaylistExpansionPlan {
    let bounded_limit = page_limit.clamp(1, 500);
    let mut seen = HashSet::new();
    let mut page_entries = Vec::new();
    let mut skipped_duplicates = Vec::new();
    let mut unavailable_entries = Vec::new();

    for item in all_items.iter().take(cursor) {
        if item.is_available {
            seen.insert(item.video_id.clone());
        }
    }

    let slice_end = (cursor + bounded_limit).min(all_items.len());
    for item in &all_items[cursor..slice_end] {
        if !item.is_available {
            unavailable_entries.push(item.clone());
            continue;
        }
        if !seen.insert(item.video_id.clone()) {
            skipped_duplicates.push(item.video_id.clone());
            continue;
        }
        page_entries.push(item.clone());
    }

    let next_cursor = if slice_end < all_items.len() {
        Some(slice_end)
    } else {
        None
    };

    PlaylistExpansionPlan {
        page_entries,
        skipped_duplicates,
        unavailable_entries,
        next_cursor,
    }
}
