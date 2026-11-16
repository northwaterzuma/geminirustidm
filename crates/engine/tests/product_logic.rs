//! Real product-code tests for the RustCatch HTTP engine.
//! These call the actual engine functions (not re-implementations) and cover
//! the deterministic protocol boundaries required by TEST-02B / TEST-03A.

use mdm_engine::checkpoint::{
    reconcile_startup_checkpoint, CrashReconcileOutcome, TransferCheckpoint,
};
use mdm_engine::http_validator::{
    evaluate_range_response, parse_content_range, RangeResponseDecision, SourceValidator,
};
use mdm_engine::planner::{ByteInterval, SegmentPlanner};
use mdm_ipc::ErrorCode;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn v(etag: Option<&str>, total: Option<u64>) -> SourceValidator {
    SourceValidator::from_headers(etag, Some("Mon, 16 Nov 2026 00:00:00 GMT"), total, None)
}

fn unsupported(e: Result<RangeResponseDecision, mdm_ipc::ErrorDetail>) -> bool {
    matches!(e, Err(ref d) if d.code == ErrorCode::ResourceChanged)
}

// ---------- TEST-02B: Content-Range parsing ----------

#[test]
fn test_parse_content_range_valid_and_wildcard() {
    let cr = parse_content_range("bytes 100-199/1000").unwrap();
    assert_eq!(cr.start, Some(100));
    assert_eq!(cr.end, Some(199));
    assert_eq!(cr.total, Some(1000));

    let cr = parse_content_range("bytes */4096").unwrap();
    assert_eq!(cr.start, None);
    assert_eq!(cr.end, None);
    assert_eq!(cr.total, Some(4096));
}

#[test]
fn test_parse_content_range_rejects_invalid() {
    assert!(parse_content_range("items 0-9/10").is_err());
    assert!(parse_content_range("bytes 9-0/10").is_err()); // start > end
    assert!(parse_content_range("bytes 0-10/10").is_err()); // end >= total
    assert!(parse_content_range("bytes abc-def/10").is_err());
    assert!(parse_content_range("bytes 0-9").is_err()); // missing '/'
}

// ---------- TEST-02B: 206 / short 206 / 200 / 416 ----------

#[test]
fn test_206_exact_range_accepted() {
    let base = v(Some("\"strong-etag-1\""), Some(1000));
    let d = evaluate_range_response(206, 100, 200, Some("bytes 100-199/1000"), 100, &base, &base, false, false).unwrap();
    match d {
        RangeResponseDecision::WritePartial { start, end_exclusive, remaining_start } => {
            assert_eq!(start, 100);
            assert_eq!(end_exclusive, 200);
            assert_eq!(remaining_start, None);
        }
        other => panic!("expected WritePartial, got {:?}", other),
    }
}

#[test]
fn test_206_short_response_reschedules_remaining_tail() {
    let base = v(Some("\"strong-etag-1\""), Some(1000));
    let d = evaluate_range_response(206, 100, 300, Some("bytes 100-179/1000"), 80, &base, &base, false, false).unwrap();
    match d {
        RangeResponseDecision::WritePartial { start, end_exclusive, remaining_start } => {
            assert_eq!((start, end_exclusive), (100, 180));
            assert_eq!(remaining_start, Some(180)); // tail [180,300) must be re-fetched
        }
        other => panic!("expected WritePartial, got {:?}", other),
    }
}

#[test]
fn test_206_wrong_start_and_mismatched_body_rejected() {
    let base = v(Some("\"strong-etag-1\""), Some(1000));
    // Content-Range start does not match request
    assert!(evaluate_range_response(206, 100, 200, Some("bytes 101-199/1000"), 99, &base, &base, false, false).is_err());
    // Body length does not match declared range
    assert!(evaluate_range_response(206, 100, 200, Some("bytes 100-199/1000"), 50, &base, &base, false, false).is_err());
    // 206 without Content-Range
    assert!(evaluate_range_response(206, 0, 100, None, 100, &base, &base, false, false).is_err());
}

#[test]
fn test_200_on_nonzero_offset_never_writes_partial() {
    let base = v(Some("\"strong-etag-1\""), Some(1000));
    // Range requested at offset 100 but server ignored it and sent full body:
    assert!(evaluate_range_response(200, 100, 200, None, 1000, &base, &base, false, false).is_err());
    // Also reject when other segments were already committed at non-zero offsets
    assert!(evaluate_range_response(200, 0, 100, None, 1000, &base, &base, true, false).is_err());
}

#[test]
fn test_200_on_offset_zero_downgrades_to_single_stream() {
    let base = v(Some("\"strong-etag-1\""), Some(1000));
    let d = evaluate_range_response(200, 0, 1000, None, 1000, &base, &base, false, false).unwrap();
    assert!(matches!(d, RangeResponseDecision::DowngradeToSingleStream));
}

#[test]
fn test_416_only_transitions_when_source_matches_and_coverage_complete() {
    let base = v(Some("\"strong-etag-1\""), Some(1000));
    let ok = evaluate_range_response(416, 1000, 1100, Some("bytes */1000"), 0, &base, &base, true, true).unwrap();
    assert!(matches!(ok, RangeResponseDecision::TransitionToVerify));

    // Total size changed -> resource changed
    assert!(unsupported(evaluate_range_response(416, 1000, 1100, Some("bytes */1200"), 0, &base, &base, true, true)));
    // Intervals incomplete -> reject
    assert!(unsupported(evaluate_range_response(416, 1000, 1100, Some("bytes */1000"), 0, &base, &base, true, false)));
}

#[test]
fn test_source_validator_change_refuses_splice() {
    let base = v(Some("\"strong-etag-1\""), Some(1000));
    let changed = v(Some("\"strong-etag-2\""), Some(1000));
    assert!(unsupported(evaluate_range_response(206, 0, 100, Some("bytes 0-99/1000"), 100, &base, &changed, false, false)));
}

#[test]
fn test_weak_etag_is_not_strong_validator() {
    let weak = v(Some("W/\"weak-1\""), Some(1000));
    assert!(!weak.is_strong_etag);
    assert!(weak.if_range_header().is_none()); // must not be used for If-Range
    // A weak ETag is never a strong validator: product refuses reliable resume
    // even when Last-Modified is also present (conservative, spec-aligned).
    assert!(!weak.can_resume_reliably());
    let weak_only = SourceValidator::from_headers(Some("W/\"weak-1\""), None, Some(1000), None);
    assert!(!weak_only.can_resume_reliably());
    // Strong ETag and Last-Modified-only sources remain resumable.
    let strong = v(Some("\"strong-1\""), Some(1000));
    assert!(strong.can_resume_reliably());
    let lm_only = SourceValidator::from_headers(None, Some("Mon, 16 Nov 2026 00:00:00 GMT"), Some(1000), None);
    assert!(lm_only.can_resume_reliably());
}

#[test]
fn test_429_and_401_classified() {
    let base = v(Some("\"strong-etag-1\""), Some(1000));
    let e429 = evaluate_range_response(429, 0, 100, None, 0, &base, &base, false, false).unwrap_err();
    assert_eq!(e429.code, ErrorCode::RateLimited);
    let e401 = evaluate_range_response(401, 0, 100, None, 0, &base, &base, false, false).unwrap_err();
    assert_eq!(e401.code, ErrorCode::AuthRequired);
}

// ---------- TEST-02B: segment planner ----------

#[test]
fn test_planner_initial_segments_cover_whole_file() {
    let p = SegmentPlanner::new(32 * 1024 * 1024, 4);
    assert_eq!(p.segments.len(), 4);
    let sum: u64 = p.segments.iter().map(|s| s.end - s.start).sum();
    assert_eq!(sum, 32 * 1024 * 1024);
    for w in p.segments.windows(2) {
        assert_eq!(w[0].end, w[1].start, "segments must not overlap or leave gaps");
    }
}

#[test]
fn test_stale_generation_write_is_fenced() {
    let mut p = SegmentPlanner::new(64 * 1024 * 1024, 1);
    let lease = p.split_largest_tail("worker-2".to_string()).unwrap();
    let original_seg_id = 0u32;
    let original_gen = 1u64;
    // Old worker (generation 1) tries to commit into the segment whose generation advanced:
    let res = p.commit_progress(original_seg_id, original_gen, 1024);
    assert!(res.is_err(), "stale generation commit must be rejected");
    assert_eq!(res.unwrap_err().code, ErrorCode::RevisionConflict);
    // New lease generation commits fine
    assert!(p.commit_progress(lease.segment_id, lease.generation, lease.start + 1024).is_ok());
}

#[test]
fn test_commit_progress_rejects_backwards_and_past_end() {
    let mut p = SegmentPlanner::new(1024, 1);
    assert!(p.commit_progress(0, 1, 100).is_ok());
    assert!(p.commit_progress(0, 1, 50).is_err()); // backwards
    assert!(p.commit_progress(0, 1, 2048).is_err()); // beyond end
}

#[test]
fn test_complete_coverage_detects_gap() {
    let mut p = SegmentPlanner::new(4096, 1);
    assert!(p.verify_complete_coverage().is_err()); // nothing committed yet
    p.commit_progress(0, 1, 4096).unwrap();
    assert!(p.verify_complete_coverage().is_ok());
}

#[test]
fn test_byte_interval_len() {
    assert_eq!(ByteInterval { start: 10, end: 30 }.len(), 20);
    assert!(ByteInterval { start: 30, end: 10 }.is_empty());
}

// ---------- TEST-03A: checkpoint crash recovery ----------

static CKPT_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(name: &str) -> PathBuf {
    let n = CKPT_SEQ.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("rustcatch-test-{}-{}-{}", name, std::process::id(), n));
    let _ = fs::create_dir_all(&p);
    p
}

fn ckpt(part: &str, final_p: &str, total: Option<u64>, sha: Option<&str>, intervals: Vec<ByteInterval>) -> TransferCheckpoint {
    TransferCheckpoint {
        schema_version: 1,
        task_id: "task-test".to_string(),
        checkpoint_generation: 1,
        source_validator: v(Some("\"etag\""), total),
        total_bytes: total,
        part_path: part.to_string(),
        final_path: final_p.to_string(),
        synced_intervals: intervals,
        expected_sha256: sha.map(str::to_string),
    }
}

#[test]
fn test_checkpoint_durable_roundtrip() {
    let dir = temp_dir("ckpt-roundtrip");
    let state = dir.join("task.state.json");
    let c = ckpt(
        dir.join("task.part").to_str().unwrap(),
        dir.join("task.bin").to_str().unwrap(),
        Some(100),
        None,
        vec![ByteInterval { start: 0, end: 50 }],
    );
    c.save_durable(&state).unwrap();
    let loaded = TransferCheckpoint::load_checked(&state).unwrap();
    assert_eq!(loaded.task_id, c.task_id);
    assert_eq!(loaded.synced_intervals.len(), 1);
    assert_eq!(loaded.synced_intervals[0].end, 50);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_higher_schema_checkpoint_refused() {
    let dir = temp_dir("ckpt-schema");
    let state = dir.join("task.state.json");
    let mut c = ckpt("", "", Some(10), None, vec![]);
    c.schema_version = 999;
    assert_eq!(c.save_durable(&state).unwrap_err().code, ErrorCode::UnsupportedVersion);
    assert!(!state.exists(), "higher schema must never be written");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_crash_after_rename_verifies_sha_before_completed() {
    let dir = temp_dir("crash-rename");
    let final_p = dir.join("task.bin");
    let data = b"rustcatch verified payload";
    fs::write(&final_p, data).unwrap();
    let mut hasher = sha2::Sha256::new();
    use sha2::Digest;
    hasher.update(data);
    let sha = hex::encode(hasher.finalize());
    let c = ckpt(
        dir.join("task.part").to_str().unwrap(),
        final_p.to_str().unwrap(),
        Some(data.len() as u64),
        Some(&sha),
        vec![ByteInterval { start: 0, end: data.len() as u64 }],
    );
    match reconcile_startup_checkpoint(&c).unwrap() {
        CrashReconcileOutcome::PromoteToCompleted { verified_sha256 } => assert_eq!(verified_sha256, sha),
        other => panic!("expected PromoteToCompleted, got {:?}", other),
    }
    // Tampered file with mismatching SHA must not be declared completed
    let c_bad = ckpt(
        dir.join("task.part").to_str().unwrap(),
        final_p.to_str().unwrap(),
        Some(data.len() as u64),
        Some("0000000000000000000000000000000000000000000000000000000000000000"),
        vec![ByteInterval { start: 0, end: data.len() as u64 }],
    );
    assert!(matches!(reconcile_startup_checkpoint(&c_bad).unwrap(), CrashReconcileOutcome::ForeignFileCollision { .. }));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_foreign_file_collision_not_deleted() {
    let dir = temp_dir("foreign-file");
    let final_p = dir.join("unowned.bin");
    fs::write(&final_p, b"someone else's data").unwrap();
    let c = ckpt(
        dir.join("task.part").to_str().unwrap(),
        final_p.to_str().unwrap(),
        Some(64),
        None,
        vec![ByteInterval { start: 0, end: 64 }],
    );
    assert!(matches!(reconcile_startup_checkpoint(&c).unwrap(), CrashReconcileOutcome::ForeignFileCollision { .. }));
    assert!(final_p.exists(), "unowned file must never be deleted");
    let _ = fs::remove_dir_all(&dir);
}
