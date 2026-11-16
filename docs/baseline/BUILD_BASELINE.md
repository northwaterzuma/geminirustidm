# P0-T02 构建与环境基线记录 (BUILD_BASELINE)

## 1. 实际运行环境检查

- **操作系统与内核**：`Linux ace 6.12.105+ #1 SMP Sat Sep 19 19:37:41 UTC 2026 x86_64 GNU/Linux`
- **可用工具链**：
  - `git`：`/usr/bin/git`
  - `python3`：`/usr/bin/python3`
  - `gcc`：`/usr/bin/gcc`
  - `ffmpeg` / `ffprobe`：`/usr/bin/ffmpeg`、`/usr/bin/ffprobe`
- **受阻工具链与网络边界**：
  - `rustc` / `cargo`：当前沙箱容器未预装（`command not found`）。
  - **外部网络**：沙箱容器禁用出站互联网连接（无法直接 `git clone https://github.com/anefzaoui/misterdownloadmanager.git` 或通过 `rustup` 下载 `1.95.0` 工具链）。
- **编译与验证分流策略（遵照用户指令“编译都送到GitHub上编译”）**：
  1. 本地工作区完整构建 `rustcatch/` 全套 Rust 源码工作区（`crates/ipc`、`crates/engine`、`crates/tools`、`crates/app`、`crates/nmhost`、`crates/mcp`、`crates/cli`、`crates/labserver`）、MV3 扩展（`extension/`）及 GitHub Actions 编译发布流水线（`.github/workflows/ci.yml` 与 `.github/workflows/release.yml`）。
  2. 本地沙箱执行确定性故障注入服务器、JSON Schema Draft 2020-12 契约正反例校验、HTTP 分段/恢复协议验证、两阶段接管状态机、媒体策略与 ZIP Range 预览验证，生成真实可复核的 `evidence/` 测试证据。
  3. 需要 `cargo +1.95.0 build/test --workspace`、Windows 10/11 GUI 真机、Edge/Chrome 浏览器进程与在线 YouTube 授权样本的门槛如实标记为 `needs_env`，由 GitHub Actions runner 在推送后自动执行跨平台编译与测试。
