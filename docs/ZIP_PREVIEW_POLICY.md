# P5-T03 HTTP Range 远程 ZIP/ZIP64 预览策略 (ZIP_PREVIEW_POLICY)

- **模块位置**：`crates/engine/src/zip_preview.rs`
- **按需 Range 预览 (`parse_zip_tail_preview`)**：仅读取文件尾部 Range 定位 EOCD (`0x06054b50`) 与中央目录 (`0x02014b50`)，无需下载完整巨型压缩包。
- **安全边界**：限制中央目录大小 `<= 8 MiB`、条目数 `<= 10,000`；分卷压缩包返回明确 `FormatUnavailable`；标记加密位 (`flags & 0x1`)；拦截 `..`、绝对路径、`:` 与压缩比 `> 200:1` 的疑似解压炸弹 (`PermissionDenied`)。
