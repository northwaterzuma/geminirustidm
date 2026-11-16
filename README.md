# RustCatch Download Manager

**RustCatch** 是一款以 **Rust (Edition 2024 / MSRV 1.95, `egui`/`eframe` + `Tokio`)** 为主体、界面和操作习惯逐项对齐 **IDM (Internet Download Manager)**、融合 **猫抓 (cat-catch)** 资源嗅探、**Arroxy** 视频工作流与错误分类规则的桌面下载管理器。

## 仓库结构

- `crates/ipc`：V1 浏览器桥接协议、统一任务 DTO、`DecimalU64` 大整数无损序列化、256 KiB 帧与 16 层 JSON 深度校验、4 阶段 `HMAC-SHA256` 握手、Windows 保留字清理与 URL 脱敏。
- `crates/engine`：HTTP/HTTPS `206 / 200 / 416` 严格校验、强弱 `ETag` 区分、`4 MiB` 动态尾部切分与 `generation` 围栏、原子检查点落盘与启动崩溃协调、全局令牌桶限速、FTP/FTPS 断点恢复、SSPI/Negotiate 隔离认证适配层、远程 HTTP Range `ZIP/ZIP64` 中央目录预览器。
- `crates/tools`：受控 `yt-dlp + EJS + Deno + FFmpeg/ffprobe` 进程树编排、Arroxy 12 类错误分类器、`MediaIntent` 格式/音轨/字幕选择器、播放列表分页去重、HLS/DASH 清单解析与 DRM 识别、本地限速转发 `LocalRelaySession`、组件锁 SHA-256 核验器、默认禁用的 `FluxExperimental` 门控。
- `crates/app`：唯一 `TaskCoordinator` 状态机、两阶段浏览器接管（`prepare -> 确认浏览器取消 -> commit / abort`）与 `RecoverHandoff`、`nav_epoch` 候选聚合、多队列与 DST/睡眠感知调度器、四步站点抓取与离线链接改写、周期同步队列、批量/剪贴板/脱敏导入导出、IDM 经典浅色界面（`UI-01..UI-12`）。
- `crates/nmhost`：`com.rustcatch.bridge` Native Messaging 宿主二进制。
- `crates/mcp`：9 项结构化工具的 stdio MCP 服务端（默认无删文件与任意命令权限）。
- `crates/cli`：`rustcatch-cli` 命令行控制与宿主注册工具（标准退出码 `0/2/3/4/5/10`）。
- `crates/labserver`：离线确定性协议与故障注入测试服务。
- `extension/`：Chrome / Edge MV3 正式单扩展（内置两阶段接管、视频浮条与猫抓 API 桥接）。
- `.github/workflows/ci.yml` & `release.yml`：GitHub Actions 跨平台自动编译、测试与打包流水线。

## 在 GitHub 上编译与运行测试

将本源码仓库推送至您的 GitHub 仓库后，GitHub Actions 会自动触发：

1. **离线契约与协议回归套件**：运行 `python3 tests/run_offline_verification.py`。
2. **跨平台 Rust 矩阵编译与单测**：在 `windows-latest` (`x86_64-pc-windows-msvc`)、`ubuntu-latest` (`x86_64-unknown-linux-gnu`) 和 `macos-latest` (`aarch64-apple-darwin`) 上自动运行 `cargo test --workspace` 与 `cargo build --release --workspace`，并产出构建构件压缩包。
