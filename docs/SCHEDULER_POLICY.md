# 多队列、时区/夏令时计划与全局限速策略 (SCHEDULER_POLICY - P4-T01)

- **模块位置**：`crates/app/src/{queue,scheduler}.rs`、`crates/engine/src/rate_limiter.rs`、`crates/tools/src/relay.rs`
- **计划防重**：每个计划触发实例分配唯一 `occurrence_id` 并持久化于 `executed_occurrences`；跨越系统睡眠唤醒、夏令时跳变或应用重启时，同一 `occurrence_id` 严格只触发一次。
- **全局限速覆盖**：原生 HTTP 下载器与外部 `yt-dlp`（通过本地 `127.0.0.1` 受控 `LocalRelaySession` 转发）共享同一个 `SharedRateLimiter` 令牌桶预算。
