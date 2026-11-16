# 上游能力与基线审计模板

复制本文件到产品仓库 `docs/baseline/UPSTREAM_AUDIT.md` 后填写。**当前全部运行验证为 `unverified`；以下已知源码定位只帮助开始审计，不代表测试通过。** 不需要历史聊天即可依据主规格、SOURCE_LOCK 和测试矩阵执行。

## 1. 审计信息

| 字段 | 值 |
|---|---|
| 审计时间与时区 | 待填写 |
| 审计 Agent / 审阅人 | 待填写 |
| 操作系统、版本、架构 | 待填写 |
| 产品工作目录/分支/当前提交 | 待填写 |
| 初始工作区改动 | 待填写，不覆盖用户现有改动 |
| SOURCE_LOCK 文件摘要 | 待填写 |
| 默认 MDM tag | v0.5.2 |
| 预期 MDM commit | `605a0c74e50361afe77bf1721d7172636bd5c57e` |
| 实际 HEAD / commit 对象核验输出 | 待填写 |
| Rust MSRV / edition（声明值） | 1.95 / 2024 |
| 实际 Rust、Cargo、原生构建工具 | 待填写 |
| Cargo.lock 摘要 | 待填写 |
| 组件锁/ComponentSetId | 待填写 |
| 基线总体结果 | unverified |

## 2. 开始审计的顺序

1. 读取实际仓库 AGENTS.md、主规格、SOURCE_LOCK、THIRD_PARTY_REUSE；记录 `git status --short`，保留用户改动。
2. 在对应 checkout 中用 `git cat-file -t <SHA>` 和 `git rev-parse --verify '<SHA>^{commit}'` 验证锁定对象。示意中的 `<SHA>` 必须替换为 SOURCE_LOCK 的完整 SHA，不把占位符原样交给 shell。
3. 记录实际 HEAD、toolchain 和 Cargo.lock；核对版本/tag 的真实内容，尤其发布版与 main 参考快照的差异。不要先更新依赖。
4. 静态确认功能入口、模型、持久化、调用链、现有测试；将“看见代码”与“执行通过”分开记录。
5. 按主规格执行基线构建与适用测试，记录完整命令、退出码、环境与证据路径。网络/真机/凭据缺失只阻塞相应门槛。
6. 每个 REQ 填写差距和下一步。先复用已满足部分，再修改缺失能力；不要为了符合规划路径重复实现已有系统。

## 3. 固定来源核验表

预期 SHA 必须从同包 SOURCE_LOCK 复制，不能填分支名或 tree SHA。已有编制阶段来源核查不替代开发机 checkout 验证。

| source_lock_id | 预期 SHA | 实际对象类型/HEAD | 具体文件及许可已复核 | 运行结果 | 证据 |
|---|---|---|---|---|---|
| mdm_v0_5_2 | `605a0c74e50361afe77bf1721d7172636bd5c57e` | 待填写 | unverified | unverified | 待填写 |
| mdm_main_reference | `1dc204ca92c51377fffbc628a9a4f739dc1a68c8` | 待填写；仅参考 | unverified | unverified | 待填写 |
| cat_catch | `7529f1e6c63c2e1b2e5d4baf5cd0577b8eb4aaba` | 待填写 | unverified | unverified | 待填写 |
| arroxy | `6df64a067f063c53a25c8deaca02e1345ffb1f87` | 待填写 | unverified | unverified | 待填写 |
| fluxdown | `9a5b0d7d5b754c27c2853dd0434494695db18e8d` | 待填写；须为 commit | unverified | unverified | 待填写 |

若只参考一个项目，无需为获得“通过”而运行其整套产品；在适用范围记录 reference-only。复制或移植相关文件仍需核对许可和移植后的测试。

## 4. REQ-01～19 初始矩阵

`静态分类` 可填写 `present / partial / missing / unknown`，初始为 unknown。`运行状态` 只能依据证据填写 `unverified / passed / failed / blocked`。延期、非首发范围单独写排期，不能把它们当测试通过。

| 需求 | 要求摘要 | 优先检查的上游位置（定位提示） | 静态分类 | 运行状态 | 对应验证 | 差距/证据 |
|---|---|---|---|---|---|---|
| REQ-01 | IDM 式窗口、中文、操作、高 DPI | crates/app；公开 IDM 行为基线 | unknown | unverified | TEST-01 | 待填写 |
| REQ-02 | HTTP(S)、Range、动态分段、限速、完整性 | crates/engine 的 download/planner/verify；labserver | unknown | unverified | TEST-02 | 待填写 |
| REQ-03 | 持久化、崩溃恢复、地址刷新、源变化 | engine/state；App 数据保存；tools 视频恢复 | unknown | unverified | TEST-03 | 待填写 |
| REQ-04 | Edge/Chrome、Native Messaging、可靠交接及幂等入队 | extension；crates/nmhost、ipc | unknown | unverified | TEST-04 | 待填写 |
| REQ-05 | 资源列表、HLS/DASH、关联、去重、保护状态 | extension；猫抓 API/候选；tools | unknown | unverified | TEST-05 | 待填写 |
| REQ-06 | YouTube 执行链、错误分类、受控组件更新 | crates/tools；Arroxy MIT 包作参考 | unknown | unverified | TEST-06、06A～E | 待填写 |
| REQ-07 | 格式、音轨、字幕、Shorts、播放列表 | tools 视频选项；App 对应窗口 | unknown | unverified | TEST-07、07A～C | 待填写 |
| REQ-08 | 分类、多队列、计划、配额、有界重试 | app/model、queue、scheduler | unknown | unverified | TEST-08 | 待填写 |
| REQ-09 | 代理、站点连接、Cookie、Basic 认证 | engine 网络设置；extension；tools | unknown | unverified | TEST-09 | 待填写 |
| REQ-10 | 批量、剪贴板、拖放、导入导出、去重 | App 链接/剪贴板/任务入口 | unknown | unverified | TEST-10 | 待填写 |
| REQ-11 | Windows 安装/更新/卸载、日志、迁移、通知 | tools/installer；App 平台适配；组件更新 | unknown | unverified | TEST-11 | 待填写 |
| REQ-12 | MCP 控制、权限、结构化错误、脱敏 | crates/mcp、ipc；统一任务入口 | unknown | unverified | TEST-12 | 待填写 |
| REQ-13 | FTP 模式、代理、恢复 | crates/engine 的 FTP 实现；labserver | unknown | unverified | TEST-13 | 待填写 |
| REQ-14 | Negotiate/NTLM/Kerberos 等高级认证 | 按实际基线查找，不推定已支持 | unknown | unverified | TEST-14 | 待填写 |
| REQ-15 | 站点抓取、过滤/深度、项目恢复、离线改写 | 按实际 App/工具模块查找 | unknown | unverified | TEST-15 | 待填写 |
| REQ-16 | 周期同步、远端变化、覆盖/备份策略 | 队列/调度；按同步实现查找 | unknown | unverified | TEST-16 | 待填写 |
| REQ-17 | ZIP 预览及不支持情形反馈 | 按实际归档/远程预览实现查找 | unknown | unverified | TEST-17 | 待填写 |
| REQ-18 | 其余已确认 IDM 功能与完成动作 | 逐项依照 IDM 基线登记 | unknown | unverified | TEST-18 | 待填写 |
| REQ-19 | macOS/Linux 安装、桥接、桌面能力 | 平台适配、安装器、浏览器注册 | unknown | unverified | TEST-19（逐平台） | 待填写 |

REQ-01～12 是首发范围；REQ-13～18 是最终功能对齐；REQ-19 分平台验收。没有做完的高级项保留差距，不以“使用少”为由删除。REQ-04 的幂等入队不意味着跨浏览器/桌面拥有绝对原子事务；必须验证主规格定义的 prepare/commit/对账。

## 5. 单项审计记录（每个 REQ 复制）

```yaml
requirement: REQ-XX
scope: null
source_lock_id: mdm_v0_5_2
source_commit: 605a0c74e50361afe77bf1721d7172636bd5c57e
source_paths: []
static_classification: unknown
static_evidence: []
existing_behavior: null
required_behavior: null
gap: null
decision: null  # retain | modify | implement | defer
reuse_register_ids: []
tests: []
runtime_status: unverified
environment: null
commands: []
exit_codes: []
evidence_paths: []
not_run: []
blocker: null
next_task: null
reviewer: null
```

同一个 REQ 的离线测试与真机/站点测试分别列结果；离线样例通过不能覆盖尚未运行的在线门槛。部分通过仍记录未运行/失败项，不能把整项写为 passed。

## 6. 构建与运行证据表

| 记录 ID | 平台/架构 | checkout/组件组合 | 命令或操作 | 退出码/观察 | 结果 | 证据路径 |
|---|---|---|---|---|---|---|
| BUILD-01 | 待填写 | 待填写 | 基线依赖元数据/构建 | 未执行 | unverified | 待填写 |
| BUILD-02 | 待填写 | 待填写 | 基线上游测试 | 未执行 | unverified | 待填写 |
| RUN-01 | Windows x64，待确认版本 | 待填写 | 启动、普通文件下载、退出/恢复 | 未执行 | unverified | 待填写 |
| RUN-02 | Edge/Chrome，待确认版本 | 待填写 | prepare/commit、ACK 丢失、Host 重启 | 未执行 | unverified | 待填写 |
| RUN-03 | 待填写 | 待填写 | 授权 YouTube 样本及媒体校验 | 未执行 | unverified | 待填写 |

文件完整性用已知样本 SHA-256；媒体按实际流/容器/时长及播放核对，未知大小不当作零。真实样本记录授权条件、日期、网络和组件版本；日志/截图/导出不得含 Cookie、Authorization、令牌或敏感签名参数。

## 7. 差异、失败与补丁决策

| 差异 ID | REQ/测试 | 预期与实测差异 | 类型（环境/上游缺陷/本地改动/未实现） | 处理路径 | 来源补丁 SHA | 复验状态 |
|---|---|---|---|---|---|---|
| 待创建 | 待填写 | 待填写 | 待判断 | 待填写 | 未选定 | unverified |

上游 main 的修复只能逐项审阅后回移，记录补丁依赖和新引入的构建要求。不允许未经验证替换底座后仍引用旧基线报告。涉及文件正确性、恢复、身份/凭证、浏览器交接的失败阻止对应门槛通过；不相关任务可以继续。

## 8. 许可与复用核对

- [ ] 本机 checkout 与 SOURCE_LOCK 一致，未跟随浮动分支。
- [ ] REUSE_REGISTER 已为每次复制/移植登记具体文件及目标路径。
- [ ] 猫抓按锁定版本 GPL-3.0 登记，没有套用早期 MIT。
- [ ] Arroxy 两个独立包保留 MIT 通知，其他主应用代码按各自许可处理。
- [ ] Flux 自写 AGPL 代码未默认复制；如采用已有明确评估记录。
- [ ] FFmpeg 实际构建许可、组件版本、摘要、签名状态及源码提供信息可追溯。
- [ ] 保留 MDM 原 JSON/任务/队列架构，拟新增文件没有形成重复系统。
- [ ] 图标、字体、截图和测试素材另行登记，不假定仓库根许可覆盖一切。

## 9. P0 审计结束判定

本模板的预填内容不能让 P0 自动完成。审计完成需要：锁定来源核验有输出；实际基线构建/测试有结果；全部 REQ 有静态分类、差距与下一步；许可和依赖未知项被明确记录；证据、TASKS 和 HANDOFF 更新。

若环境阻塞，结论写“审计部分完成，以下门槛仍未验证”，列出具体受阻测试及所需条件，继续无依赖任务。P0 完成也不表示产品发布可用。最终发布按主规格的各项门槛判断。
