# GitHub Actions 验证结果：CI-43864085628

- 仓库：`northwaterzuma/wbhy4falshv1`
- 分支：`main`
- 提交：`490af44 fix(ui): submit typed url directly and constrain address field width`
- 工作流：RustCatch CI (Build, Test & Contract Verification)
- Run ID：`43864085628`
- 结果：**success**

## Jobs

| Job | 结果 | 时长 |
| --- | --- | --- |
| Contract Schema & Offline Protocol Suite | success | 7s |
| Rust Build & Test (macos-latest) | success | 3m4s |
| Rust Build & Test (windows-latest) | success | 6m9s |
| Rust Build & Test (ubuntu-latest) | success | 4m39s |

## Rust 测试

Windows job 的 `Run Rust Workspace Unit & Integration Tests` 步骤实测汇总：

- `3 + 4 + 20 + 4 + 8 + 9 + 10 = 58` 项 Rust 测试通过
- **58 passed / 0 failed / 0 ignored**
- 其中 `crates/app/tests/ui_actions.rs` 为本次新增：**4 passed / 0 failed**
  - `add_url_prepares_download_info_for_youtube_and_creates_task`
  - `empty_url_reports_error_and_does_not_create_task`
  - `prepare_add_url_action_skips_the_extra_click_when_address_is_present`
  - `menu_actions_open_visible_dialogs`

## 本地检查

- `cargo +stable check --workspace --all-targets`：通过
- `cargo +stable test -p mdm-app --test ui_actions`：本机因 `dlltool.exe: program not found` 无法链接测试二进制；真实测试结果以 GitHub Actions Windows job 为准。

## 证据边界

该 CI 只证明代码可编译、格式/Clippy 检查通过、58 项自动化测试和契约套件通过。它**不**等于 Windows GUI 真机点击验收通过；GUI 真机、浏览器真机、授权 YouTube 在线样本等门槛继续保持 `needs_env`。
