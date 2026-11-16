# MCP 服务端权限边界与安全规范 (MCP_PERMISSIONS - P4-T02)

- **模块位置**：`crates/mcp/src/lib.rs`
- **工具清单 (9 项)**：`downloads_list`, `downloads_get`, `downloads_add`, `downloads_pause/resume/cancel/retry`, `media_inspect`, `media_select`, `queues_list/queues_update`, `limits_get/limits_set`, `diagnostics_get`。
- **强安全边界**：
  1. 默认 stdio MCP 会话仅授予 `Read / Add / Control / InspectNetwork / ManageQueue / ManageLimits`，**默认不包含 `DeleteFiles` 权限**。
  2. 不提供任何任意命令执行、自定义 FFmpeg 参数透传或任意本地文件读取入口。
  3. 所有文件名经过 `sanitize_file_name` 过滤 `..`、`/`、`\`、`:` (Windows ADS) 与保留设备名。
