# GitHub Actions 验证结果：v0.1.3-components

- 仓库：`northwaterzuma/wbhy4falshv1`
- 预发布标签：`v0.1.3-components`
- 标签指向提交：`96f48dcaf637a553af270c27bfb2626a3ca38e19`
- 提交说明：`feat(release): resolve and bundle verified media components`
- 发布工作流：RustCatch Release & Component Lock Resolver
- 正式发布 Run ID：`43876092254`
- 结果：**success**，其中 `Publish to GitHub Release` 步骤 success
- Release 页：https://github.com/northwaterzuma/wbhy4falshv1/releases/tag/v0.1.3-components
- Release 类型：prerelease

## 本轮代码变化

- `.github/workflows/release.yml`：在 Windows 发布构建中从官方 release 下载 yt-dlp、Deno、FFmpeg，计算实际 SHA-256，生成 `resolved-COMPONENT_LOCK.json`，并把组件复制到随包 `components/` 目录。
- `crates/app/src/runner.rs`：运行时优先读取 exe 同目录的 `COMPONENT_LOCK.json`；输出模板去掉扩展名，避免 yt-dlp 自动补扩展时出现双重扩展。
- `crates/tools/src/proc.rs`：受控 argv 增加 `--js-runtimes deno:<locked deno path>`；`--proxy` 仅在本地 relay URL 非空时加入。
- `crates/tools/tests/streams_and_proc.rs`：增加 Deno 运行时 argv 断言。

## 主 CI 验证

- Run ID：`43873418303`
- 触发：push 到 `main`
- 结果：**4/4 jobs success**

| Job | 结果 | 时长 |
| --- | --- | --- |
| Contract Schema & Offline Protocol Suite | success | 11s |
| Rust Build & Test (ubuntu-latest) | success | 4m46s |
| Rust Build & Test (windows-latest) | success | 4m35s |
| Rust Build & Test (macos-latest) | success | 3m42s |

三平台 `Run Rust Workspace Unit & Integration Tests` 汇总均为 **60 passed / 0 failed / 0 ignored**。

## 发布前试跑

- Run ID：`43875366720`
- 触发方式：`workflow_dispatch`，ref `main`
- 结果：**success**
- `Resolve and Verify Media Components`、`Assemble dist/ Portable & Extension Package`、`Upload Release Bundle` 全部 success。
- `Publish to GitHub Release` 按设计跳过（非 tag 触发），用于避免先发布坏包。

试跑包内组件锁检查结果：

- `status = resolved_verified`
- `component_set_id = ytdlp-2026.10.26-deno-v2.9.8-ffmpeg-n9.0`
- `platform_triple = x86_64-pc-windows-msvc`
- 包内实际组件 SHA-256 与组件锁逐项一致。

## 正式发布资产

资产名：`rustcatch-win64-portable-unsigned-test.zip`

| 项目 | 值 |
| --- | --- |
| GitHub 报告 digest | `sha256:a5b5e29e2534fdf168b666a3ef2d6f72f7ed2354e2a82fe528dd911af983f6fa` |
| 本地下载后复算 SHA-256 | `a5b5e29e2534fdf168b666a3ef2d6f72f7ed2354e2a82fe528dd911af983f6fa` |
| 文件大小 | 203,283,038 bytes |

两者一致，说明资产上传后内容未被破坏。

## 包内组件与哈希

正式包内 `COMPONENT_LOCK.json` 实测：

- `status = resolved_verified`
- `component_set_id = ytdlp-2026.10.26-deno-v2.9.8-ffmpeg-n9.0`
- `resolved_at_utc = 2026-11-16T13:32:29.9577956Z`

| 组件 | 版本 | 包内路径 | 实测 SHA-256 |
| --- | --- | --- | --- |
| yt-dlp | 2026.10.26 | `components/yt-dlp/yt-dlp.exe` | `f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d` |
| yt-dlp-ejs | bundled in yt-dlp | `components/yt-dlp/yt-dlp.exe` | `f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d` |
| Deno | v2.9.8 | `components/deno/deno.exe` | `924397800533d3eb0ed0c889b20ad70e18b970d93acd8135bc043f425e528266` |
| FFmpeg | n9.0 | `components/ffmpeg/ffmpeg.exe` | `9886c618fcbb33d42df41f3cc102d495a90473e66b7180a63e02b0c3015d93d6` |
| ffprobe | n9.0 | `components/ffmpeg/ffprobe.exe` | `482155f54c6a578e28ad173e9cebc973de63b32eaf4403e0abc9a4779405c4d9` |

包内还存在 `rustcatch.exe`、`rustcatch-cli.exe`、`rustcatch-mcp.exe`、`rustcatch-nmhost.exe`、`extension/`、`LICENSE`、`THIRD_PARTY_NOTICES` 和 `config.toml`。

## 行为边界

- 用户上一轮日志中的 `unresolved_do_not_execute / ToolUnavailable` 根因已在本包中解决：发布包不再使用仓库内未解析的锁，而是使用由 GitHub Actions 生成并随包分发的 `resolved_verified` 锁。
- 这证明组件闸门输入已经齐备、发布链路真实产出可用包；**不等于在线 YouTube 下载验收通过**。
- 真实下载是否成功仍需在 Windows 真机解压运行 `rustcatch.exe`，输入 YouTube 地址并观察日志；TEST-06A/B/C/D 等在线样本门槛仍保持 needs_env，直到用户真机确认。
