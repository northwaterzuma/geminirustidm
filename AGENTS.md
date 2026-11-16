# RustCatch Agent 执行规则

本文件适用于本开发资料及据此建立的 RustCatch 工程。已有仓库 AGENTS.md 先读取并合并，不无条件覆盖。具体需求和接口以 RUST_DOWNLOAD_MANAGER_SPEC.md 为主；实际用户指令、系统权限优先。

## 必须交付的产品

- Rust 桌面下载管理器，IDM 的经典布局、操作和功能逐项对齐；YouTube 是首发必选。
- MDM 固定发布提交为底座；猫抓捕获、Arroxy 媒体工作流、Flux 解析器设计按明确边界吸收。
- 首发 REQ-01～12；首发后继续 P5，完成 REQ-13～18 的 IDM 差距与 REQ-19 分平台验收。延期不算完成。
- 最终只需要一套自有扩展；猫抓 API 桥接是中间步骤。

## 开始工作

1. 读取 START_HERE.md、完整规格、SOURCE_LOCK.json、TASKS.md 和已有 HANDOFF.md。
2. 查看工作区和 git status，保护已有用户改动；空目录独立 clone，已有工程独立分支/工作树。
3. 按精确 commit 复现构建；保留 Cargo.lock 和上游 patch/rev。声明 MSRV 是 1.95，edition 2024；不要为绕错降低版本声明。
4. 完成 P0 的源码/许可、工具链、UI 和测试基线，选第一个无阻塞任务实际实施。

## 架构硬约束

- 主体 egui/eframe + Tokio + Rust；第一方核心业务、下载、任务、IPC、Native Host、CLI、MCP 不转移到脚本服务。
- 扩展允许最小 JS/TS；yt-dlp、FFmpeg/ffprobe、Deno/EJS 是经校验的外部工具，用户无需自行配置 Python/Node。
- 复用一套 App/coordinator、任务、队列、存储与下载引擎。不新增第二应用或为整齐更换数据库。
- 默认 YouTube 由 yt-dlp 端到端下载与后处理，Rust 管版本、进程、计划、限额、恢复和结果；不要一次拿直链就永远交给普通 HTTP 下载。
- 上游已有功能先测试再补差距；不得把 README 宣称直接标为验收通过。

## 接口与正确性

- TaskState、phase、request_id/idempotency_key 和错误代码遵循主规格及 contracts/；Rust 类型是实现后的唯一契约源，导出 Schema/TS 并与样例回归。
- browser prepare 阶段零站点网络副作用；收到 durable ACK 后确认浏览器取消，再 commit。commit/abort 竞争由 coordinator CAS；只收到 durable Aborted 后才恢复浏览器。
- 同可信调用方幂等键至多一个任务，不承诺跨进程原子 exactly-once。旧 attempt 事件不得修改新任务。
- Native Messaging 双向单帧最多 256 KiB，凭据材料最多 32 KiB，候选最多100条/页；字节/深度/重复键/调用身份还需运行时校验。
- 不确定可重放的请求保留浏览器；格式、身份或源验证器变化不得拼接旧字节。Range、最终提交、恢复、取消必须有故障测试。
- 暂停、取消、移除记录、删除文件是不同操作；完成必须在传输、后处理、验证、最终文件提交及持久化之后。
- 工具 argv 不经 shell 拼接；全部受控组件使用锁定路径，不能悄悄回退 PATH 或直连。取消要覆盖子进程树。
- Cookie、授权头、签名 URL、代理密码不进日志、导出、MCP 或普通任务 JSON。新任务只使用自己授权的本地上下文。

## 实施与验证

每项任务先确认实际代码路径，列出最小可观察结果，再写实现和必要测试。关键测试验证结果和故障，不镜像内部实现。不用 TODO、假数据、吞错误或静态截图冒充完成。避免没有功能收益的整仓重构。

离线 CI、真实 Windows/浏览器、真实授权 YouTube 样本、安装与迁移分别留证。不能运行就写 not_run/needs_env 并说明具体缺项，不得计入通过。只重跑受影响的测试与必要发布门槛；不要无目的扩大测试。

引用或移植代码记录来源 commit、路径、许可、修改及测试。默认 GPL 开源复用；Flux AGPL 自写代码未经组合审查不默认复制；FFmpeg 按实际二进制构建配置登记。自有产品身份独立，保留上游版权。

## 多 Agent 协作

一个人负责共享 DTO、状态、协议；其余按 UI、engine、browser、media、release/test 分工。记录文件所有权，避免并发修改 app.rs、Cargo.lock、Schema。使用独立分支/工作树；集成人负责真实跨模块闭环，不能仅合并编译。

## 续接与完成

每完成一个任务或上下文即将不足时，更新 TASKS.md、tasks.json 和 HANDOFF.md：真实路径/分支/commit、改动、测试命令与结果、证据、未运行项、决定、阻塞及下一条可执行动作。不要丢弃未提交修改。新 Agent 先读交接和 git status，再继续，不重新规划整个项目。

缺环境、账号、网络或发布凭据时只暂停相应门槛，继续无依赖任务。公开发布、推送、上架、签名凭据使用按实际用户授权执行；本文件不授予外部操作权限。最终回复写清完成了哪个范围、通过哪些实际测试、仍缺什么，不把资料生成或编译成功叫成产品完成。
