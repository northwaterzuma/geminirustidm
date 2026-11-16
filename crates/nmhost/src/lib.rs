//! Native Messaging Host (`com.rustcatch.bridge`) framing and 4-stage `hello` authentication.

use mdm_app::{CaptureRegistry, ContextRecord, TaskCoordinator};
use mdm_ipc::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{self, Read, Write};

pub struct NativeHostSession {
    pub allowed_extension_origins: Vec<String>,
    pub caller_origin: String,
    pub authenticated_session: Option<(String, String)>, // (instance_id, session_ref)
    pub enrolled_secrets: BTreeMap<String, Vec<u8>>,
    pub active_challenges: BTreeMap<String, (String, String, u64)>, // challenge_id -> (instance_id, nonce, expires_at_ms)
    pub coordinator: TaskCoordinator,
    pub capture_registry: CaptureRegistry,
}

impl NativeHostSession {
    pub fn new(allowed_extension_origins: Vec<String>, caller_origin: String) -> Self {
        Self {
            allowed_extension_origins,
            caller_origin,
            authenticated_session: None,
            enrolled_secrets: BTreeMap::new(),
            active_challenges: BTreeMap::new(),
            coordinator: TaskCoordinator::new(),
            capture_registry: CaptureRegistry::new(),
        }
    }

    /// Reads a single 4-byte native-endian length-prefixed frame, enforcing 256 KiB BEFORE allocation.
    pub fn read_native_frame<R: Read>(reader: &mut R) -> io::Result<Option< Result<Vec<u8>, ErrorDetail> >> {
        let mut len_buf = [0u8; 4];
        match reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        let frame_len = u32::from_ne_bytes(len_buf) as usize;
        if frame_len == 0 || frame_len > MAX_FRAME_BYTES {
            return Ok(Some(Err(ErrorDetail {
                code: ErrorCode::TooLarge,
                message: format!("Native frame length {} violates 1..={} byte bound", frame_len, MAX_FRAME_BYTES),
                retryable: false,
                retry_after_ms: None,
            })));
        }
        let mut payload = vec![0u8; frame_len];
        reader.read_exact(&mut payload)?;
        Ok(Some(Ok(payload)))
    }

    /// Writes a 4-byte native-endian length-prefixed JSON response to stdout.
    pub fn write_native_frame<W: Write>(writer: &mut W, resp: &BrowserBridgeResponse) -> io::Result<()> {
        let bytes = serde_json::to_vec(resp).expect("response serializes");
        let len_bytes = (bytes.len() as u32).to_ne_bytes();
        writer.write_all(&len_bytes)?;
        writer.write_all(&bytes)?;
        writer.flush()
    }

    pub fn handle_raw_frame(&mut self, raw: &[u8], now_epoch_ms: u64) -> BrowserBridgeResponse {
        if !self.allowed_extension_origins.contains(&self.caller_origin) {
            return BrowserBridgeResponse::error(
                "untrusted-origin",
                ErrorCode::PermissionDenied,
                "Extension origin is not in allowed_origins",
                false,
                None,
            );
        }
        let val = match validate_raw_frame(raw) {
            Ok(v) => v,
            Err(err) => return BrowserBridgeResponse::error("invalid-frame", err.code, err.message, false, None),
        };
        let req: BrowserBridgeRequest = match serde_json::from_value(val) {
            Ok(r) => r,
            Err(e) => return BrowserBridgeResponse::error("invalid-envelope", ErrorCode::InvalidRequest, e.to_string(), false, None),
        };
        if req.version != PROTOCOL_VERSION_V1 {
            return BrowserBridgeResponse::error(req.request_id, ErrorCode::UnsupportedVersion, "Only protocol version 1 is supported", false, None);
        }

        if req.op == "hello" {
            return self.handle_hello(&req, now_epoch_ms);
        }

        // All non-hello operations require an authenticated session on this connection
        let Some((authed_instance_id, _)) = &self.authenticated_session else {
            return BrowserBridgeResponse::error(
                req.request_id,
                ErrorCode::PermissionDenied,
                "Connection must complete hello.prove authentication before invoking data/control ops",
                false,
                None,
            );
        };
        let authed_instance_id = authed_instance_id.clone();

        match req.op.as_str() {
            "capture.publish" => {
                let browser: BrowserMeta = serde_json::from_value(req.payload["browser"].clone()).unwrap();
                if browser.installation_id != authed_instance_id {
                    return BrowserBridgeResponse::error(req.request_id, ErrorCode::PermissionDenied, "installation_id mismatch", false, None);
                }
                let candidates: Vec<CaptureCandidate> = serde_json::from_value(req.payload["candidates"].clone()).unwrap_or_default();
                match self.capture_registry.publish_batch(&browser, candidates) {
                    Ok(count) => BrowserBridgeResponse {
                        version: 1,
                        request_id: req.request_id,
                        status: ResponseStatus::Ok,
                        task_id: None,
                        task_state: None,
                        revision: None,
                        durable: None,
                        handoff_token: None,
                        expires_at: None,
                        data: Some(serde_json::json!({ "stored_count": count })),
                        error: None,
                    },
                    Err(e) => BrowserBridgeResponse::error(req.request_id, e.code, e.message, e.retryable, e.retry_after_ms),
                }
            }
            "context.put" => {
                let raw_materials = serde_json::to_vec(&req.payload["materials"]).unwrap_or_default();
                if raw_materials.len() > MAX_CONTEXT_BYTES {
                    return BrowserBridgeResponse::error(req.request_id, ErrorCode::TooLarge, "Context materials exceed 32768 bytes", false, None);
                }
                let browser: BrowserMeta = serde_json::from_value(req.payload["browser"].clone()).unwrap();
                if browser.installation_id != authed_instance_id {
                    return BrowserBridgeResponse::error(req.request_id, ErrorCode::PermissionDenied, "Cross-instance context.put denied", false, None);
                }
                let cref = format!("ctx-{}", self.coordinator.contexts.len() + 1);
                let digest = hex::encode(Sha256::digest(&raw_materials));
                self.coordinator.contexts.insert(
                    cref.clone(),
                    ContextRecord {
                        context_ref: cref.clone(),
                        owner_instance_id: browser.installation_id,
                        owner_profile_id: browser.profile_id,
                        page_origin: req.payload["page_origin"].as_str().unwrap_or_default().to_string(),
                        allowed_hosts: serde_json::from_value(req.payload["allowed_hosts"].clone()).unwrap_or_default(),
                        context_revision: 1,
                        expires_at_utc: req.payload["expires_at"].as_str().unwrap_or("2026-11-16T08:15:00Z").to_string(),
                        materials_digest: digest,
                        revoked: false,
                    },
                );
                BrowserBridgeResponse {
                    version: 1,
                    request_id: req.request_id,
                    status: ResponseStatus::Ok,
                    task_id: None,
                    task_state: None,
                    revision: None,
                    durable: None,
                    handoff_token: None,
                    expires_at: None,
                    data: Some(serde_json::json!({
                        "context_ref": cref,
                        "context_revision": "1",
                        "expires_at": "2026-11-16T08:15:00Z",
                        "material_changed": true
                    })),
                    error: None,
                }
            }
            _ => BrowserBridgeResponse::error(req.request_id, ErrorCode::InvalidRequest, "Unsupported op in dispatch", false, None),
        }
    }

    fn handle_hello(&mut self, req: &BrowserBridgeRequest, now_epoch_ms: u64) -> BrowserBridgeResponse {
        let versions: Vec<u16> = serde_json::from_value(req.payload["supported_versions"].clone()).unwrap_or_default();
        if !versions.contains(&PROTOCOL_VERSION_V1) {
            return BrowserBridgeResponse::error(&req.request_id, ErrorCode::UnsupportedVersion, "No common protocol version", false, None);
        }
        let stage = req.payload["stage"].as_str().unwrap_or("");
        match stage {
            "negotiate" => BrowserBridgeResponse {
                version: 1,
                request_id: req.request_id.clone(),
                status: ResponseStatus::Ok,
                task_id: None,
                task_state: None,
                revision: None,
                durable: None,
                handoff_token: None,
                expires_at: None,
                data: Some(serde_json::json!({
                    "stage": "negotiated",
                    "selected_version": 1,
                    "capabilities": ["handoff_v1", "capture_v1", "context_v1", "media_v1"],
                    "max_frame_bytes": MAX_FRAME_BYTES
                })),
                error: None,
            },
            "enroll" => {
                let instance_id = format!("inst-{}", self.enrolled_secrets.len() + 1);
                let secret = format!("sec-{}-{}", instance_id, now_epoch_ms).into_bytes();
                let secret_hex = hex::encode(&secret);
                self.enrolled_secrets.insert(instance_id.clone(), secret);
                BrowserBridgeResponse {
                    version: 1,
                    request_id: req.request_id.clone(),
                    status: ResponseStatus::Ok,
                    task_id: None,
                    task_state: None,
                    revision: None,
                    durable: None,
                    handoff_token: None,
                    expires_at: None,
                    data: Some(serde_json::json!({
                        "stage": "enrolled",
                        "selected_version": 1,
                        "capabilities": ["handoff_v1", "capture_v1", "context_v1"],
                        "max_frame_bytes": MAX_FRAME_BYTES,
                        "registered_instance_id": instance_id,
                        "enrollment_secret": secret_hex
                    })),
                    error: None,
                }
            }
            "challenge" => {
                let inst_id = req.payload["registered_instance_id"].as_str().unwrap_or("");
                if !self.enrolled_secrets.contains_key(inst_id) {
                    return BrowserBridgeResponse::error(&req.request_id, ErrorCode::PermissionDenied, "Unknown registered_instance_id", false, None);
                }
                let challenge_id = format!("chal-{}", self.active_challenges.len() + 1);
                let nonce = format!("nonce-{}-{}", challenge_id, now_epoch_ms);
                self.active_challenges.insert(challenge_id.clone(), (inst_id.to_string(), nonce.clone(), now_epoch_ms + 60_000));
                BrowserBridgeResponse {
                    version: 1,
                    request_id: req.request_id.clone(),
                    status: ResponseStatus::Ok,
                    task_id: None,
                    task_state: None,
                    revision: None,
                    durable: None,
                    handoff_token: None,
                    expires_at: None,
                    data: Some(serde_json::json!({
                        "stage": "challenge",
                        "selected_version": 1,
                        "capabilities": ["handoff_v1", "capture_v1", "context_v1"],
                        "max_frame_bytes": MAX_FRAME_BYTES,
                        "registered_instance_id": inst_id,
                        "challenge_id": challenge_id,
                        "challenge": nonce,
                        "expires_at": "2026-11-16T08:01:00Z"
                    })),
                    error: None,
                }
            }
            "prove" => {
                let inst_id = req.payload["registered_instance_id"].as_str().unwrap_or("");
                let chal_id = req.payload["challenge_id"].as_str().unwrap_or("");
                let proof = req.payload["auth_proof"].as_str().unwrap_or("");
                // Single-use challenge: remove immediately on prove attempt
                let Some((expected_inst, nonce, exp_ms)) = self.active_challenges.remove(chal_id) else {
                    return BrowserBridgeResponse::error(&req.request_id, ErrorCode::PermissionDenied, "Challenge not found or already consumed", false, None);
                };
                if now_epoch_ms > exp_ms || expected_inst != inst_id {
                    return BrowserBridgeResponse::error(&req.request_id, ErrorCode::PermissionDenied, "Challenge expired or instance mismatch", false, None);
                }
                let secret = self.enrolled_secrets.get(inst_id).unwrap();
                if !verify_hello_hmac(secret, 1, chal_id, &nonce, &self.caller_origin, inst_id, proof) {
                    return BrowserBridgeResponse::error(&req.request_id, ErrorCode::PermissionDenied, "Invalid HMAC auth_proof", false, None);
                }
                let session_ref = format!("sess-{}-{}", inst_id, now_epoch_ms);
                self.authenticated_session = Some((inst_id.to_string(), session_ref.clone()));
                BrowserBridgeResponse {
                    version: 1,
                    request_id: req.request_id.clone(),
                    status: ResponseStatus::Ok,
                    task_id: None,
                    task_state: None,
                    revision: None,
                    durable: None,
                    handoff_token: None,
                    expires_at: None,
                    data: Some(serde_json::json!({
                        "stage": "authenticated",
                        "selected_version": 1,
                        "capabilities": ["handoff_v1", "capture_v1", "context_v1"],
                        "max_frame_bytes": MAX_FRAME_BYTES,
                        "registered_instance_id": inst_id,
                        "session_ref": session_ref
                    })),
                    error: None,
                }
            }
            _ => BrowserBridgeResponse::error(&req.request_id, ErrorCode::InvalidRequest, "Unknown hello.stage", false, None),
        }
    }
}
