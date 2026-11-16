//! `rustcatch-cli` command parser and exit-code mapper:
//! 0 = Accepted/Query Ok, 2 = Invalid Arguments, 3 = Service Unreachable,
//! 4 = Permission/Context Error, 5 = State/Revision Conflict, 10 = Execution Failed.

use mdm_ipc::ErrorCode;

pub fn map_error_to_exit_code(code: ErrorCode) -> i32 {
    match code {
        ErrorCode::InvalidRequest | ErrorCode::UnsupportedVersion | ErrorCode::TooLarge => 2,
        ErrorCode::PermissionDenied | ErrorCode::ContextExpired | ErrorCode::AuthRequired | ErrorCode::AuthRejected => 4,
        ErrorCode::InvalidState
        | ErrorCode::IdempotencyConflict
        | ErrorCode::RevisionConflict
        | ErrorCode::LeaseExpired
        | ErrorCode::AlreadyCommitted => 5,
        _ => 10,
    }
}

/// Generates platform-specific Native Messaging host manifest JSON (`com.rustcatch.bridge.json`).
pub fn generate_native_host_manifest(host_exe_abs_path: &str, allowed_extension_ids: &[String]) -> serde_json::Value {
    let origins: Vec<String> = allowed_extension_ids
        .iter()
        .map(|id| format!("chrome-extension://{}/", id.trim_matches('/')))
        .collect();
    serde_json::json!({
        "name": "com.rustcatch.bridge",
        "description": "RustCatch Native Messaging Host Bridge",
        "path": host_exe_abs_path,
        "type": "stdio",
        "allowed_origins": origins
    })
}
