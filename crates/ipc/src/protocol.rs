use crate::task_api::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserBridgeRequest {
    pub version: u16,
    pub request_id: String,
    pub idempotency_key: String,
    pub op: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserBridgeResponse {
    pub version: u16,
    pub request_id: String,
    pub status: ResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_state: Option<TaskState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<DecimalU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub durable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Ok,
    Prepared,
    Accepted,
    Queued,
    Aborted,
    NotModified,
    Error,
}

impl BrowserBridgeResponse {
    pub fn error(request_id: impl Into<String>, code: ErrorCode, message: impl Into<String>, retryable: bool, retry_after_ms: Option<u64>) -> Self {
        Self {
            version: PROTOCOL_VERSION_V1,
            request_id: request_id.into(),
            status: ResponseStatus::Error,
            task_id: None,
            task_state: None,
            revision: None,
            durable: None,
            handoff_token: None,
            expires_at: None,
            data: None,
            error: Some(ErrorDetail {
                code,
                message: message.into(),
                retryable,
                retry_after_ms,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserMeta {
    pub installation_id: String,
    pub profile_id: String,
    pub download_id: Option<u64>,
    pub tab_id: u32,
    pub frame_id: u32,
    pub document_id: String,
    pub nav_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SourceSpec {
    DirectHttp {
        url: String,
        #[serde(default)]
        page_url: Option<String>,
        method: String,
        replayability: Replayability,
        #[serde(default)]
        replayability_evidence_ref: Option<String>,
    },
    PageUrl {
        url: String,
        #[serde(default)]
        page_url: Option<String>,
        #[serde(default)]
        platform_media_id: Option<String>,
    },
    Hls {
        url: String,
        page_url: String,
        candidate_id: String,
    },
    Dash {
        url: String,
        page_url: String,
        candidate_id: String,
    },
    CapturedFile {
        url: String,
        page_url: String,
        candidate_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Replayability {
    KnownReplayable,
    Unknown,
    NonReplayable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPolicy {
    pub directory_policy: DirectoryPolicy,
    pub file_name_hint: String,
    #[serde(default)]
    pub directory_ref: Option<String>,
    #[serde(default)]
    pub queue_id: Option<String>,
    #[serde(default = "default_collision")]
    pub collision: CollisionPolicy,
}

fn default_collision() -> CollisionPolicy {
    CollisionPolicy::Rename
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryPolicy {
    CategoryDefault,
    UserSelected,
    QueueDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionPolicy {
    Rename,
    Overwrite,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureCandidate {
    pub candidate_id: String,
    pub kind: String,
    pub url: String,
    pub page_url: String,
    pub browser: BrowserMeta,
    pub mime: String,
    pub observed_at: String,
    pub size_bytes: Option<DecimalU64>,
    pub size_kind: SizeKind,
    pub source: String,
    pub request_context_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextMaterials {
    #[serde(default)]
    pub cookies: Vec<CookieEntry>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String,
    pub expires_at: Option<String>,
    pub partition_key: Option<CookiePartitionKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CookiePartitionKey {
    pub top_level_site: String,
    pub has_cross_site_ancestor: bool,
}
