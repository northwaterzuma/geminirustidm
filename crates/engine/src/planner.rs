use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};

pub const MIN_WORK_BLOCK_BYTES: u64 = 4 * 1024 * 1024; // 4 MiB

/// Half-open byte interval `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteInterval {
    pub start: u64,
    pub end: u64,
}

impl ByteInterval {
    pub fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentLease {
    pub segment_id: u32,
    pub generation: u64,
    pub start: u64,
    pub committed_offset: u64,
    pub end: u64,
    pub active_worker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentPlanner {
    pub total_bytes: u64,
    pub max_connections: u8,
    pub next_generation: u64,
    pub segments: Vec<SegmentLease>,
}

impl SegmentPlanner {
    pub fn new(total_bytes: u64, max_connections: u8) -> Self {
        let conns = max_connections.clamp(1, 16) as u64;
        let ideal_segments = if total_bytes < MIN_WORK_BLOCK_BYTES {
            1
        } else {
            (total_bytes / MIN_WORK_BLOCK_BYTES).clamp(1, conns)
        };
        let chunk = total_bytes / ideal_segments;
        let mut segments = Vec::with_capacity(ideal_segments as usize);
        let mut cursor = 0;
        for idx in 0..ideal_segments {
            let end = if idx + 1 == ideal_segments {
                total_bytes
            } else {
                cursor + chunk
            };
            segments.push(SegmentLease {
                segment_id: idx as u32,
                generation: 1,
                start: cursor,
                committed_offset: cursor,
                end,
                active_worker: None,
            });
            cursor = end;
        }
        Self {
            total_bytes,
            max_connections,
            next_generation: 2,
            segments,
        }
    }

    /// Commits bytes for a segment only if `generation` matches the current lease generation.
    pub fn commit_progress(&mut self, segment_id: u32, generation: u64, new_offset: u64) -> Result<(), ErrorDetail> {
        let seg = self.segments.iter_mut().find(|s| s.segment_id == segment_id).ok_or_else(|| ErrorDetail {
            code: ErrorCode::InvalidRequest,
            message: "Unknown segment_id".to_string(),
            retryable: false,
            retry_after_ms: None,
        })?;
        if seg.generation != generation {
            return Err(ErrorDetail {
                code: ErrorCode::RevisionConflict,
                message: format!("Stale worker generation {} (current {})", generation, seg.generation),
                retryable: false,
                retry_after_ms: None,
            });
        }
        if new_offset < seg.committed_offset || new_offset > seg.end {
            return Err(ErrorDetail {
                code: ErrorCode::InvalidRequest,
                message: "Invalid segment offset progression".to_string(),
                retryable: false,
                retry_after_ms: None,
            });
        }
        seg.committed_offset = new_offset;
        if seg.committed_offset == seg.end {
            seg.active_worker = None;
        }
        Ok(())
    }

    /// Dynamically splits the largest remaining uncommitted tail if >= 2 * MIN_WORK_BLOCK_BYTES.
    /// Increments the generation of the split segment to fence off stale writes beyond the new end.
    pub fn split_largest_tail(&mut self, new_worker_id: String) -> Option<SegmentLease> {
        let (best_idx, remaining) = self
            .segments
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.end.saturating_sub(s.committed_offset)))
            .max_by_key(|(_, rem)| *rem)?;

        if remaining < MIN_WORK_BLOCK_BYTES * 2 {
            return None;
        }
        let new_seg_id = self.segments.len() as u32;
        let generation = self.next_generation;
        self.next_generation += 2;

        let seg = &mut self.segments[best_idx];
        let mid = seg.committed_offset + remaining / 2;
        let old_end = seg.end;
        seg.end = mid;
        seg.generation = generation;

        let new_lease = SegmentLease {
            segment_id: new_seg_id,
            generation: generation + 1,
            start: mid,
            committed_offset: mid,
            end: old_end,
            active_worker: Some(new_worker_id),
        };
        self.segments.push(new_lease.clone());
        Some(new_lease)
    }

    /// Verifies that committed intervals cover `[0, total_bytes)` with zero gaps and zero overlaps.
    pub fn verify_complete_coverage(&self) -> Result<(), ErrorDetail> {
        let mut intervals: Vec<ByteInterval> = self
            .segments
            .iter()
            .map(|s| ByteInterval {
                start: s.start,
                end: s.committed_offset,
            })
            .collect();
        intervals.sort_by_key(|i| i.start);
        let mut cursor = 0u64;
        for iv in intervals {
            if iv.start != cursor {
                return Err(ErrorDetail {
                    code: ErrorCode::InternalError,
                    message: format!("Segment gap or overlap detected at offset {} vs {}", cursor, iv.start),
                    retryable: false,
                    retry_after_ms: None,
                });
            }
            cursor = iv.end;
        }
        if cursor != self.total_bytes {
            return Err(ErrorDetail {
                code: ErrorCode::InternalError,
                message: format!("Incomplete coverage: committed {} of {} bytes", cursor, self.total_bytes),
                retryable: true,
                retry_after_ms: None,
            });
        }
        Ok(())
    }
}
