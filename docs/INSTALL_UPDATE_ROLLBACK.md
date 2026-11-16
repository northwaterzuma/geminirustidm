# 安装、组件更新、数据迁移与回滚手册 (INSTALL_UPDATE_ROLLBACK - P4-T02)

- **产品身份隔离**：应用名 `RustCatch`、二进制 `rustcatch / rustcatch-cli / rustcatch-mcp / rustcatch-nmhost`、Native Host `com.rustcatch.bridge`，数据目录独立于上游 MDM，首次启动绝不自动扫描或覆盖上游数据。
- **Schema 版本守卫**：持久化文件携带 `schema_version = 1`；遇到更高 `schema_version` 时 `TransferCheckpoint::load_checked` 与 `save_durable` 立即拒绝写入(`UnsupportedVersion`)，保护降级安全。
- **组件原子更新与回滚**：外部工具（`yt-dlp / EJS / Deno / FFmpeg / ffprobe`）按 `ComponentSetId` 绑定运行中的 attempt；未通过 SHA-256 校验或状态为 `unresolved_do_not_execute` 的二进制严禁执行。
