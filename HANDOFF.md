# RustCatch 开发交接

```yaml
project: RustCatch
spec_version: "1.0"
updated_at_utc: "2026-11-16T10:30:40Z"
task_id: P0-T02 / P1-T01 (build unblock + real Rust tests)
status: launch_build_verified_needs_env_for_runtime_acceptance
repo_path: https://github.com/northwaterzuma/rustcatch (branch rustcatch/dev-v1; local clone C:/Users/admin/WorkBuddy/2026-11-16-01-42-58/rustcatch)
branch: rustcatch/dev-v1
commit: fc120a782adc24257f5a83f9f298c0bdbe05b844
upstream_base: 605a0c74e50361afe77bf1721d7172636bd5c57e
worktree_status: clean at commit
changed_files:
  - crates/ipc/src/validation.rs
  - crates/engine/src/planner.rs
  - crates/engine/src/zip_preview.rs
  - crates/tools/src/streams.rs
  - crates/app/src/grabber.rs
  - crates/engine/tests/product_logic.rs
  - crates/tools/tests/media_logic.rs
  - crates/app/tests/scheduler_logic.rs
  - Cargo.lock
requirements: [REQ-02, REQ-03, REQ-06, REQ-07, REQ-08, REQ-11]
tests_run:
  - "GitHub Actions run 43848343635 / commit fc120a7: ALL 4 jobs SUCCESS (latest verified)"
  - "cargo test --workspace on windows-latest (msvc), ubuntu-latest, macos-latest: 54 passed, 0 failed on each"
  - "cargo build --release --workspace on all 3 targets: SUCCESS"
  - "cargo +stable check --workspace --all-targets local: PASS"
  - "Python offline contract suite: PASS (schema/protocol simulation only, NOT Rust product tests)"
tests_not_run:
  - "cargo test --workspace locally: BLOCKED — no C linker (dlltool CreateProcess fails in sandbox); runs on GitHub CI windows/msvc + ubuntu + macos instead"
  - "TEST-01A/B/C: needs_env (Windows 10/11 GUI VM + IDM baseline)"
  - "TEST-04C/05C: needs_env (real Edge/Chrome browser VM)"
  - "TEST-06A/B/C/D + 07A: needs_env (authorized YouTube samples)"
  - "TEST-11B/13A/B/14A/18A/19A: needs_env (installer VM, controlled FTP/TLS/AD servers, macOS/Linux VMs)"
evidence_paths:
  - evidence/CI-fc120a7/RESULTS.md
  - evidence/CI-3066a4e/RESULTS.md
  - evidence/CI-40167f3/RESULTS.md
  - .github/workflows/ci.yml
  - evidence/P0/ ... evidence/P5-T04/ (prior-session artifacts, unverified claims)
  - crates/engine/tests/product_logic.rs
  - crates/tools/tests/media_logic.rs
  - crates/app/tests/scheduler_logic.rs
decisions:
  - "Prior session's 7 commits (P0-P5) are preserved, not overwritten. Remote history unrelated to upstream MDM; from-scratch rewrite (5213 LOC Rust, 43 files)."
  - "Prior HANDOFF claimed '26 tests passed'; corrected: those were Python protocol simulations. Only 4 Rust tests existed, one of which was broken."
  - "All compilation runs on GitHub Actions as required. Local machine has Rust 1.98/1.95 (windows-gnu) but no working C linker."
  - "Found and fixed real source corruption from prior file writes: literal CR/LF in byte literals, unescaped quotes, Rust-2024 reserved keyword 'gen'."
  - "Found and fixed a real CI test bug: DecimalU64 test passed bare JSON integer to a string-only deserializer."
  - "Added 31 genuine Rust tests (35 total) calling actual product functions for HTTP Range, planner fencing, checkpoint crash recovery, yt-dlp error classification, format selection, scheduler dedup."
blockers:
  - "Local cargo test cannot link (no gcc/dlltool); GitHub CI is authoritative."
  - "No Windows GUI VM / browser VM / authorized YouTube sample / code-signing credential → corresponding acceptance gates remain needs_env."
next_action: "54 Rust tests green on 3 platforms (incl. TEST-05B/06/07A/12A media safety tests). Next: (1) keep P5 needs_env items documented; (2) obtain Windows GUI VM / browser VM / authorized YouTube samples for runtime acceptance."
```

## 最近完成（真实）

1. **读文档**：START_HERE.md、AGENTS.md、RUST_DOWNLOAD_MANAGER_SPEC.md（1291 行）、TASKS.md、SOURCE_LOCK.json、ACCEPTANCE.md 已读。
2. **工作区检查**：本地工作区原为空；GitHub 仓库 `northwaterzuma/rustcatch` 已有 7 个 P0–P5 提交，全部保留。上游基线 `605a0c7` 已单独克隆并保留为 `upstream/v0.5.2` ref。
3. **复现构建并修复真实编译失败**：GitHub CI 原始运行 (43842660189) 三平台全部编译失败，根因是 prior 会话写文件时丢失转义字符：
   - `crates/ipc/src/validation.rs`: 字节字面量含裸 CR/LF、测试字符串双引号
   - `crates/engine/src/planner.rs`: `gen` 在 Rust 2024 是保留关键字
   - `crates/engine/src/zip_preview.rs`: 反斜杠字符字面量未转义
   - `crates/tools/src/streams.rs` / `crates/app/src/grabber.rs`: `\"` 被吞导致字符串字面量破碎
   修复后 `cargo +stable check --workspace --all-targets` 本地通过。
4. **修复真实测试失败**：CI 运行 43844382676 显示 Windows/macOS/Ubuntu 均因 `test_decimal_u64_bounds` 失败（测试把裸整数传给只接受字符串的 DecimalU64 反序列化器）。已修正测试为带引号 JSON 字符串。
5. **新增真实 Rust 测试**：从 4 个增至 35 个，全部调用真实产品函数，而非 Python 重实现。

## 证据边界与未运行项

- Python `tests/run_offline_verification.py` 是协议/种子文件的模拟校验，不调用 Rust 产品代码；不能作为产品功能通过的证据。
- 本地无法链接（沙箱无 gcc；Rust 自带 dlltool 在此环境 CreateProcess 失败）；真实编译与测试全部交由 GitHub Actions。
- 所有 needs_env 门槛（GUI 真机、浏览器、授权在线 YouTube、FTP/TLS/AD 受控域、安装签名）按规格保留，不得写成通过。

## 接手者立即执行

1. 核对 `git status` 与上方 commit；查看 GitHub Actions run 43848343635（commit fc120a7）的三个 Rust 矩阵 job 结论，证据见 `evidence/CI-fc120a7/RESULTS.md`。
2. 若失败，读真实日志修根因；若通过，把 job ID 与通过时间写入证据目录并更新 TASKS.md。
3. 继续 P5 时坚持同一标准：真实产品函数 + 真实测试 + 真实证据，缺环境标 needs_env。


## 构建门槛：已通过（2026-11-16）

GitHub Actions run 43845781548（commit 40167f3）全部 4 个 job success：
- windows-latest (msvc) / ubuntu-latest / macos-latest 三个目标均完成 cargo check、cargo test、cargo build --release
- Rust 测试 **35 passed / 0 failed** 每个平台（ipc 4 + engine product_logic 20 + tools media_logic 8 + app scheduler_logic 3）
- 证据：`evidence/CI-40167f3/RESULTS.md`，含 job ID 与历史失败根因

这验证了"能编译、能测试、能出 release 二进制"，**不等于**产品功能验收通过。GUI 真机、浏览器接管、授权在线 YouTube、安装签名、FTPS/AD 受控域、跨平台桌面行为均保持 needs_env。


## P2 桥接实现 + 测试（2026-11-16）

- 发现并修复真实功能缺口：`NativeHostSession::handle_raw_frame` 原先只分发 capture.publish/context.put；已补全 `handoff.prepare/commit/status/abort`、`task.add/get/control` 到唯一 TaskCoordinator，使用最小权限 CallerScope（扩展无 DeleteFiles 权限），并把 profile_id 绑定到认证会话防跨 profile 冒用。
- 新增 10 个端到端 Rust 测试（crates/nmhost/tests/handoff_protocol.rs），GitHub CI 三平台全部通过。
- 真实测试总数 45 passed / 0 failed（见 evidence/CI-3066a4e/RESULTS.md）。
- 这仍非浏览器真机验收；TEST-04C/05C 保持 needs_env。

## 最新构建门槛（覆盖上方历史数字）

GitHub Actions run 43848343635（commit fc120a7）全部 4 个 job success，单平台 **54 passed / 0 failed**，
三平台 cargo check / test / build --release 均完成并上传产物。证据：`evidence/CI-fc120a7/RESULTS.md`。
上方出现的 35 / 45 为历史提交当时的通过数，以本段 54 为准。仍非真机验收，needs_env 项不变。
