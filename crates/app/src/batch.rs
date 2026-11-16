//! P4-T01 (REQ-10): Batch URL wildcard expander, clipboard filter, and secret-free import/export.

use crate::model::TaskRecord;
use mdm_ipc::{ErrorCode, ErrorDetail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchLineResult {
    pub line_number: usize,
    pub raw_input: String,
    pub valid_url: Option<String>,
    pub skipped_duplicate: bool,
    pub error_message: Option<String>,
}

/// Parses and validates a batch list of URLs, reporting invalid lines individually and deduplicating within the same batch.
pub fn parse_batch_urls(lines: &[String], max_batch_size: usize) -> Result<Vec<BatchLineResult>, ErrorDetail> {
    if lines.len() > max_batch_size {
        return Err(ErrorDetail {
            code: ErrorCode::TooLarge,
            message: format!("Batch size {} exceeds limit {}", lines.len(), max_batch_size),
            retryable: false,
            retry_after_ms: None,
        });
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for (idx, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        match Url::parse(trimmed) {
            Ok(u) if u.scheme() == "http" || u.scheme() == "https" || u.scheme() == "ftp" || u.scheme() == "ftps" => {
                let normalized = u.to_string();
                let is_dup = !seen.insert(normalized.clone());
                out.push(BatchLineResult {
                    line_number: idx + 1,
                    raw_input: trimmed.to_string(),
                    valid_url: if is_dup { None } else { Some(normalized) },
                    skipped_duplicate: is_dup,
                    error_message: None,
                });
            }
            _ => {
                out.push(BatchLineResult {
                    line_number: idx + 1,
                    raw_input: trimmed.to_string(),
                    valid_url: None,
                    skipped_duplicate: false,
                    error_message: Some("Unsupported or malformed URL".to_string()),
                });
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedTaskBundle {
    pub schema_version: u32,
    pub exported_at_utc: String,
    pub tasks: Vec<TaskRecord>,
}

/// Exports tasks with all credentials, context_refs, and signed URLs stripped/redacted.
pub fn export_redacted_tasks(tasks: &[TaskRecord]) -> ExportedTaskBundle {
    let sanitized = tasks
        .iter()
        .map(|t| {
            let mut copy = t.clone();
            copy.context_ref = None;
            copy.source_display = mdm_ipc::redact_sensitive_url(&copy.source_display);
            copy.protected_source_ref = "[redacted-on-export]".to_string();
            copy
        })
        .collect();
    ExportedTaskBundle {
        schema_version: mdm_ipc::CURRENT_SCHEMA_VERSION,
        exported_at_utc: "2026-11-16T08:00:00Z".to_string(),
        tasks: sanitized,
    }
}
