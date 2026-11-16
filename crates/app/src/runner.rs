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

fn output_template(request: &StartRequest) -> String {
    let stem = request
        .file_name_hint
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(&request.file_name_hint);
    match request.output_dir.as_deref().map(str::trim) {
        Some(dir) if !dir.is_empty() => format!(
            "{}/{}",
            dir.trim_end_matches(|c| c == '/' || c == '\\'),
            stem
        ),
        _ => stem.to_string(),
    }
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
    let spec = match build_ytdlp_command(
        &ytdlp_bin,
        ffmpeg_bin.parent().unwrap_or(&root),
        &deno_bin,
        "",
        format_expr(request.media_intent.as_ref()),
        &output_template(&request),
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
}
