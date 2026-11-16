# 猫抓 API 桥接与媒体候选聚合模型 (CAPTURE_MODEL - P2-T02)

- **模块位置**：`crates/app/src/capture.rs`、`crates/tools/src/streams.rs`、`extension/background.js`
- **两阶段演进**：
  - 阶段 A（开发桥接）：按需向经核准的猫抓扩展发送 `chrome.runtime.sendMessage(id, {action: "getData", tabId})`，返回值视为非可信输入校验后标记 `source = "cat_catch"`。
  - 阶段 B（首发正式单扩展）：内置 WebRequest 媒体嗅探与 HLS/DASH 清单识别（`source = "integrated_capture"`），用户仅需安装 RustCatch 单一扩展。
- **`nav_epoch` 代次隔离**：标签页跳转或 SPA 切视频递增 `nav_epoch`，旧代次异步候选直接丢弃并拒绝(`StaleNavigation`)；过滤 YouTube `googlevideo` 临时分片，按 `(url, mime, frame_id)` 去重，绝不因页面标题相同误合并独立音轨与视频轨。
