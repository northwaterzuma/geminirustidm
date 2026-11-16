# 第三方来源复用登记与实施规则

本文件用于执行 P0-T02，并作为 `docs/baseline/REUSE_REGISTER.md` 的初始内容。**当前只完成来源静态核查；没有任何产品构建、运行或测试通过结论。** 提交与许可以同包 `SOURCE_LOCK.json` 为准，需求与验收以主规格和 TASKS.md 为准。

默认路线：GPL 开源的 Rust 主体，fork MDM，选择性移植媒体规则，先通过 API 对接猫抓。不得引入第二个 Electron/GPUI/Tauri 主界面、Node 常驻产品服务、独立任务队列或数据库。浏览器平台必要的 JS/TS、受控 yt-dlp/FFmpeg/Deno 不改变主体 Rust 的要求。

## 1. 源码锁定与复用分类

| 来源 ID | 固定 commit | 用途 | 锁定记录中的许可 |
|---|---|---|---|
| `mdm_v0_5_2` | `605a0c74e50361afe77bf1721d7172636bd5c57e` | 默认开发底座，v0.5.2 | GPL-3.0-or-later |
| `mdm_main_reference` | `1dc204ca92c51377fffbc628a9a4f739dc1a68c8` | 只选择性回移补丁 | GPL-3.0-or-later |
| `cat_catch` | `7529f1e6c63c2e1b2e5d4baf5cd0577b8eb4aaba` | 开发期外部 API，首发整合为单扩展 | GPL-3.0 |
| `arroxy` | `6df64a067f063c53a25c8deaca02e1345ffb1f87` | 工作流与测试参考 | 主应用 GPL-3.0-or-later；两个独立包 MIT |
| `fluxdown` | `9a5b0d7d5b754c27c2853dd0434494695db18e8d` | resolver 设计及后置适配器 | 根目录 AGPL-3.0；SDK AGPL-3.0-or-later |

FluxDown 上述 SHA 已核对为 **commit**，不是 tree；开发机仍须验证 checkout 的对象类型与 SHA。禁止用分支最新内容代替固定提交；路径不存在、内容或许可不一致时记录差异，不能静默改跟 main。

复用类型统一登记为：

- `retain`：保留 MDM 原模块，最小修改；保留版权、许可、历史和依赖固定信息。
- `port`：翻译或改造来源代码到 Rust；仍登记来源及许可，不能因换语言声称独立原创。
- `api`：通过文档化接口互操作，不复制实现；原扩展如随产品分发，另登记其分发义务。
- `reference`：参考行为、契约和故障场景，自行实现；复制代码、测试、正则或素材时改记 `port` 并复核许可。
- `optional_review_required`：尚未批准进入默认源码/发行物；先完成对应许可与工程评估。

“移到子进程”“隐藏入口”或“默认关闭功能”不自动消除来源许可义务。

## 2. MDM：保留主程序与下载系统

下表均为上游已有模块定位；具体文件在固定发布提交中复核。`streams.rs` 等新能力如只存在于 main 参考快照，必须单独回移登记，不冒充发布基线已有。

| 登记 ID | 上游路径 | 推荐方式与职责 | 本地处理 | 对应测试 |
|---|---|---|---|---|
| MDM-01 | `crates/app/src/app.rs`、`model.rs`、`queue.rs`、`scheduler.rs` | retain：主窗口、任务目录、队列、计划 | 保留单一 App；用真实任务驱动原有 egui 窗口 | TEST-01、03、08、10 |
| MDM-02 | `crates/engine/src/download.rs`、`planner.rs`、`state.rs` | retain：HTTP/FTP、分段、限速、恢复 | 不重写普通下载内核；保留原子 JSON/分段检查点 | TEST-02、03、09、13 |
| MDM-03 | `crates/tools/src/video.rs`、`proc.rs` | retain：视频与辅助进程生命周期 | 默认 YouTube 仍由 yt-dlp 端到端传输；Rust 统一管理，不另建视频队列 | TEST-06、07、09 |
| MDM-04 | `extension/manifest.json`、`background.js`、`page.js`、`video.js` | retain：MV3、浏览器接管、媒体入口 | 增加猫抓 API 适配与主规格定义的可靠交接 | TEST-04、05、09 |
| MDM-05 | `crates/nmhost`、`crates/ipc`、`crates/mcp`、`crates/cli` | retain：浏览器桥与所有控制入口 | 统一调用现有任务服务，权限由服务端落实 | TEST-04、11、12 |
| MDM-06 | `crates/labserver` | retain：受控服务器与故障注入 | 补充真实协议失败样例；不把模拟结果当真实站点成功率 | TEST-02～06、09、13 |

保留 `Cargo.lock`、固定 git rev、patch、作者归属和 `ATTRIBUTION.md`。先在 Rust 1.95 / edition 2024 复现发布基线；禁止 blanket `cargo update` 或替换整套 GUI 库来掩盖构建失败。fork 身份、更新地址、扩展 ID、Native Host 名、数据目录及签名公钥按主规格统一更换。

## 3. 猫抓：先接口，后选择性代码整合

| 登记 ID | 上游路径/接口 | 推荐方式 | 本项目目标与验收 |
|---|---|---|---|
| CAT-01 | 官方 `getData(tabId)` API；`js/background.js` 的 `onMessageExternal` | api，中间里程碑 | 用户打开当前标签资源面板时读取；缺失/禁用时回退 MDM 自有捕获。TEST-04、05 |
| CAT-02 | `js/background.js`：webRequest 监听、`findMedia`、候选构造 | reference；统一扩展阶段才决定 port | 借鉴 MIME/URL 判断、请求关联、资源列表；不搬入独立下载队列。TEST-05、09 |
| CAT-03 | `js/init.js`：规则与默认配置 | reference / 选择性 port | 明确版本化规则，保留域名、签名参数和媒体角色差异，防止误去重。TEST-05 |
| CAT-04 | `manifest.json`、`CHANGELOG.md` | reference | 复核权限与版本边界；不能把旧版支持或宣传当当前能力。TEST-04、05 |

开发桥接不修改猫抓；首发必须完成单一自有扩展。其返回值作为非可信候选验证，不向外部扩展请求全部标签资源。当前快照按 GPL-3.0 登记，不引用早期 MIT 许可。猫抓 CHANGELOG 记录过移除 YouTube 支持；YouTube 首发能力由受控 yt-dlp 路线负责，不能依赖“嗅探到播放分片”替代完整解析。

官方 API 定位：[extension-api](https://cat-catch.94cat.com/docs/extension-api)。API 文档是定位依据；实际能力和字段须在锁定扩展上验证，不能只读网页后标记通过。

## 4. Arroxy：移植媒体规则，不移植 Electron 产品

以下路径均相对 Arroxy 仓库。两个 MIT 包已核对各自 LICENSE，不能与主应用许可混写。

| 登记 ID | 上游路径 | 方式/许可 | 迁入现有 Rust 系统的内容 | 对应测试 |
|---|---|---|---|---|
| ARR-01 | `packages/yt-dlp-bridge/src/workflow.ts`、`extractorArgs.ts`、`parsers.ts`、`progress.ts`、`redaction.ts`、`tests/` | port，MIT | 结构化参数、单一 extractor 参数合并、进度/结果解析、脱敏；移入 `mdm-tools`，不用 Node 包装服务 | TEST-06、07、09、12 |
| ARR-02 | `packages/ytdlp-errors/src/classifier.ts`、`patterns.ts`、`kinds.ts`、`tests/fixtures/yt-dlp-stderr/` | port，MIT | 错误种类与样例；迁移时检查 Rust 正则语义，不能只改语法而不跑分类回归 | TEST-06、09 |
| ARR-03 | `src/shared/mediaIntent.ts`、`ytdlp/formatSelector.ts`、`ytdlp/formatSorter.ts` | 选择性 port，GPL-3.0-or-later | 画质/兼容预设、音轨/容器、降级规则；映射现有视频窗口。选择语法只支持本产品使用的子集 | TEST-07 |
| ARR-04 | `src/shared/autoRetry.ts`、`src/main/services/download/QueueAutoRetry.ts`、`QueueSiteHold.ts` | reference / port，GPL-3.0-or-later | 有预算退避、站点冷却、重启后重试时间恢复；放进 MDM 原队列。合并错误只重试本地阶段 | TEST-03、06、08 |
| ARR-05 | `src/main/services/cookiesResolver.ts`、`exploreEscalation.ts`、`ExploreSessionService.ts` | reference / port，GPL-3.0-or-later | 单一身份来源、必要时登录、临时 Cookie 清理、代理一致性；复用用户浏览器上下文，不引入 Explore 浏览器 | TEST-06、09、12 |
| ARR-06 | `src/main/services/phases/mediaRequest.ts`、`src/shared/videoPart.ts`、`src/main/services/phases/SubtitleOnlyPhase.ts` | reference / port，GPL-3.0-or-later | 字幕、区间下载、SponsorBlock 和输出处理的能力边界；只在主规格对应任务纳入范围后实施 | TEST-07，附功能专项样例 |
| ARR-07 | `src/main/services/binary/RuntimeBinaryIndexService.ts`、`RuntimeBinaryTrust.ts`、`src/main/services/ytDlpJsRuntime.ts` | reference，GPL-3.0-or-later | 依赖清单校验、缓存兜底与运行时探测；扩展 MDM 原组件管理器，不再做第二更新器 | TEST-06E、11 |

ARR-05 的 YouTube Cookie 域过滤可借鉴，但通用网站 scope 应进一步收紧到任务需要的域。ARR-07 的 `ELECTRON_RUN_AS_NODE` 依赖 Electron，不可直接搬入 Rust 主体；本项目按组件锁使用 Deno。Arroxy 的隐藏浏览器 PoT 链路不进入默认实现。所有新增登录态使用遵守用户明确选择，不能自动扫描或切换其他账户。

## 5. FluxDown：先参考契约，AGPL 自写代码不默认复制

以下 `youtube-down/` 均指 `packages/plugin-sdk/examples/youtube-down/`。路径已经静态定位，但不是本项目运行通过的证据。

| 登记 ID | 上游路径 | 首期处理 | 后期可评估能力/验收 |
|---|---|---|---|
| FLUX-01 | `youtube-down/README.md`、`manifest.json`、`rules.json` | reference | 页面 URL 提交、忽略 googlevideo 播放分片、元数据规则；TEST-05、07 |
| FLUX-02 | `youtube-down/src/main.js`、`innertube.js`、`clients.js`、`playability.js`、`playlist.js` | reference；复制需 optional_review_required | resolver 输入/输出、站点错误与列表模型；共享 TEST-06/07 的同一套样本 |
| FLUX-03 | `youtube-down/src/formats.js`、`cookies.js`、`player.js` | reference；复制需 optional_review_required | 格式组合、请求身份、播放器刷新与求解缓存；TEST-06、07、09 |
| FLUX-04 | `native/engine/src/plugin/runtime.rs`、`quickjs.rs`、`bridge.rs`、`market.rs` | reference；不引入另一桌面架构 | 若实现实验适配器，用 Rust 宿主限制网络、文件系统、时间、内存与取消；增加隔离专项测试 |
| FLUX-05 | `youtube-down/vendor/yt-dlp-ejs/LICENSE`、`yt.solver.lib.min.js`、`yt.solver.core.min.js` | 单独组件核验，非默认照搬 | vendor 的 Unlicense 与文件头 ISC/MIT 等声明各自保留；锁定来源及版本，TEST-06E |

主规格默认排期中，Flux 自写 AGPL 代码不复制进 GPL 主体。实施 P3-T03 前登记具体文件、组合方式与许可结论；若选择采用，需要同步调整对应发行说明、源码交付和许可安排。通过外部进程或功能开关隔离，不应被当作自动豁免。

Flux 的插件不调用 yt-dlp 可执行文件，但仍依赖 QuickJS 与 yt-dlp-ejs；与默认路线具有共同失效点。其解析阶段无外部 FFmpeg 依赖，不等于最终音视频合并无需 FFmpeg。不开启 GPUI 主界面、不导入 Flux 队列/数据库，不把备用路径宣传为已证明更稳定。

## 6. 既有路径与拟新增路径

上表源路径只代表上游位置。拟在本项目新增的 `crates/app/src/capture.rs`、`task_coordinator.rs`、`crates/tools/src/media_policy.rs`、`crates/ipc/src/task_api.rs` 是设计候选；先检查基线已有等价代码，再决定扩展或提取。不得为了匹配文件名新增重复架构。

每项 port 在 `REUSE_REGISTER.md` 记录最终目标路径、提交、差异和测试；若采用其他文件名，更新登记与主规格映射。保留一个任务目录、一个协调器、一套队列、一个组件更新器；继续原子 JSON 持久化，不无依据迁移 SQLite。

## 7. 组件二进制与许可落实

yt-dlp、EJS、Deno、FFmpeg/ffprobe 及未来 QuickJS 的准确版本、平台、架构、来源 URL、大小、摘要、签名验证结果与许可放入 `COMPONENT_LOCK.json`。版本未知或摘要未核对时使用 null，禁止执行未验证组件。

FFmpeg 必须针对**实际分发构建**审查：

1. 核对下载包的 LICENSE、版权声明、来源和构建配置；安全验证通过后记录 `-version` / `-buildconf` 输出。
2. 区分 LGPL/GPL 构建、静态/动态组件及所启用库的额外条款；不能仅凭“使用 ffmpeg 命令行”判定无需处理。
3. 检出 `--enable-gpl`、`--enable-nonfree` 或其他影响分发的配置时明确登记并处理；未经许可确认的构建不得进入发行包。
4. 随发行物保留要求的通知及源码提供方式，精确对应所分发版本。ffprobe 与 ffmpeg 应属于同一已验证组件组合。

发行前逐项落实，初始全部未完成：

- [ ] 所有已复制/移植文件登记来源完整 SHA、路径和实际许可。
- [ ] MDM、猫抓、Arroxy 版权与许可保留；两个 MIT 子包分别列明，不遗漏测试素材。
- [ ] Flux 自写代码未混入默认发行物；如采用，已有明确组合许可与交付结论。
- [ ] vendor/EJS/QuickJS/FFmpeg 及各自捆绑第三方声明完整。
- [ ] 提供 GPL 对应源码、修改说明及构建所需材料；发行物与源码提交对应。
- [ ] 更新身份、图标、品牌与安装器资源归属明确；不复制 IDM Logo、私有代码或激活系统。
- [ ] 组件清单可追溯；哈希检查与签名验证分别记录，不混为一谈。
- [ ] 所有复用项绑定测试；静态存在不能标为运行通过。

## 8. 逐项登记模板

```yaml
reuse_id: null
source_lock_id: null
source_commit: null
source_paths: []
source_license: null
mode: null  # retain | port | api | reference | optional_review_required
destination_paths: []
modification_summary: null
copyright_notices_preserved: false
license_review: unverified
tests: []
test_result: unverified
implementation_commit: null
evidence_paths: []
reviewer: null
```

推荐顺序：核对 SOURCE_LOCK → checkout 静态审计 → 建立登记 → 复现 MDM → 选择最小改动 → 实现并验证对应 TEST → 记录准确证据。任何外部发布动作仍以实际会话授权为准。
