//! P5-T02 (REQ-16): Periodic Synchronization Queue with validator comparison,
//! staging verification, local conflict backup, and non-destructive remote deletion handling.

use mdm_engine::SourceValidator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncDecision {
    UnchangedSkip,
    StageAndReplaceWithBackup { backup_path: String },
    ConflictPreserveLocalAndBackup { conflict_copy_path: String },
    RemoteDeletedKeepLocal,
    RequireExplicitValidatorOrFullCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncItemState {
    pub task_id: String,
    pub target_path: String,
    pub last_synced_validator: SourceValidator,
    pub last_synced_sha256: String,
}

/// Evaluates periodic synchronization action for a tracked resource.
/// Never deletes the local file when remote returns 404/410, and never overwrites a locally modified file without creating a backup.
pub fn evaluate_periodic_sync(
    item: &SyncItemState,
    remote_status: u16,
    remote_validator: &SourceValidator,
    current_local_sha256: &str,
) -> SyncDecision {
    if remote_status == 404 || remote_status == 410 {
        return SyncDecision::RemoteDeletedKeepLocal;
    }
    if !remote_validator.can_resume_reliably() {
        return SyncDecision::RequireExplicitValidatorOrFullCheck;
    }
    if item.last_synced_validator.matches(remote_validator) {
        return SyncDecision::UnchangedSkip;
    }
    let local_modified_by_user = current_local_sha256 != item.last_synced_sha256;
    if local_modified_by_user {
        SyncDecision::ConflictPreserveLocalAndBackup {
            conflict_copy_path: format!("{}.conflict.bak", item.target_path),
        }
    } else {
        SyncDecision::StageAndReplaceWithBackup {
            backup_path: format!("{}.prev.bak", item.target_path),
        }
    }
}
