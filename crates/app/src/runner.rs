//! Background media worker. The egui thread never spawns or waits on processes.

use mdm_ipc::{ErrorCode, ErrorDetail, MediaIntent, MediaKind};
use mdm_tools::{ComponentLockManifest, build_ytdlp_command, classify_ytdlp_stderr};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

const COMPONENT_LOCK_JSON: &str = include_str!("../../../COMPONENT_LOCK.json");

#[derive(Debug, Clone)]
pub struct StartRequest {
    pub task_id: String,
    pub attempt_id: String,
    pub page_url: String,
    pub file_name_hint: String,
    pub output_dir: Option<String>,
    pub media_intent: Option<MediaIntent>,
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    Log {
        task_id: String,
        message: String,
    },
    Progress {
        task_id: String,
        attempt_id: String,
        bytes: u64,
    },
    Completed {
        task_id: String,
        attempt_id: String,
    },
    Failed {
        task_id: String,
        attempt_id: String,
        error: ErrorDetail,
    },
}

#[derive(Clone)]
pub struct WorkerHandle {
    start_tx: Sender<StartRequest>,
}

impl WorkerHandle {
    pub fn start(&self, request: StartRequest) {
        let _ = self.start_tx.send(request);
    }
}

pub fn spawn_worker() -> (WorkerHandle, Receiver<WorkerEvent>) {
    let (start_tx, start_rx) = mpsc::channel::<StartRequest>();
    let (event_tx, event_rx) = mpsc::channel::<WorkerEvent>();
    thread::spawn(move || {
        while let Ok(request) = start_rx.recv() {
            handle_request(request, &event_tx);
        }
    });
    (WorkerHandle { start_tx }, event_rx)
}

fn log(event_tx: &Sender<WorkerEvent>, task_id: &str, message: impl Into<String>) {
    let _ = event_tx.send(WorkerEvent::Log {
        task_id: task_id.to_string(),
        message: message.into(),
    });
}

fn fail(event_tx: &Sender<WorkerEvent>, task_id: &str, attempt_id: &str, error: ErrorDetail) {
    let _ = event_tx.send(WorkerEvent::Failed {
        task_id: task_id.to_string(),
        attempt_id: attempt_id.to_string(),
        error,
    });
}

fn load_manifest(root: &Path) -> Result<ComponentLockManifest, ErrorDetail> {
    let runtime_path = root.join("COMPONENT_LOCK.json");
    let json = match std::fs::read_to_string(&runtime_path) {
        Ok(json) => json,
        Err(_) => COMPONENT_LOCK_JSON.to_string(),
    };
    serde_json::from_str(&json).map_err(|err| ErrorDetail {
        code: ErrorCode::InternalError,
        message: format!("COMPONENT_LOCK.json 解析失败: {}", err),
        retryable: false,
        retry_after_ms: None,
    })
}

fn install_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn component_path(manifest: &ComponentLockManifest, root: &Path, id: &str) -> Option<PathBuf> {
    manifest
        .components
        .iter()
        .find(|component| component.id == id)
        .and_then(|component| component.relative_executable.as_ref())
        .map(|relative| root.join(relative))
}

fn format_expr(intent: Option<&MediaIntent>) -> &'static str {
    match intent.map(|intent| intent.kind) {
        Some(MediaKind::AudioOnly) => "ba/b",
        Some(MediaKind::VideoOnly) => "bv*/b",
        Some(MediaKind::SubtitlesOnly) => "sub",
        _ => "bv*+ba/b",
    }
}

fn output_stem(file_name_hint: &str) -> &str {
    file_name_hint
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(file_name_hint)
}

/// Empty output directory falls back to a `Downloads` sub-folder next to the executable.
fn resolve_output_dir(requested: Option<&str>, root: &Path) -> PathBuf {
    match requested.map(str::trim) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => root.join("Downloads"),
    }
}

fn output_template(request: &StartRequest, out_dir: &Path) -> String {
    out_dir
        .join(output_stem(&request.file_name_hint))
        .to_string_lossy()
        .to_string()
}

/// yt-dlp fragment files look like `<stem>.f399.mp4`; the merged file keeps only `<stem>.<ext>`.
fn is_fragment(name: &str, stem: &str) -> bool {
    let prefix = format!("{}.f", stem);
    let Some(rest) = name.strip_prefix(&prefix) else {
        return false;
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    !digits.is_empty() && rest[digits.len()..].starts_with('.')
}

fn probe_has_audio(ffprobe: &Path, file: &Path) -> bool {
    Command::new(ffprobe)
        .args(["-v", "error", "-show_entries", "stream=codec_type", "-of", "csv=p=0"])
        .arg(file)
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .any(|line| line.trim() == "audio")
        })
        .unwrap_or(false)
}

/// Verifies the merged output, removes yt-dlp fragments kept by `--keep-video`,
/// and returns a human-readable completion line with the final absolute path.
fn finalize_output(
    out_dir: &Path,
    file_name_hint: &str,
    ffprobe: &Path,
) -> Result<String, String> {
    let stem = output_stem(file_name_hint);
    let entries = std::fs::read_dir(out_dir)
        .map_err(|err| format!("读取输出目录失败: {}", err))?;
    let mut candidates = Vec::new();
    let mut fragments = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(stem) {
            continue;
        }
        if is_fragment(&name, stem) {
            fragments.push(entry.path());
            continue;
        }
        let ext = name
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .unwrap_or_default();
        if matches!(
            ext.as_str(),
            "mp4" | "webm" | "mkv" | "mov" | "m4a" | "mp3" | "flac" | "opus"
        ) {
            candidates.push(entry.path());
        }
    }
    let final_path = candidates
        .into_iter()
        .max_by_key(|path| std::fs::metadata(path).and_then(|m| m.modified()).ok())
        .ok_or_else(|| format!("未在 {} 找到合并成品", out_dir.display()))?;
    let has_audio = probe_has_audio(ffprobe, &final_path);
    let mut removed = 0usize;
    for fragment in fragments {
        if std::fs::remove_file(&fragment).is_ok() {
            removed += 1;
        }
    }
    Ok(format!(
        "下载完成: {} | 音轨: {} | 已清理 {} 个中间文件",
        final_path.display(),
        if has_audio { "有" } else { "无" },
        removed
    ))
}

fn parse_downloaded_bytes(line: &str) -> Option<u64> {
    let body = line.split_once("] ").map(|(_, rest)| rest).unwrap_or(line);
    for token in body.split_whitespace() {
        let lower = token.to_ascii_lowercase();
        for (suffix, factor) in [
            ("gib", 1024.0 * 1024.0 * 1024.0),
            ("mib", 1024.0 * 1024.0),
            ("kib", 1024.0),
            ("gb", 1000.0 * 1000.0 * 1000.0),
            ("mb", 1000.0 * 1000.0),
            ("kb", 1000.0),
        ] {
            if let Some(number) = lower.strip_suffix(suffix) {
                if let Ok(value) = number.parse::<f64>() {
                    return Some((value * factor) as u64);
                }
            }
        }
    }
    None
}

fn handle_request(request: StartRequest, event_tx: &Sender<WorkerEvent>) {
    log(
        event_tx,
        &request.task_id,
        format!(
            "worker 已启动，attempt={}，准备校验媒体组件",
            request.attempt_id
        ),
    );

    let root = install_root();
    let manifest = match load_manifest(&root) {
        Ok(manifest) => manifest,
        Err(error) => {
            log(event_tx, &request.task_id, error.message.clone());
            fail(event_tx, &request.task_id, &request.attempt_id, error);
            return;
        }
    };
    let component_set_id = match manifest.verify_ready_for_execution(&root) {
        Ok(set_id) => set_id,
        Err(error) => {
            let message = format!(
                "组件闸门拒绝执行：{}。COMPONENT_LOCK 必须先解析并核验 yt-dlp/EJS/Deno/FFmpeg；当前包未内置已验证组件。",
                error.message
            );
            log(event_tx, &request.task_id, &message);
            fail(
                event_tx,
                &request.task_id,
                &request.attempt_id,
                ErrorDetail {
                    code: error.code,
                    message,
                    retryable: false,
                    retry_after_ms: None,
                },
            );
            return;
        }
    };
    log(
        event_tx,
        &request.task_id,
        format!("组件集 {} 校验通过，准备调用受控 yt-dlp", component_set_id),
    );

    let ytdlp_bin = match component_path(&manifest, &root, "yt-dlp") {
        Some(path) => path,
        None => {
            let error = ErrorDetail {
                code: ErrorCode::ToolUnavailable,
                message: "COMPONENT_LOCK 缺少 yt-dlp 可执行路径".to_string(),
                retryable: false,
                retry_after_ms: None,
            };
            fail(event_tx, &request.task_id, &request.attempt_id, error);
            return;
        }
    };
    let deno_bin = component_path(&manifest, &root, "deno")
        .unwrap_or_else(|| root.join("components/deno/deno.exe"));
    let ffmpeg_bin = component_path(&manifest, &root, "ffmpeg")
        .unwrap_or_else(|| root.join("components/ffmpeg/ffmpeg.exe"));
    let ffprobe_bin = component_path(&manifest, &root, "ffprobe")
        .unwrap_or_else(|| root.join("components/ffmpeg/ffprobe.exe"));
    let out_dir = resolve_output_dir(request.output_dir.as_deref(), &root);
    if let Err(error) = std::fs::create_dir_all(&out_dir) {
        let detail = ErrorDetail {
            code: ErrorCode::InternalError,
            message: format!("创建下载目录失败 {}: {}", out_dir.display(), error),
            retryable: false,
            retry_after_ms: None,
        };
        log(event_tx, &request.task_id, detail.message.clone());
        fail(event_tx, &request.task_id, &request.attempt_id, detail);
        return;
    }
    log(
        event_tx,
        &request.task_id,
        format!("下载目录: {}", out_dir.display()),
    );
    let spec = match build_ytdlp_command(
        &ytdlp_bin,
        ffmpeg_bin.parent().unwrap_or(&root),
        &deno_bin,
        "",
        format_expr(request.media_intent.as_ref()),
        &output_template(&request, &out_dir),
        &request.page_url,
        4,
    ) {
        Ok(spec) => spec,
        Err(error) => {
            log(event_tx, &request.task_id, &error.message);
            fail(event_tx, &request.task_id, &request.attempt_id, error);
            return;
        }
    };
    log(
        event_tx,
        &request.task_id,
        format!(
            "执行命令: {}（argv 由受控构造器生成，不经 shell）",
            spec.executable
        ),
    );

    let mut command = Command::new(&spec.executable);
    command.args(&spec.args);
    command.env_clear();
    for key in ["SystemRoot", "TEMP", "TMP"] {
        if let Ok(value) = std::env::var(key) {
            command.env(key, value);
        }
    }
    command.envs(spec.clean_env.clone());
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let detail = ErrorDetail {
                code: ErrorCode::ToolUnavailable,
                message: format!("yt-dlp 启动失败: {}", error),
                retryable: false,
                retry_after_ms: None,
            };
            log(event_tx, &request.task_id, &detail.message);
            fail(event_tx, &request.task_id, &request.attempt_id, detail);
            return;
        }
    };
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let stderr_thread = thread::spawn(move || {
        BufReader::new(stderr)
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
            .join("\n")
    });

    for line_result in BufReader::new(stdout).lines() {
        let line = match line_result {
            Ok(line) => line,
            Err(error) => {
                log(
                    event_tx,
                    &request.task_id,
                    format!("读取 stdout 失败: {}", error),
                );
                break;
            }
        };
        log(event_tx, &request.task_id, line.clone());
        if let Some(bytes) = parse_downloaded_bytes(&line) {
            let _ = event_tx.send(WorkerEvent::Progress {
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
                bytes,
            });
        }
    }

    let status = child.wait();
    let stderr_text = stderr_thread.join().unwrap_or_default();
    if !stderr_text.trim().is_empty() {
        log(
            event_tx,
            &request.task_id,
            format!("yt-dlp stderr: {}", stderr_text),
        );
    }
    match status {
        Ok(status) if status.success() => {
            match finalize_output(&out_dir, &request.file_name_hint, &ffprobe_bin) {
                Ok(summary) => log(event_tx, &request.task_id, summary),
                Err(error) => log(
                    event_tx,
                    &request.task_id,
                    format!("成品校验提示: {}", error),
                ),
            }
            let _ = event_tx.send(WorkerEvent::Completed {
                task_id: request.task_id.clone(),
                attempt_id: request.attempt_id.clone(),
            });
        }
        Ok(status) => {
            let (_, error) = classify_ytdlp_stderr(&stderr_text);
            let error = if stderr_text.trim().is_empty() {
                ErrorDetail {
                    code: ErrorCode::InternalError,
                    message: format!("yt-dlp 退出码 {}，且 stderr 为空", status),
                    retryable: false,
                    retry_after_ms: None,
                }
            } else {
                error
            };
            fail(event_tx, &request.task_id, &request.attempt_id, error);
        }
        Err(error) => {
            fail(
                event_tx,
                &request.task_id,
                &request.attempt_id,
                ErrorDetail {
                    code: ErrorCode::InternalError,
                    message: format!("等待 yt-dlp 失败: {}", error),
                    retryable: false,
                    retry_after_ms: None,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolved_component_lock_reports_tool_unavailable() {
        let (handle, events) = spawn_worker();
        handle.start(StartRequest {
            task_id: "task-test".to_string(),
            attempt_id: "att-test".to_string(),
            page_url: "https://youtu.be/l7sCL3geF0U".to_string(),
            file_name_hint: "youtube_l7sCL3geF0U.mp4".to_string(),
            output_dir: None,
            media_intent: None,
        });

        let mut failure = None;
        for _ in 0..4 {
            match events.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(WorkerEvent::Failed { error, .. }) => {
                    failure = Some(error);
                    break;
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        let failure = failure.expect("worker must report a terminal failure");
        assert_eq!(failure.code, ErrorCode::ToolUnavailable);
        assert!(failure.message.contains("COMPONENT_LOCK"));
    }

    #[test]
    fn empty_output_dir_falls_back_to_downloads_subfolder() {
        let root = Path::new("C:/apps/rustcatch");
        let resolved = resolve_output_dir(None, root);
        assert_eq!(resolved, root.join("Downloads"));
        assert_eq!(
            resolve_output_dir(Some("  D:/videos  "), root),
            PathBuf::from("D:/videos")
        );
        assert_eq!(
            resolve_output_dir(Some("   "), root),
            root.join("Downloads")
        );

        let request = StartRequest {
            task_id: "t".into(),
            attempt_id: "a".into(),
            page_url: "https://youtu.be/x".into(),
            file_name_hint: "youtube_x.mp4".into(),
            output_dir: None,
            media_intent: None,
        };
        assert_eq!(
            output_template(&request, &resolved),
            root.join("Downloads").join("youtube_x").to_string_lossy()
        );
    }

    #[test]
    fn fragment_files_are_detected_but_merged_output_is_kept() {
        assert!(is_fragment("youtube_x.f399.mp4", "youtube_x"));
        assert!(is_fragment("youtube_x.f251.webm", "youtube_x"));
        assert!(!is_fragment("youtube_x.webm", "youtube_x"));
        assert!(!is_fragment("youtube_x.mp4", "youtube_x"));
        assert!(!is_fragment("other.f399.mp4", "youtube_x"));
    }

    #[test]
    fn finalize_output_removes_fragments_and_reports_final_path() {
        let root = std::env::temp_dir().join(format!(
            "rustcatch-finalize-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let stem = "youtube_x";
        std::fs::write(root.join(format!("{}.f399.mp4", stem)), b"video").unwrap();
        std::fs::write(root.join(format!("{}.f251.webm", stem)), b"audio").unwrap();
        std::fs::write(root.join(format!("{}.webm", stem)), b"merged").unwrap();
        std::fs::write(root.join("unrelated.txt"), b"keep").unwrap();

        let summary = finalize_output(
            &root,
            "youtube_x.mp4",
            Path::new("C:/nonexistent/ffprobe.exe"),
        )
        .expect("finalize must find merged output");
        assert!(summary.contains("音轨: 无"), "summary={}", summary);
        assert!(summary.contains("已清理 2 个中间文件"), "summary={}", summary);
        assert!(!root.join(format!("{}.f399.mp4", stem)).exists());
        assert!(!root.join(format!("{}.f251.webm", stem)).exists());
        assert!(root.join(format!("{}.webm", stem)).exists());
        assert!(root.join("unrelated.txt").exists());

        let _ = std::fs::remove_dir_all(&root);
    }
}
