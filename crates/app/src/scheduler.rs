//! Timezone/DST/Sleep-aware queue scheduler with `occurrence_id` deduplication
//! and post-completion actions (including cancellable shutdown countdown).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissedRunPolicy {
    SkipMissed,
    RunOnceOnWake,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleRule {
    pub schedule_id: String,
    pub queue_id: String,
    pub timezone_iana: String,
    pub missed_policy: MissedRunPolicy,
    pub executed_occurrences: HashSet<String>,
}

impl ScheduleRule {
    /// Fires at most once per `(schedule_id, occurrence_id)` across sleep/wake, DST shifts, and restarts.
    pub fn should_trigger(&mut self, occurrence_id: &str, is_missed_window: bool) -> bool {
        if self.executed_occurrences.contains(occurrence_id) {
            return false;
        }
        if is_missed_window && self.missed_policy == MissedRunPolicy::SkipMissed {
            self.executed_occurrences.insert(occurrence_id.to_string());
            return false;
        }
        self.executed_occurrences.insert(occurrence_id.to_string());
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionActionConfig {
    pub notify_desktop: bool,
    pub open_folder_on_complete: bool,
    pub antivirus_executable: Option<String>,
    pub antivirus_args: Vec<String>,
    pub shutdown_on_queue_complete: bool,
    pub shutdown_countdown_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownCountdownState {
    pub active: bool,
    pub remaining_seconds: u32,
    pub cancelled_by_user: bool,
}

impl ShutdownCountdownState {
    pub fn start_if_eligible(cfg: &CompletionActionConfig, queue_succeeded: bool) -> Self {
        if queue_succeeded && cfg.shutdown_on_queue_complete {
            Self {
                active: true,
                remaining_seconds: cfg.shutdown_countdown_seconds.max(30),
                cancelled_by_user: false,
            }
        } else {
            Self {
                active: false,
                remaining_seconds: 0,
                cancelled_by_user: false,
            }
        }
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.cancelled_by_user = true;
    }
}
