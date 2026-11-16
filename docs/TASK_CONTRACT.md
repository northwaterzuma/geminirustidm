# 统一任务契约、状态机与持久化恢复规范 (TASK_CONTRACT - P1-T02)

- **模块位置**：`crates/ipc/src/{task_api,protocol,validation}.rs`、`crates/app/src/{model,task_coordinator}.rs`
- **规范状态 (12 种)**：`HandoffPending, Queued, Resolving, AwaitingUser, Running, Finishing, Pausing, Paused, Cancelling, Cancelled, Failed, Completed`。
- **规范阶段 (6 种)**：`Resolve, Select, Transfer, Merge, Verify, Commit`。
- **两层幂等与墓碑机制**：
  - 创建幂等：`(caller_key, idempotency_key)` 绑定唯一 `task_id` 与创建语义摘要 `creation_digest`；相同摘要返回原任务，不同摘要返回 `IdempotencyConflict`。
  - 单次操作幂等：`(caller_key, op, request_id)` 缓存响应；`commit`/`status`/`abort` 引用已有交接而不与 `prepare` 做创建摘要比对。
  - 墓碑 (`tombstones`)：`RemoveRecord` 或 `handoff.abort` 记录墓碑（默认保留 30 天），防止扩展 outbox 延迟重放复活已删除/已中止任务。
