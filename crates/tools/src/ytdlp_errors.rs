//! Ported from antonio-orionus/Arroxy `packages/ytdlp-errors` (MIT License)
//! Commit: `6df64a067f063c53a25c8deaca02e1345ffb1f87`

use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum YtDlpErrorCategory {
    Network,
    RateLimited,
    AuthRequired,
    AuthRejected,
    BotBlocked,
    IpBlocked,
    GeoBlocked,
    FormatUnavailable,
    ExtractorChanged,
    DrmProtected,
    DiskFull,
    MergeFailed,
}

impl YtDlpErrorCategory {
    pub fn to_error_code(self) -> ErrorCode {
        match self {
            Self::Network => ErrorCode::NetworkError,
            Self::RateLimited => ErrorCode::RateLimited,
            Self::AuthRequired => ErrorCode::AuthRequired,
            Self::AuthRejected => ErrorCode::AuthRejected,
            Self::BotBlocked => ErrorCode::BotBlocked,
            Self::IpBlocked => ErrorCode::IpBlocked,
            Self::GeoBlocked => ErrorCode::GeoBlocked,
            Self::FormatUnavailable => ErrorCode::FormatUnavailable,
            Self::ExtractorChanged => ErrorCode::ExtractorChanged,
            Self::DrmProtected => ErrorCode::DrmProtected,
            Self::DiskFull => ErrorCode::DiskFull,
            Self::MergeFailed => ErrorCode::MergeFailed,
        }
    }

    pub fn is_retryable(self) -> bool {
        matches!(self, Self::Network | Self::RateLimited)
    }
}

/// Classifies yt-dlp / FFmpeg stderr output into the 12 canonical error categories.
/// Never conflates all 403 responses into generic network failure.
pub fn classify_ytdlp_stderr(stderr: &str) -> (YtDlpErrorCategory, ErrorDetail) {
    let lower = stderr.to_ascii_lowercase();

    let category = if lower.contains("drm protected") || lower.contains("widevine") || lower.contains("playready") || lower.contains("fairplay") {
        YtDlpErrorCategory::DrmProtected
    } else if lower.contains("no space left on device") || lower.contains("enospc") || lower.contains("disk full") {
        YtDlpErrorCategory::DiskFull
    } else if lower.contains("sign in to confirm you’re not a bot") || lower.contains("confirm you're not a bot") || lower.contains("po token") || lower.contains("botguard") {
        YtDlpErrorCategory::BotBlocked
    } else if lower.contains("http error 429") || lower.contains("too many requests") || lower.contains("rate limit") {
        YtDlpErrorCategory::RateLimited
    } else if lower.contains("not available in your country") || lower.contains("geo-restricted") || lower.contains("uploader has not made this video available") {
        YtDlpErrorCategory::GeoBlocked
    } else if lower.contains("your ip is being blocked") || lower.contains("ip address has been blocked") {
        YtDlpErrorCategory::IpBlocked
    } else if lower.contains("cookies are no longer valid") || lower.contains("http error 401") || lower.contains("session expired") {
        YtDlpErrorCategory::AuthRejected
    } else if lower.contains("private video") || lower.contains("members-only") || lower.contains("sign in to view") || lower.contains("age-restricted") {
        YtDlpErrorCategory::AuthRequired
    } else if lower.contains("requested format is not available") || lower.contains("format not found") {
        YtDlpErrorCategory::FormatUnavailable
    } else if lower.contains("unable to extract") || lower.contains("nsig extraction failed") || lower.contains("signature extraction failed") {
        YtDlpErrorCategory::ExtractorChanged
    } else if lower.contains("postprocessing:") || lower.contains("ffmpeg exited with") || lower.contains("conversion failed") || lower.contains("error muxing") {
        YtDlpErrorCategory::MergeFailed
    } else {
        YtDlpErrorCategory::Network
    };

    let retry_after_ms = if category == YtDlpErrorCategory::RateLimited {
        Some(60_000)
    } else if category == YtDlpErrorCategory::Network {
        Some(30_000)
    } else {
        None
    };

    let detail = ErrorDetail {
        code: category.to_error_code(),
        message: format!("Media pipeline classified error as {:?}", category),
        retryable: category.is_retryable(),
        retry_after_ms,
    };
    (category, detail)
}
