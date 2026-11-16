# P5-T02 站点抓取向导与周期同步规范 (GRABBER_SYNC_POLICY)

- **模块位置**：`crates/app/src/{grabber,sync}.rs`
- **四步站点抓取 (`GrabberProject`)**：默认同站（`same_site_only = true`）、最大深度 `2`、最大页数 `500`、总字节上限 `2 GiB`；不执行网页 JS；严格拦截 `file://`、`localhost`、`127.0.0.0/8` 与云元数据 `169.254.169.254` SSRF；下载完成后由 `rewrite_html_links_for_offline` 将已下载资源改写为本地相对路径，未下载外链保持原样。
- **周期同步 (`evaluate_periodic_sync`)**：通过强 ETag / Last-Modified 比对远端变化；若远端返回 `404/410` 默认保留本地文件（`RemoteDeletedKeepLocal`）；若本地文件已被用户修改则生成 `.conflict.bak` 备份（`ConflictPreserveLocalAndBackup`），绝不静默覆盖毁损用户本地修改。
