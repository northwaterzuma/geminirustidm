# v0.1.5-weibo-redirect 验证结果

- 提交：`d4de8b4 fix(media): recover real URL from weibo visitor-gate redirect and retry`
- 标签：`v0.1.5-weibo-redirect`
- 主 CI：run `43886518075`，4/4 jobs success；Windows / Ubuntu / macOS 各 **67 passed / 0 failed**（较 v0.1.4 的 65 项增加 2 项微博重定向测试）
- 发布流水线：run `43888160976`，success，`Publish to GitHub Release` 步骤通过
- Release 页：https://github.com/northwaterzuma/wbhy4falshv1/releases/tag/v0.1.5-weibo-redirect
- 资产：`rustcatch-win64-portable-unsigned-test.zip`，203,300,117 bytes，GitHub digest `sha256:865385b99b368ca80cfc03f941955a2d9612afa105f28404c6329e6706bd32ca`

## 用户报告与根因

用户日志：输入 `http://t.cn/AXTh2EWk`，yt-dlp 输出

```
[redirect] Following redirect to https://passport.weibo.com/visitor/visitor?...&url=<encoded>
ERROR: Unsupported URL: https://passport.weibo.com/visitor/visitor?...
NetworkError: Media pipeline classified error as Network
```

真实诊断（用 v0.1.3 打包 yt-dlp 实测）：

| 输入 | 结果 |
| --- | --- |
| `http://t.cn/AXTh2EWk` | 页面指向 `video.weibo.com/show?fid=1034:...`，再跳 `passport.weibo.com/visitor/visitor?...&url=<编码真实地址>` |
| 直接用 visitor 页 | `[generic]` 提取器 → `Unsupported URL`（复现用户报错） |
| 直接喂 `https://weibo.com/tv/show/1034:5364596685602859` | `[WeiboVideo]` 提取器成功，生成访客 cookie，解析出 `mp4_hd` / `mp4_720p` 两种**带 mp4a 音频**的格式 |

根因：不是网络故障，也不是 yt-dlp 不支持微博。是短链经微博访客校验网关跳转后，yt-dlp 把网关页当最终目标，用 generic 提取器失败；而网关 URL 的 `url=` 参数里就藏着专用提取器能处理的干净地址。分类为 `NetworkError` 是误导。

## 修复

零新依赖，不主动发网络请求，复用 yt-dlp 已看到的重定向信息：

- `tools::video::extract_weibo_visitor_target(line)`：纯函数，解析 `Following redirect to <url>`，仅当目标是 `passport.weibo.com` 或 `weibo.com/visitor/` 时提取 `url=` 参数；校验还原目标 host 必须是 `weibo.com` 或其子域，防止把用户导向恶意域。
- `runner::handle_request`：把单次执行抽成 `run_once`，保留全部 stdout 行；首次失败后扫描重定向行，若还原出与当前不同的真实地址，**用同一套受控 argv 构造器重试一次**，并在日志写明"检测到微博访客校验跳转，已从重定向中还原真实地址并重试一次: <url>"。
- 无可用重定向目标时维持原有错误分类与失败路径，行为不变。
- 新增 2 项真实 Rust 单测：从真实日志行还原 URL；非 visitor 跳转/非微博目标被忽略。

## 证据边界

- 本机实测证明：干净 `weibo.com/tv/show/...` 地址能被 `[WeiboVideo]` 完美解析，带音频。
- CI 三平台 67 项测试通过，含新增重定向解析单测。
- v0.1.5 在 GUI 中对该短链的真机端到端下载仍需用户确认；本修复只针对"访客校验网关"这一已确认的失败模式，其它微博反爬/登录墙场景不在本次范围。
