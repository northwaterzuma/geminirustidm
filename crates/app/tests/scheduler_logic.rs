//! Real product-code tests for the scheduler's occurrence deduplication and
//! completion-action safety. Covers TEST-08A.

use mdm_app::scheduler::{CompletionActionConfig, MissedRunPolicy, ScheduleRule, ShutdownCountdownState};

fn rule(policy: MissedRunPolicy) -> ScheduleRule {
    ScheduleRule {
        schedule_id: "sched-1".to_string(),
        queue_id: "q-default".to_string(),
        timezone_iana: "Asia/Shanghai".to_string(),
        missed_policy: policy,
        executed_occurrences: Default::default(),
    }
}

#[test]
fn test_same_occurrence_fires_only_once() {
    let mut r = rule(MissedRunPolicy::RunOnceOnWake);
    assert!(r.should_trigger("2026-11-16T09:00:00Z", false));
    assert!(!r.should_trigger("2026-11-16T09:00:00Z", false), "duplicate occurrence must not re-trigger");
    assert!(r.should_trigger("2026-11-16T10:00:00Z", false), "different occurrence should trigger");
}

#[test]
fn test_skip_missed_policy_skips_only_missed_occurrences() {
    let mut r = rule(MissedRunPolicy::SkipMissed);
    assert!(!r.should_trigger("missed-1", true), "missed run must be skipped");
    assert!(r.should_trigger("on-time-1", false), "normal occurrence must still fire");
    // A skipped occurrence is recorded, so a late replay cannot fire it either
    assert!(!r.should_trigger("missed-1", false));
}

#[test]
fn test_shutdown_only_after_success_and_is_cancellable() {
    let mut cfg = CompletionActionConfig {
        notify_desktop: true,
        open_folder_on_complete: false,
        antivirus_executable: None,
        antivirus_args: vec![],
        shutdown_on_queue_complete: true,
        shutdown_countdown_seconds: 5,
    };
    let no = ShutdownCountdownState::start_if_eligible(&cfg, false);
    assert!(!no.active, "failed/stopped queue must not trigger success action");
    let mut yes = ShutdownCountdownState::start_if_eligible(&cfg, true);
    assert!(yes.active);
    assert_eq!(yes.remaining_seconds, 30, "countdown must have a safe minimum");
    yes.cancel();
    assert!(!yes.active);
    assert!(yes.cancelled_by_user);

    cfg.shutdown_on_queue_complete = false;
    assert!(!ShutdownCountdownState::start_if_eligible(&cfg, true).active);
}
