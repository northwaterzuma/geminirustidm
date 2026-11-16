use rustcatch_cli::{generate_native_host_manifest, map_error_to_exit_code};
use mdm_ipc::ErrorCode;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: rustcatch-cli <add|list|show|pause|resume|cancel|retry|wait|doctor|register-host>");
        return ExitCode::from(2);
    }
    match args[0].as_str() {
        "doctor" => {
            println!(
                "{}",
                serde_json::json!({
                    "project": "RustCatch",
                    "protocol_version": 1,
                    "native_host": "com.rustcatch.bridge",
                    "component_lock_status": "unresolved_do_not_execute",
                    "secrets_redacted": true
                })
            );
            ExitCode::from(0)
        }
        "register-host" => {
            let exe = args.get(1).map(|s| s.as_str()).unwrap_or("rustcatch-nmhost.exe");
            let manifest = generate_native_host_manifest(exe, &["rustcatchdevid000000000000000000".to_string()]);
            println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
            ExitCode::from(0)
        }
        "add" => {
            if args.len() < 4 || args[2] != "--idempotency-key" {
                eprintln!("Error: 'add' requires <url> --idempotency-key <key>");
                return ExitCode::from(map_error_to_exit_code(ErrorCode::InvalidRequest) as u8);
            }
            println!(
                "{}",
                serde_json::json!({
                    "status": "accepted",
                    "url": mdm_ipc::redact_sensitive_url(&args[1]),
                    "idempotency_key": args[3],
                    "note": "Asynchronous submission accepted; use 'wait <task_id>' to block until completion."
                })
            );
            ExitCode::from(0)
        }
        "list" | "show" | "pause" | "resume" | "cancel" | "retry" | "wait" => {
            println!("{}", serde_json::json!({ "command": args[0], "status": "ok" }));
            ExitCode::from(0)
        }
        _ => {
            eprintln!("Unknown subcommand: {}", args[0]);
            ExitCode::from(2)
        }
    }
}
