# P3-T03 Flux 实验性备用适配器许可与技术评估报告 (FLUX_EVALUATION)

- **许可审查结论**：上游 `zerx-lab/FluxDown`（commit `9a5b0d7d5b754c27c2853dd0434494695db18e8d`）根仓库采用 **AGPL-3.0**，`packages/plugin-sdk` 采用 **AGPL-3.0-or-later**。本项目 `RustCatch` 主体采用 **GPL-3.0-or-later**。为避免 AGPL 网络服务传染条款污染默认 GPL 桌面发行物，**不复制任何 FluxDown AGPL 自写代码进入本项目仓库**。
- **技术冗余评估**：FluxDown 的 `youtube-down` 插件虽内嵌 QuickJS 而不调用 `yt-dlp.exe`，但底层仍依赖 `yt-dlp-ejs` 求解签名，与默认 `yt-dlp + EJS + Deno` 路线共享核心失效点，且解析后仍需 `FFmpeg` 混流。
- **工程落地**：`crates/tools/src/flux_adapter.rs` 默认完全关闭（`flux_experimental = false`），调用时由 `check_flux_adapter_gate` 明确拦截，仅保留页面 URL 规则与过滤 `googlevideo` 分片的架构借鉴。
