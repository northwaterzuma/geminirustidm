//! Controlled ComponentSet manager for yt-dlp, yt-dlp-ejs, Deno, FFmpeg, and ffprobe.

use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub id: String,
    pub role: String,
    pub version: Option<String>,
    pub sha256: Option<String>,
    pub relative_executable: Option<String>,
    pub compatibility_verified: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentLockManifest {
    pub schema_version: u32,
    pub project: String,
    pub component_set_id: Option<String>,
    pub status: String,
    pub platform_triple: String,
    pub components: Vec<ComponentEntry>,
}

impl ComponentLockManifest {
    /// Verifies that all required media components have non-null version and SHA-256 and match on-disk digests.
    /// Never falls back to system PATH and never executes unverified or null-hash binaries.
    pub fn verify_ready_for_execution(&self, install_root: &Path) -> Result<String, ErrorDetail> {
        if self.status == "unresolved_do_not_execute" {
            return Err(ErrorDetail {
                code: ErrorCode::ToolUnavailable,
                message: "COMPONENT_LOCK status is unresolved_do_not_execute; refusing to execute unverified external tools".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
        let set_id = self.component_set_id.as_ref().ok_or_else(|| ErrorDetail {
            code: ErrorCode::ToolUnavailable,
            message: "Missing component_set_id in COMPONENT_LOCK".to_string(),
            retryable: false,
            retry_after_ms: None,
        })?;

        for comp in &self.components {
            let (Some(ver), Some(exp_sha), Some(rel_path)) = (&comp.version, &comp.sha256, &comp.relative_executable) else {
                return Err(ErrorDetail {
                    code: ErrorCode::ToolUnavailable,
                    message: format!("Component '{}' is missing version, sha256, or executable path", comp.id),
                    retryable: false,
                    retry_after_ms: None,
                });
            };
            if ver.is_empty() || exp_sha.len() != 64 {
                return Err(ErrorDetail {
                    code: ErrorCode::ToolUnavailable,
                    message: format!("Component '{}' has invalid version or SHA-256 digest", comp.id),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
            if rel_path.contains("..") {
                return Err(ErrorDetail {
                    code: ErrorCode::PermissionDenied,
                    message: format!("Path traversal detected in component relative path: {}", rel_path),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
            let bin_path = install_root.join(rel_path);
            let bytes = fs::read(&bin_path).map_err(|_| ErrorDetail {
                code: ErrorCode::ToolUnavailable,
                message: format!("Component binary not found at locked path: {}", bin_path.display()),
                retryable: false,
                retry_after_ms: None,
            })?;
            let actual_sha = hex::encode(Sha256::digest(&bytes));
            if &actual_sha != exp_sha {
                return Err(ErrorDetail {
                    code: ErrorCode::ToolUnavailable,
                    message: format!("SHA-256 mismatch for component '{}': expected {}, got {}", comp.id, exp_sha, actual_sha),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
        }
        Ok(set_id.clone())
    }
}
