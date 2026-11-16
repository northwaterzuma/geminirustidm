use crate::http_validator::SourceValidator;
use crate::planner::ByteInterval;
use mdm_ipc::{ErrorCode, ErrorDetail, CURRENT_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCheckpoint {
    pub schema_version: u32,
    pub task_id: String,
    pub checkpoint_generation: u64,
    pub source_validator: SourceValidator,
    pub total_bytes: Option<u64>,
    pub part_path: String,
    pub final_path: String,
    pub synced_intervals: Vec<ByteInterval>,
    pub expected_sha256: Option<String>,
}

impl TransferCheckpoint {
    /// Atomically persists the checkpoint only after syncing the part file, temporary state file,
    /// atomic rename, and parent directory metadata.
    pub fn save_durable(&self, state_file_path: &Path) -> Result<(), ErrorDetail> {
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(ErrorDetail {
                code: ErrorCode::UnsupportedVersion,
                message: format!(
                    "Refusing to overwrite higher schema_version {} with current {}",
                    self.schema_version, CURRENT_SCHEMA_VERSION
                ),
                retryable: false,
                retry_after_ms: None,
            });
        }

        // 1. Sync the part file if it exists on disk
        let part = Path::new(&self.part_path);
        if part.exists() {
            let pf = OpenOptions::new().read(true).write(true).open(part).map_err(|e| ErrorDetail {
                code: ErrorCode::InternalError,
                message: format!("Failed to open part file for sync: {}", e),
                retryable: false,
                retry_after_ms: None,
            })?;
            pf.sync_all().map_err(|e| ErrorDetail {
                code: ErrorCode::InternalError,
                message: format!("Failed to fsync part file: {}", e),
                retryable: false,
                retry_after_ms: None,
            })?;
        }

        // 2. Write and sync temporary state file in the same directory
        let tmp_path = state_file_path.with_extension("state.tmp");
        let serialized = serde_json::to_vec_pretty(self).map_err(|e| ErrorDetail {
            code: ErrorCode::InternalError,
            message: format!("Checkpoint serialization error: {}", e),
            retryable: false,
            retry_after_ms: None,
        })?;
        {
            let mut tf = File::create(&tmp_path).map_err(|e| ErrorDetail {
                code: ErrorCode::DiskFull,
                message: format!("Failed to create temp checkpoint file: {}", e),
                retryable: false,
                retry_after_ms: None,
            })?;
            tf.write_all(&serialized).map_err(|e| ErrorDetail {
                code: ErrorCode::DiskFull,
                message: format!("Failed to write temp checkpoint file: {}", e),
                retryable: false,
                retry_after_ms: None,
            })?;
            tf.sync_all().map_err(|e| ErrorDetail {
                code: ErrorCode::InternalError,
                message: format!("Failed to fsync temp checkpoint file: {}", e),
                retryable: false,
                retry_after_ms: None,
            })?;
        }

        // 3. Atomic replace
        fs::rename(&tmp_path, state_file_path).map_err(|e| ErrorDetail {
            code: ErrorCode::InternalError,
            message: format!("Failed to atomically replace checkpoint file: {}", e),
            retryable: false,
            retry_after_ms: None,
        })?;

        // 4. Sync parent directory metadata on Unix/supported platforms
        if let Some(parent) = state_file_path.parent() {
            if let Ok(dir_file) = File::open(parent) {
                let _ = dir_file.sync_all();
            }
        }
        Ok(())
    }

    pub fn load_checked(state_file_path: &Path) -> Result<Self, ErrorDetail> {
        let bytes = fs::read(state_file_path).map_err(|e| ErrorDetail {
            code: ErrorCode::NotFound,
            message: format!("Failed to read checkpoint: {}", e),
            retryable: false,
            retry_after_ms: None,
        })?;
        let ckpt: Self = serde_json::from_slice(&bytes).map_err(|e| ErrorDetail {
            code: ErrorCode::InvalidRequest,
            message: format!("Corrupt checkpoint JSON: {}", e),
            retryable: false,
            retry_after_ms: None,
        })?;
        if ckpt.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(ErrorDetail {
                code: ErrorCode::UnsupportedVersion,
                message: format!(
                    "Checkpoint schema_version {} is newer than supported {}; refusing destructive write",
                    ckpt.schema_version, CURRENT_SCHEMA_VERSION
                ),
                retryable: false,
                retry_after_ms: None,
            });
        }
        Ok(ckpt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrashReconcileOutcome {
    /// Final file was already committed and verified before crash; mark Completed without re-downloading.
    PromoteToCompleted { verified_sha256: String },
    /// Resume from verified `.part` file intervals.
    ResumeFromPart { valid_bytes: u64 },
    /// Unowned file exists at final_path; do NOT delete it and do NOT mark completed; require rename or user action.
    ForeignFileCollision { final_path: PathBuf },
}

/// Reconciles crash window between final file rename and task completion record persistence.
pub fn reconcile_startup_checkpoint(ckpt: &TransferCheckpoint) -> Result<CrashReconcileOutcome, ErrorDetail> {
    let part = Path::new(&ckpt.part_path);
    let final_p = Path::new(&ckpt.final_path);

    if !part.exists() && final_p.exists() {
        let meta = fs::metadata(final_p).map_err(|e| ErrorDetail {
            code: ErrorCode::InternalError,
            message: e.to_string(),
            retryable: false,
            retry_after_ms: None,
        })?;
        if let Some(expected_len) = ckpt.total_bytes {
            let synced_total: u64 = ckpt.synced_intervals.iter().map(|i| i.len()).sum();
            if meta.len() == expected_len && synced_total == expected_len {
                let data = fs::read(final_p).unwrap_or_default();
                let actual_sha = hex::encode(Sha256::digest(&data));
                if let Some(exp_sha) = &ckpt.expected_sha256 {
                    if &actual_sha != exp_sha {
                        return Ok(CrashReconcileOutcome::ForeignFileCollision {
                            final_path: final_p.to_path_buf(),
                        });
                    }
                }
                return Ok(CrashReconcileOutcome::PromoteToCompleted {
                    verified_sha256: actual_sha,
                });
            }
        }
        return Ok(CrashReconcileOutcome::ForeignFileCollision {
            final_path: final_p.to_path_buf(),
        });
    }

    let valid_bytes: u64 = ckpt.synced_intervals.iter().map(|i| i.len()).sum();
    Ok(CrashReconcileOutcome::ResumeFromPart { valid_bytes })
}
