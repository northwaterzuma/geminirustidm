# 正式单扩展打包、安装与共存说明 (EXTENSION_INSTALL - P2-T04)

- **目录位置**：`extension/`（MV3 规范：`manifest.json`, `background.js`, `page.js`, `video.js`, `popup.html`, `popup.js`）。
- **Native Host 绑定**：通过 `rustcatch-cli register-host` 生成 `com.rustcatch.bridge.json`，`allowed_origins` 仅授权 RustCatch 扩展 ID。
- **单扩展独立运行**：干净 Chrome/Edge 仅安装 `extension/` 即可完成两阶段下载接管、网页视频浮条与 HLS/DASH 嗅探，不依赖外部猫抓扩展；若用户同时安装猫抓，可在设置中显式开启 API 桥接而不产生重复接管。
