# v0.1.4-ui-understandability 验证结果

- 仓库：`northwaterzuma/wbhy4falshv1`
- 提交：`49fb35c fix(ui): clean yt-dlp fragments, default Downloads dir, category filtering`
- 标签：`v0.1.4-ui-understandability`
- 主 CI：run `43881259247`，4/4 jobs success；Windows / Ubuntu / macOS 各 **65 passed / 0 failed**（较 v0.1.3 的 60 项增加 5 项）
- 发布流水线：run `43882920447`，success，`Publish to GitHub Release` 步骤通过
- Release 页：https://github.com/northwaterzuma/wbhy4falshv1/releases/tag/v0.1.4-ui-understandability
- 资产：`rustcatch-win64-portable-unsigned-test.zip`，203,296,963 bytes，GitHub digest `sha256:1925e68c33ca961fe7d80400be3ef92c459bd028293ab5791861e92d4d7102cf`

## 用户报告的三个问题与真实根因

### 1. "下载下来只有视频没有声音"

用 `v0.1.3-components` 包内组件 + 用户下载目录 `C:\00github rust\rustcatch-win64-portable-unsigned-test` 的真实文件验证：

| 文件 | ffprobe 流 | 结论 |
| --- | --- | --- |
| `youtube_l7sCL3geF0U.webm` (211MB) | `av1 video` + `opus audio`，1920x1080，493.6 秒 | **合并成品，有声音** |
| `youtube_l7sCL3geF0U.f399.mp4` (204MB) | 仅 `av1 video` | yt-dlp 纯视频分片，无音轨 |
| `youtube_l7sCL3geF0U.f251.webm` (7.2MB) | 仅 `opus audio` | yt-dlp 纯音频分片 |

Windows 最近打开记录指向 `youtube_l7sCL3geF0U.f399.mp4.lnk`，证明用户打开的是**分片文件**而不是合并成品。根因不是编码/格式选择：`bv*+ba` 在本机用打包组件实测成功合并出带音轨成品。真正的产品缺陷是 `--keep-video` 保留了分片，而 Rust 侧没有在验证后清理，目录中同时出现多个同名文件，用户无法分辨。

修复：
- `runner.rs::finalize_output`：成功后用包内 ffprobe 校验成品音轨，删除 `<stem>.f<digits>.*` 分片，日志输出成品绝对路径、`音轨: 有/无`、`已清理 N 个中间文件`。
- 新增 3 项 Rust 测试：分片识别、默认目录、清理后只保留成品且不动无关文件。

### 2. "默认下载到程序文件夹，应该建下载子目录"

根因：`output_template` 在 `directory_ref` 为空时返回裸文件名（相对路径），worker 继承的 cwd 就是 exe 目录。

修复：
- `runner.rs::resolve_output_dir`：空目录回退到 exe 同目录的 `Downloads/`，并 `create_dir_all`。
- `ui.rs::default_download_directory`：UI-03 下载信息窗"保存目录"预填同一路径，用户可见可改。
- 新增 2 项 Rust 测试覆盖默认目录回退、空白目录回退、自定义目录保留、UI 预填与任务落盘一致。

### 3. "已完成/未完成看不到任何东西，还没有归类"

根因：`app.rs` 中心任务列表遍历 `coordinator.tasks.values()` 全量渲染，`selected_category` 只改变高亮文字，从未参与过滤。

修复：
- `ui.rs::task_matches_category`：状态类（所有下载/未完成/已完成）、类型类（视频/音乐/压缩包/文档/程序）、队列类（主队列/同步队列）分别按 `state` / `category` / `queue_id` 匹配。
- `app.rs`：中心列表按当前分类过滤并显示"显示 N / 总数"；侧栏每个分类显示实时计数；空分类显示"该分类下暂无任务"。
- 新增 2 项 Rust 测试覆盖 Queued→Completed 迁移后"未完成/已完成"计数互换，以及视频/主队列分类匹配。

## 证据边界

- 本机实测证明 v0.1.3 打包组件能正确合并出带音轨成品，用户看到的"没声音"是打开错文件。
- v0.1.4 的修复经 GitHub CI 三平台 65 项测试验证，产物由 Release 流水线自动出包。
- 新包在 GUI 中的最终效果仍需用户下载 v0.1.4 后真机确认。
