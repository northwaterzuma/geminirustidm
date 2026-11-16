//! Multi-queue concurrency manager, site cooldown tracker, and bounded retry backoff.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadQueue {
    pub id: String,
    pub name: String,
    pub is_sync_queue: bool,
    pub max_concurrent: usize,
    pub active_tasks: Vec<String>,
    pub pending_tasks: VecDeque<String>,
    pub paused: bool,
}

impl DownloadQueue {
    pub fn new(id: impl Into<String>, name: impl Into<String>, max_concurrent: usize, is_sync_queue: bool) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            is_sync_queue,
            max_concurrent: max_concurrent.clamp(1, 16),
            active_tasks: Vec::new(),
            pending_tasks: VecDeque::new(),
            paused: false,
        }
    }

    pub fn enqueue(&mut self, task_id: String) {
        if !self.pending_tasks.contains(&task_id) && !self.active_tasks.contains(&task_id) {
            self.pending_tasks.push_back(task_id);
        }
    }

    /// Pops next eligible task if queue is not paused and active count < `max_concurrent`.
    pub fn pop_next_slot(&mut self, global_active: usize, global_max: usize) -> Option<String> {
        if self.paused || self.active_tasks.len() >= self.max_concurrent || global_active >= global_max {
            return None;
        }
        let next = self.pending_tasks.pop_front()?;
        self.active_tasks.push(next.clone());
        Some(next)
    }
}

/// Computes bounded retry backoff (`[30, 60, 120]` seconds) respecting `Retry-After` if larger.
pub fn compute_retry_not_before_ms(
    now_epoch_ms: u64,
    retry_count: u32,
    max_retries: u32,
    retry_after_ms: Option<u64>,
) -> Option<u64> {
    if retry_count >= max_retries {
        return None;
    }
    let schedule_sec = [30u64, 60, 120];
    let idx = (retry_count as usize).min(schedule_sec.len() - 1);
    let base_ms = schedule_sec[idx] * 1000;
    let delay_ms = retry_after_ms.unwrap_or(0).max(base_ms);
    Some(now_epoch_ms + delay_ms)
}

#[derive(Debug, Default)]
pub struct SiteBudgetTracker {
    pub site_cooldown_until_ms: BTreeMap<String, u64>,
    pub active_youtube_tasks: usize,
    pub max_youtube_tasks: usize,
}
