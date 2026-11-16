use crate::checkpoint::TransferCheckpoint;
use crate::http_validator::SourceValidator;
use crate::planner::SegmentPlanner;
use mdm_ipc::{ErrorCode, ErrorDetail};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct DownloadJobState {
    pub task_id: String,
    pub validator: SourceValidator,
    pub planner: Option<SegmentPlanner>,
    pub single_stream_bytes: u64,
    pub expected_sha256: Option<String>,
}

impl DownloadJobState {
    pub fn new(task_id: String, validator: SourceValidator, max_conns: u8, expected_sha256: Option<String>) -> Self {
        let planner = if validator.can_resume_reliably() {
            validator.total_bytes.map(|tot| SegmentPlanner::new(tot, max_conns))
        } else {
            None
        };
        Self {
            task_id,
            validator,
            planner,
            single_stream_bytes: 0,
            expected_sha256,
        }
    }

    pub fn verify_final_bytes(&self, payload: &[u8]) -> Result<String, ErrorDetail> {
        if let Some(tot) = self.validator.total_bytes {
            if payload.len() as u64 != tot {
                return Err(ErrorDetail {
                    code: ErrorCode::NetworkError,
                    message: format!("Final payload size {} != expected {}", payload.len(), tot),
                    retryable: true,
                    retry_after_ms: None,
                });
            }
        }
        if let Some(planner) = &self.planner {
            planner.verify_complete_coverage()?;
        }
        let actual_sha = hex::encode(Sha256::digest(payload));
        if let Some(exp) = &self.expected_sha256 {
            if &actual_sha != exp {
                return Err(ErrorDetail {
                    code: ErrorCode::InternalError,
                    message: format!("SHA-256 mismatch: expected {}, got {}", exp, actual_sha),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
        }
        Ok(actual_sha)
    }

    pub fn to_checkpoint(&self, generation: u64, part_path: String, final_path: String) -> TransferCheckpoint {
        let intervals = self
            .planner
            .as_ref()
            .map(|p| {
                p.segments
                    .iter()
                    .filter(|s| s.committed_offset > s.start)
                    .map(|s| crate::planner::ByteInterval {
                        start: s.start,
                        end: s.committed_offset,
                    })
                    .collect()
            })
            .unwrap_or_default();
        TransferCheckpoint {
            schema_version: mdm_ipc::CURRENT_SCHEMA_VERSION,
            task_id: self.task_id.clone(),
            checkpoint_generation: generation,
            source_validator: self.validator.clone(),
            total_bytes: self.validator.total_bytes,
            part_path,
            final_path,
            synced_intervals: intervals,
            expected_sha256: self.expected_sha256.clone(),
        }
    }
}
