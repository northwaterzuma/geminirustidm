use crate::media_policy::{FormatChoice, FormatSelectionResult};
use crate::streams::DrmStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetIdentity {
    pub extractor: String,
    pub platform_media_id: String,
    pub canonical_page_url: String,
    pub track_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaCheckpoint {
    pub schema_version: u32,
    pub task_id: String,
    pub plan_revision: u64,
    pub asset: AssetIdentity,
    pub component_set_id: String,
    pub selected: FormatSelectionResult,
    pub drm: DrmStatus,
    pub video_track_completed: bool,
    pub audio_track_completed: bool,
    pub subtitles_completed: bool,
    pub merge_completed: bool,
    pub temporary_paths: Vec<String>,
}

impl MediaCheckpoint {
    /// Checks whether a refreshed media extraction matches the existing checkpoint's asset and track fingerprint.
    /// Never splices a different stream merely because the video title or format_id looks similar.
    pub fn can_reuse_completed_tracks(&self, refreshed_asset: &AssetIdentity, new_selection: &FormatSelectionResult) -> bool {
        self.asset.extractor == refreshed_asset.extractor
            && self.asset.platform_media_id == refreshed_asset.platform_media_id
            && self.asset.track_fingerprint == refreshed_asset.track_fingerprint
            && self.selected.selected_video == new_selection.selected_video
            && self.selected.selected_audio == new_selection.selected_audio
    }
}

/// Filters out YouTube playback range chunks (`googlevideo.com/videoplayback`) from creating standalone capture tasks,
/// requiring full canonical watch/shorts URL resolution instead.
pub fn is_ephemeral_playback_chunk(url: &str) -> bool {
    url.contains("googlevideo.com/videoplayback") && (url.contains("&range=") || url.contains("?range=") || url.contains("&rn="))
}

/// Recovers a clean media page URL from a yt-dlp "Following redirect to" line
/// that landed on a Weibo visitor/passport gate. The gate URL carries the real
/// target in its `url=` query parameter, which yt-dlp's generic extractor cannot
/// handle but the dedicated WeiboVideo extractor can.
///
/// Never follows network redirects itself: it only parses text yt-dlp already saw.
pub fn extract_weibo_visitor_target(line: &str) -> Option<String> {
    const MARKER: &str = "Following redirect to ";
    let redirected = line.split_once(MARKER)?.1.trim();
    let parsed = url::Url::parse(redirected).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let is_gate = host == "passport.weibo.com"
        || host == "weibo.com" && parsed.path().starts_with("/visitor/");
    if !is_gate {
        return None;
    }
    let target = parsed
        .query_pairs()
        .find(|(key, _)| key == "url")?
        .1
        .into_owned();
    let target_url = url::Url::parse(&target).ok()?;
    let target_host = target_url.host_str()?.to_ascii_lowercase();
    if !(target_host == "weibo.com" || target_host.ends_with(".weibo.com")) {
        return None;
    }
    Some(target)
}

/// Normalizes YouTube Watch and Shorts URLs into `(extractor, platform_media_id)`.
pub fn extract_youtube_identity(page_url: &str) -> Option<(String, String)> {
    let parsed = url::Url::parse(page_url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    if host == "youtu.be" {
        let id = parsed.path().trim_matches('/');
        if !id.is_empty() {
            return Some(("youtube".to_string(), id.to_string()));
        }
    }
    if host.ends_with("youtube.com") {
        if parsed.path() == "/watch" {
            for (k, v) in parsed.query_pairs() {
                if k == "v" && !v.is_empty() {
                    return Some(("youtube".to_string(), v.into_owned()));
                }
            }
        } else if let Some(rest) = parsed.path().strip_prefix("/shorts/") {
            let id = rest.split('/').next().unwrap_or("");
            if !id.is_empty() {
                return Some(("youtube".to_string(), id.to_string()));
            }
        }
    }
    None
}

pub fn compute_track_fingerprint(video: Option<&FormatChoice>, audio: Option<&FormatChoice>) -> String {
    let v_str = video
        .map(|v| format!("{}:{}:{:?}:{:?}", v.format_id, v.container, v.video_codec, v.size_bytes))
        .unwrap_or_else(|| "none".to_string());
    let a_str = audio
        .map(|a| format!("{}:{}:{:?}:{:?}", a.format_id, a.container, a.audio_codec, a.size_bytes))
        .unwrap_or_else(|| "none".to_string());
    format!("{}|{}", v_str, a_str)
}
