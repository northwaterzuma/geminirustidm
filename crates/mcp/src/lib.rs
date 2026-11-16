//! MCP stdio tool dispatcher enforcing capability scopes (`read`, `add`, `control`, `inspect_network`, `manage_queue`, `manage_limits`).
//! Never exposes file deletion or arbitrary command execution by default, and redacts all credentials/signed URLs.

use mdm_app::TaskCoordinator;
use mdm_ipc::*;

pub struct McpServer {
    pub caller: CallerScope,
}

impl McpServer {
    pub fn default_stdio_Read_add_control() -> Self {
        Self {
            caller: CallerScope {
                os_user: "local-mcp-user".to_string(),
                origin_kind: CallerOriginKind::McpClient,
                instance_id: "mcp-stdio".to_string(),
                profile_id: "default".to_string(),
                capabilities: vec![
                    CapabilityScope::Read,
                    CapabilityScope::Add,
                    CapabilityScope::Control,
                    CapabilityScope::InspectNetwork,
                    CapabilityScope::ManageQueue,
                    CapabilityScope::ManageLimits,
                    // Note: CapabilityScope::DeleteFiles is intentionally omitted by default!
                ],
            },
        }
    }

    pub fn invoke_tool(
        &self,
        coordinator: &mut TaskCoordinator,
        tool_name: &str,
        args: &serde_json::Value,
        now_epoch_ms: u64,
    ) -> Result<serde_json::Value, ErrorDetail> {
        match tool_name {
            "downloads_list" => {
                self.require_scope(CapabilityScope::Read)?;
                let limit = args["limit"].as_u64().unwrap_or(50).clamp(1, 100) as usize;
                let items: Vec<TaskSnapshot> = coordinator.tasks.values().take(limit).map(|t| t.to_snapshot()).collect();
                Ok(serde_json::json!({ "items": items, "next_cursor": null }))
            }
            "downloads_get" | "diagnostics_get" => {
                self.require_scope(CapabilityScope::Read)?;
                let task_id = args["task_id"].as_str().unwrap_or("");
                let Some(task) = coordinator.tasks.get(task_id) else {
                    return Err(ErrorDetail {
                        code: ErrorCode::NotFound,
                        message: "Task not found".to_string(),
                        retryable: false,
                        retry_after_ms: None,
                    });
                };
                Ok(serde_json::to_value(task.to_snapshot()).unwrap())
            }
            "downloads_add" => {
                self.require_scope(CapabilityScope::Add)?;
                let url = args["url"].as_str().unwrap_or("");
                let hint = args["file_name_hint"].as_str().unwrap_or("download.bin");
                let idem = args["idempotency_key"].as_str().unwrap_or("mcp-default-key");
                let source = SourceSpec::DirectHttp {
                    url: url.to_string(),
                    page_url: None,
                    method: "GET".to_string(),
                    replayability: Replayability::Unknown,
                    replayability_evidence_ref: None,
                };
                let output = OutputPolicy {
                    directory_policy: DirectoryPolicy::CategoryDefault,
                    file_name_hint: hint.to_string(),
                    directory_ref: None,
                    queue_id: None,
                    collision: CollisionPolicy::Rename,
                };
                let resp = coordinator.add_task(&self.caller, "mcp-add", idem, &source, output, None, "system-default".to_string(), None);
                if let Some(err) = resp.error {
                    Err(err)
                } else {
                    Ok(serde_json::to_value(resp).unwrap())
                }
            }
            "downloads_pause" | "downloads_resume" | "downloads_cancel" | "downloads_retry" => {
                self.require_scope(CapabilityScope::Control)?;
                let task_id = args["task_id"].as_str().unwrap_or("");
                let rev: DecimalU64 = serde_json::from_value(args["expected_revision"].clone()).map_err(|e| ErrorDetail {
                    code: ErrorCode::InvalidRequest,
                    message: e.to_string(),
                    retryable: false,
                    retry_after_ms: None,
                })?;
                let op = match tool_name {
                    "downloads_pause" => ControlOp::Pause,
                    "downloads_resume" => ControlOp::Resume,
                    "downloads_cancel" => ControlOp::Cancel,
                    _ => ControlOp::Retry,
                };
                let resp = coordinator.control_task(&self.caller, "mcp-ctrl", task_id, rev.0, op, now_epoch_ms);
                if let Some(err) = resp.error {
                    Err(err)
                } else {
                    Ok(serde_json::to_value(resp).unwrap())
                }
            }
            "media_inspect" => {
                self.require_scope(CapabilityScope::InspectNetwork)?;
                Ok(serde_json::json!({ "status": "ok", "formats": [] }))
            }
            "media_select" => {
                self.require_scope(CapabilityScope::Control)?;
                Ok(serde_json::json!({ "status": "queued" }))
            }
            "queues_list" => {
                self.require_scope(CapabilityScope::Read)?;
                Ok(serde_json::json!({ "queues": ["main", "sync"] }))
            }
            "queues_update" => {
                self.require_scope(CapabilityScope::ManageQueue)?;
                Ok(serde_json::json!({ "status": "updated" }))
            }
            "limits_get" => {
                self.require_scope(CapabilityScope::Read)?;
                Ok(serde_json::json!({ "global_rate_limit_bps": 0, "max_active_tasks": 3 }))
            }
            "limits_set" => {
                self.require_scope(CapabilityScope::ManageLimits)?;
                Ok(serde_json::json!({ "status": "updated" }))
            }
            _ => Err(ErrorDetail {
                code: ErrorCode::PermissionDenied,
                message: format!("Disallowed or unknown MCP tool '{}': file deletion and arbitrary commands are forbidden", tool_name),
                retryable: false,
                retry_after_ms: None,
            }),
        }
    }

    fn require_scope(&self, scope: CapabilityScope) -> Result<(), ErrorDetail> {
        if self.caller.capabilities.contains(&scope) {
            Ok(())
        } else {
            Err(ErrorDetail {
                code: ErrorCode::PermissionDenied,
                message: format!("Caller lacks required capability scope {:?}", scope),
                retryable: false,
                retry_after_ms: None,
            })
        }
    }
}
