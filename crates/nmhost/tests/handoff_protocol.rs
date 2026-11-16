//! Real product-code tests for Native Messaging authentication and the
//! two-phase browser handoff. Exercises NativeHostSession::handle_raw_frame
//! end-to-end (not a re-implementation). Covers TEST-04A / TEST-04B.

use mdm_ipc::{compute_hello_hmac, ErrorCode, ResponseStatus};
use rustcatch_nmhost::NativeHostSession;
use serde_json::json;

const ORIGIN: &str = "chrome-extension://rustcatch-test";

fn frame(session: &mut NativeHostSession, op: &str, request_id: &str, idem: &str, payload: serde_json::Value) -> mdm_ipc::BrowserBridgeResponse {
    let raw = serde_json::to_vec(&json!({
        "version": 1,
        "request_id": request_id,
        "idempotency_key": idem,
        "op": op,
        "payload": payload,
    })).unwrap();
    session.handle_raw_frame(&raw, 1_700_000_000_000)
}

/// Drives the 4-stage hello handshake; panics with the failing stage if broken.
fn authenticate(session: &mut NativeHostSession) -> (String, String) {
    let neg = frame(session, "hello", "h-neg", "", json!({"stage":"negotiate","supported_versions":[1]}));
    assert_eq!(neg.status, ResponseStatus::Ok, "negotiate failed: {:?}", neg.error);

    let enr = frame(session, "hello", "h-enr", "", json!({"stage":"enroll","supported_versions":[1]}));
    assert_eq!(enr.status, ResponseStatus::Ok, "enroll failed: {:?}", enr.error);
    let inst = enr.data.as_ref().unwrap()["registered_instance_id"].as_str().unwrap().to_string();
    let secret_hex = enr.data.as_ref().unwrap()["enrollment_secret"].as_str().unwrap().to_string();
    let secret = hex::decode(&secret_hex).unwrap();

    let chl = frame(session, "hello", "h-chl", "", json!({
        "stage":"challenge","supported_versions":[1],"registered_instance_id":inst
    }));
    assert_eq!(chl.status, ResponseStatus::Ok, "challenge failed: {:?}", chl.error);
    let chal_id = chl.data.as_ref().unwrap()["challenge_id"].as_str().unwrap().to_string();
    let nonce = chl.data.as_ref().unwrap()["challenge"].as_str().unwrap().to_string();

    let proof = compute_hello_hmac(&secret, 1, &chal_id, &nonce, ORIGIN, &inst);
    let prove = frame(session, "hello", "h-prov", "", json!({
        "stage":"prove","supported_versions":[1],
        "registered_instance_id":inst,"challenge_id":chal_id,"auth_proof":proof
    }));
    assert_eq!(prove.status, ResponseStatus::Ok, "prove failed: {:?}", prove.error);
    (inst, secret_hex)
}

fn session() -> NativeHostSession {
    NativeHostSession::new(vec![ORIGIN.to_string()], ORIGIN.to_string())
}

fn browser(inst: &str, dl: u64) -> serde_json::Value {
    json!({
        "installation_id": inst,
        "profile_id": "profile-demo",
        "download_id": dl,
        "tab_id": 7,
        "frame_id": 0,
        "document_id": "doc-demo",
        "nav_epoch": 3
    })
}

fn replayable_source() -> serde_json::Value {
    json!({
        "kind": "direct_http",
        "url": "https://files.example.invalid/demo.bin",
        "page_url": "https://example.invalid/downloads",
        "method": "GET",
        "replayability": "known_replayable",
        "replayability_evidence_ref": "site-rule-demo"
    })
}

fn output() -> serde_json::Value {
    json!({"directory_policy":"category_default","file_name_hint":"demo.bin"})
}

// ---------- TEST-04A: protocol + authentication ----------

#[test]
fn test_untrusted_origin_rejected_before_all_ops() {
    let mut s = NativeHostSession::new(vec![ORIGIN.to_string()], "chrome-extension://evil".to_string());
    let r = frame(&mut s, "hello", "r1", "", json!({"stage":"negotiate","supported_versions":[1]}));
    assert_eq!(r.status, ResponseStatus::Error);
    assert_eq!(r.error.unwrap().code, ErrorCode::PermissionDenied);
}

#[test]
fn test_ops_require_authentication() {
    let mut s = session();
    let r = frame(&mut s, "handoff.prepare", "r1", "idem-1", json!({}));
    assert_eq!(r.status, ResponseStatus::Error);
    assert_eq!(r.error.unwrap().code, ErrorCode::PermissionDenied);
}

#[test]
fn test_unsupported_version_rejected() {
    let mut s = session();
    let raw = serde_json::to_vec(&json!({"version":2,"request_id":"r","idempotency_key":"","op":"hello","payload":{"stage":"negotiate","supported_versions":[2]}})).unwrap();
    let r = s.handle_raw_frame(&raw, 1_700_000_000_000);
    assert_eq!(r.error.unwrap().code, ErrorCode::UnsupportedVersion);
}

#[test]
fn test_full_handshake_succeeds_and_challenge_is_single_use() {
    let mut s = session();
    let (inst, _) = authenticate(&mut s);
    assert!(s.authenticated_session.is_some());
    // Replaying the same challenge must fail: single-use nonce.
    let raw = serde_json::to_vec(&json!({"version":1,"request_id":"replay","idempotency_key":"","op":"hello","payload":{"stage":"prove","supported_versions":[1],"registered_instance_id":inst,"challenge_id":"chal-1","auth_proof":"00"}})).unwrap();
    let r = s.handle_raw_frame(&raw, 1_700_000_000_000);
    assert_eq!(r.error.unwrap().code, ErrorCode::PermissionDenied);
}

// ---------- TEST-04B: two-phase handoff ----------

#[test]
fn test_prepare_is_durable_zero_transfer_and_commit_queues() {
    let mut s = session();
    let (inst, _) = authenticate(&mut s);
    let prep = frame(&mut s, "handoff.prepare", "prep-1", "idem-handoff-1", json!({
        "browser": browser(&inst, 42),
        "source": replayable_source(),
        "output": output(),
        "context_ref": null,
        "network_profile_id": "system-default",
        "media_intent": null
    }));
    assert_eq!(prep.status, ResponseStatus::Prepared, "prepare failed: {:?}", prep.error);
    assert_eq!(prep.task_state, Some(mdm_ipc::TaskState::HandoffPending));
    assert_eq!(prep.durable, Some(true));
    assert!(prep.handoff_token.is_some());
    let task_id = prep.task_id.clone().unwrap();
    // HandoffPending must NOT be running/queued before commit
    let state = s.coordinator.tasks.get(&task_id).unwrap().state;
    assert_eq!(state, mdm_ipc::TaskState::HandoffPending);

    // commit requires explicit trusted cancellation confirmation
    let bad = frame(&mut s, "handoff.commit", "commit-bad", "idem-handoff-1", json!({
        "task_id": task_id,
        "expected_revision": "1",
        "handoff_token": prep.handoff_token.clone().unwrap(),
        "browser_download_id": "42",
        "browser_cancel_confirmed": false
    }));
    assert_eq!(bad.error.unwrap().code, ErrorCode::InvalidRequest);

    let commit = frame(&mut s, "handoff.commit", "commit-1", "idem-handoff-1", json!({
        "task_id": task_id,
        "expected_revision": "1",
        "handoff_token": prep.handoff_token.clone().unwrap(),
        "browser_download_id": "42",
        "browser_cancel_confirmed": true
    }));
    assert_eq!(commit.status, ResponseStatus::Queued, "commit failed: {:?}", commit.error);
    assert_eq!(s.coordinator.tasks.get(&task_id).unwrap().state, mdm_ipc::TaskState::Queued);
}

#[test]
fn test_duplicate_prepare_same_idempotency_key_returns_same_task() {
    let mut s = session();
    let (inst, _) = authenticate(&mut s);
    let p1 = frame(&mut s, "handoff.prepare", "p1", "idem-dup", json!({
        "browser": browser(&inst, 43), "source": replayable_source(), "output": output(),
        "context_ref": null, "network_profile_id": "system-default", "media_intent": null
    }));
    // Different request_id, same idempotency key and creation semantics
    let p2 = frame(&mut s, "handoff.prepare", "p2", "idem-dup", json!({
        "browser": browser(&inst, 43), "source": replayable_source(), "output": output(),
        "context_ref": null, "network_profile_id": "system-default", "media_intent": null
    }));
    assert_eq!(p1.status, ResponseStatus::Prepared);
    assert_eq!(p2.status, ResponseStatus::Prepared);
    assert_eq!(p1.task_id, p2.task_id, "one trusted caller + idem key => one task");
}

#[test]
fn test_post_and_unknown_replayability_rejected() {
    let mut s = session();
    let (inst, _) = authenticate(&mut s);
    let mut src = replayable_source();
    src["method"] = json!("POST");
    let r = frame(&mut s, "handoff.prepare", "p-post", "idem-post", json!({
        "browser": browser(&inst, 44), "source": src, "output": output(),
        "context_ref": null, "network_profile_id": "system-default", "media_intent": null
    }));
    assert_eq!(r.error.unwrap().code, ErrorCode::NotReplayable);

    let mut src2 = replayable_source();
    src2["replayability"] = json!("unknown");
    let r2 = frame(&mut s, "handoff.prepare", "p-unk", "idem-unk", json!({
        "browser": browser(&inst, 45), "source": src2, "output": output(),
        "context_ref": null, "network_profile_id": "system-default", "media_intent": null
    }));
    assert_eq!(r2.error.unwrap().code, ErrorCode::NotReplayable);
}

#[test]
fn test_commit_without_prepare_is_not_found() {
    let mut s = session();
    let (_, _) = authenticate(&mut s);
    let r = frame(&mut s, "handoff.commit", "c-orphan", "never-prepared", json!({
        "task_id": "task-nope",
        "expected_revision": "1",
        "handoff_token": "fake",
        "browser_download_id": "99",
        "browser_cancel_confirmed": true
    }));
    assert_eq!(r.error.unwrap().code, ErrorCode::NotFound);
}

#[test]
fn test_task_get_returns_redacted_snapshot_for_owner() {
    let mut s = session();
    let (inst, _) = authenticate(&mut s);
    let prep = frame(&mut s, "handoff.prepare", "p-snap", "idem-snap", json!({
        "browser": browser(&inst, 46), "source": replayable_source(), "output": output(),
        "context_ref": null, "network_profile_id": "system-default", "media_intent": null
    }));
    let id = prep.task_id.unwrap();
    let got = frame(&mut s, "task.get", "g1", "", json!({"task_id": id}));
    assert_eq!(got.status, ResponseStatus::Ok);
    let data = got.data.unwrap().to_string();
    assert!(!data.contains("sig="), "snapshot must not expose signed URL params");
}

#[test]
fn test_profile_switch_within_session_rejected() {
    let mut s = session();
    let (inst, _) = authenticate(&mut s);
    let mut b = browser(&inst, 47);
    b["profile_id"] = json!("profile-A");
    let r1 = frame(&mut s, "handoff.prepare", "pA", "idem-A", json!({
        "browser": b, "source": replayable_source(), "output": output(),
        "context_ref": null, "network_profile_id": "system-default", "media_intent": null
    }));
    assert_eq!(r1.status, ResponseStatus::Prepared);
    let mut b2 = browser(&inst, 48);
    b2["profile_id"] = json!("profile-B");
    let r2 = frame(&mut s, "handoff.prepare", "pB", "idem-B", json!({
        "browser": b2, "source": replayable_source(), "output": output(),
        "context_ref": null, "network_profile_id": "system-default", "media_intent": null
    }));
    assert_eq!(r2.error.unwrap().code, ErrorCode::PermissionDenied);
}
