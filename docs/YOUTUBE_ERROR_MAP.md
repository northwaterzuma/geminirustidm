# YouTube 管线与 12 类错误分类映射表 (YOUTUBE_ERROR_MAP - P3-T01)

- **模块位置**：`crates/tools/src/{ytdlp_errors,video,proc,components}.rs`
- **12 类规范错误码映射**：
  1. `Network` -> `ErrorCode::NetworkError`（可按预算退避重试 30/60/120s）
  2. `RateLimited` (HTTP 429) -> `ErrorCode::RateLimited`（全站 `youtube` 共享冷却，遵守 `Retry-After`）
  3. `AuthRequired` (私享/会员/年龄限制) -> `ErrorCode::AuthRequired`（转 `AwaitingUser`，不无限重试）
  4. `AuthRejected` (Cookie 失效/401) -> `ErrorCode::AuthRejected`（转 `AwaitingUser` 请求更新会话）
  5. `BotBlocked` (BotGuard / PO Token 挑战) -> `ErrorCode::BotBlocked`（停止重试，提示网络或 PO Token 配置）
  6. `IpBlocked` -> `ErrorCode::IpBlocked`（停止重试）
  7. `GeoBlocked` -> `ErrorCode::GeoBlocked`（停止重试）
  8. `FormatUnavailable` -> `ErrorCode::FormatUnavailable`（重新提取或转 `AwaitingUser(reason=selection)`）
  9. `ExtractorChanged` (nsig/签名解析失败) -> `ErrorCode::ExtractorChanged`（检查已验证组件组合更新）
  10. `DrmProtected` (Widevine/PlayReady/FairPlay) -> `ErrorCode::DrmProtected`（明确拒绝，不伪装成功）
  11. `DiskFull` (`ENOSPC`) -> `ErrorCode::DiskFull`（保留已有分片，停止重试）
  12. `MergeFailed` (FFmpeg 混流异常) -> `ErrorCode::MergeFailed`（保留已下载音视频轨，仅重试本地 Merge 阶段）
