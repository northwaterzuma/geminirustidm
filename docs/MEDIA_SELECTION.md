# 媒体格式、音轨、字幕与播放列表选择规范 (MEDIA_SELECTION - P3-T02)

- **模块位置**：`crates/tools/src/media_policy.rs`
- **选择规则**：
  - `require_exact_format = true` 时若指定 `format_id` 缺失，立即返回 `FormatUnavailable`，绝不静默降级。
  - `max_height` 仅作为分辨率上限；`hdr` 区分 `preserve / require_hdr / prefer_sdr / any`。
  - 音轨优先匹配 `audio_language`（`original` 严格匹配 `is_original == true`，不静默替换为 AI 配音轨）。
  - 播放列表按单页最多 500 条分页预览（`expand_playlist_page`），同次展开按 `video_id` 去重，私享/删除视频单独归入 `unavailable_entries`，不导致其余正常视频失败。
