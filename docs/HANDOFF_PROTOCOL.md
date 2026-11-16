# Native Messaging 握手与两阶段浏览器交接协议 (HANDOFF_PROTOCOL - P2-T01)

- **模块位置**：`crates/nmhost/src/lib.rs`、`crates/app/src/task_coordinator.rs`、`extension/background.js`
- **帧与结构边界**：4 字节本机字节序长度前缀 + UTF-8 JSON，双向单帧上限 `262,144` 字节（分配内存前检查），JSON 嵌套深度 `<= 16`，拒绝重复键。
- **四阶段 `hello` 握手**：`negotiate -> enroll -> challenge -> prove`，使用域分隔 `HMAC-SHA256` 常量时间校验与 60 秒一次性 `challenge_id`，未完成 `prove` 前拒绝一切非 `hello` 请求。
- **两阶段接管不变量**：
  1. `handoff.prepare`：仅接受 `GET` + `known_replayable` + 非空 `replayability_evidence_ref`；持久化为 `HandoffPending` 并返回 `durable=true`，零站点网络请求。
  2. `handoff.commit`：扩展确认浏览器原下载已取消（`browser_cancel_confirmed = true`）后提交，协调器 CAS 转为 `Queued`。已成功 commit 的重放优先返回原结果（即使租约已过期或 revision 已推进）。
  3. `handoff.abort`：浏览器取消失败或已完成时调用；仅在收到持久化 `Aborted` 响应后才恢复浏览器下载。若已 commit 则返回 `AlreadyCommitted`。
  4. 租约过期进入 `AwaitingUser(reason=handoff_reconcile)`，由本地 `RecoverHandoff` 显式恢复或放弃，普通 `Resume`/`Retry` 严禁将其直接入队。
