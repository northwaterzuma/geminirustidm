# HTTP 下载正确性与恢复能力规范 (HTTP_CAPABILITIES - P1-T01)

- **模块位置**：`crates/engine/src/{http_validator,planner,checkpoint,download}.rs`
- **RFC 9110 Range 验证 (`evaluate_range_response`)**：
  1. `206 Partial Content`：严格校验 `Content-Range: bytes start-end/total` 的 `start == req_start`、`end + 1 <= req_end_exclusive`、`total == baseline_validator.total_bytes`，且实际响应体字节数等于 `end + 1 - start`。合法短响应自动将剩余尾部 `[end + 1, req_end_exclusive)` 重新调度。
  2. `200 OK`（服务器忽略 Range）：若 `req_start > 0` 或任务已有非零偏移已提交分段，立即拒绝写入并报错 `ResourceChanged`（绝不把完整响应体写入非零偏移）；仅在初始偏移 0 时安全降级为单流下载。
  3. `416 Range Not Satisfiable`：解析 `bytes */N`，仅当 `N == baseline_validator.total_bytes` 且所有半开区间 `[0, N)` 完整覆盖无空洞时进入 `Verify` 阶段，否则报错 `ResourceChanged`。
  4. **强/弱验证器**：区分强 ETag 与 `W/` 弱 ETag；仅强 ETag 用于 `If-Range`，且要求 `Content-Encoding: identity`。
- **动态尾部切分 (`SegmentPlanner::split_largest_tail`)**：最小工作块 `4 MiB`；仅切分未提交尾部 >= `8 MiB` 的分段，切分时递增 `generation` 围栏，旧 worker 迟到写入因 generation 不匹配被拒绝。
