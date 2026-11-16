# 批量任务与脱敏导入导出规范 (IMPORT_EXPORT - P4-T01)

- **模块位置**：`crates/app/src/batch.rs`
- **批量校验 (`parse_batch_urls`)**：逐行校验协议（仅允许 `http/https/ftp/ftps`），单批有界，同批重复 URL 自动标记 `skipped_duplicate`，非法行单独反馈行号与原因。
- **导出脱敏 (`export_redacted_tasks`)**：导出 JSON 时强制清空 `context_ref`、替换 `protected_source_ref`，并对 `source_display` 执行签名与 Token 参数脱敏（`REDACTED`），导入时绝不自动启动全部下载或覆盖已有文件。
