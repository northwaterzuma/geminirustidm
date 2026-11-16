//! Local loopback proxy relay that binds external tools (yt-dlp) to Rust's SharedRateLimiter,
//! active NetworkProfile, and ContextHandle revocation lifecycle.

use mdm_engine::SharedRateLimiter;
use mdm_ipc::{ErrorCode, ErrorDetail, NetworkMode, NetworkProfile};

#[derive(Debug, Clone)]
pub struct LocalRelaySession {
    pub attempt_id: String,
    pub relay_token: String,
    pub network_profile: NetworkProfile,
    pub context_revoked: bool,
    pub limiter: SharedRateLimiter,
}

impl LocalRelaySession {
    pub fn validate_forwarding(&self, provided_token: &str, upstream_reachable: bool) -> Result<(), ErrorDetail> {
        if self.context_revoked {
            return Err(ErrorDetail {
                code: ErrorCode::ContextExpired,
                message: "Auth context has been revoked or expired; terminating relay connection".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
        if provided_token != self.relay_token {
            return Err(ErrorDetail {
                code: ErrorCode::PermissionDenied,
                message: "Invalid local relay token".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
        if self.network_profile.mode == NetworkMode::Manual && !upstream_reachable {
            // Never silently fall back to direct connection when configured proxy is unreachable
            return Err(ErrorDetail {
                code: ErrorCode::NetworkError,
                message: "Configured manual proxy is unreachable and direct_fallback is strictly disabled".to_string(),
                retryable: true,
                retry_after_ms: Some(15_000),
            });
        }
        Ok(())
    }
}

/// Maps YouTube and all `*.googlevideo.com` media CDN hosts to the shared `"youtube"` site budget key.
pub fn canonical_site_budget_key(host: &str) -> String {
    let lower = host.to_ascii_lowercase();
    if lower == "youtu.be"
        || lower.ends_with("youtube.com")
        || lower.ends_with("googlevideo.com")
        || lower.ends_with("ytimg.com")
    {
        "youtube".to_string()
    } else {
        lower
    }
}
