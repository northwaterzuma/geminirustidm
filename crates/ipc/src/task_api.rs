use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

pub const PROTOCOL_VERSION_V1: u16 = 1;
pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_FRAME_BYTES: usize = 262_144;
pub const MAX_JSON_DEPTH: usize = 16;
pub const MAX_CANDIDATES_PER_PAGE: usize = 100;
pub const MAX_CONTEXT_BYTES: usize = 32_768;

/// Decimal string wrapper for u64 to prevent IEEE-754 precision loss in JS/JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DecimalU64(pub u64);

impl Serialize for DecimalU64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.is_empty() || s.len() > 20 {
            return Err(serde::de::Error::custom("invalid u64 decimal string length"));
        }
        if s != "0" && s.starts_with('0') {
            return Err(serde::de::Error::custom("leading zeros not allowed in u64 string"));
        }
        if !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(serde::de::Error::custom("non-digit in u64 decimal string"));
        }
        let val = s.parse::<u64>().map_err(|_| serde::de::Error::custom("u64 decimal overflow"))?;
        Ok(DecimalU64(val))
    }
}

impl fmt::Display for DecimalU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    HandoffPending,
    Queued,
    Resolving,
    AwaitingUser,
    Running,
    Finishing,
    Pausing,
    Paused,
    Cancelling,
    Cancelled,
    Failed,
    Completed,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskState::Cancelled | TaskState::Completed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPhase {
    Resolve,
    Select,
    Transfer,
    Merge,
    Verify,
    Commit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    UnsupportedVersion,
    InvalidRequest,
    InvalidState,
    PermissionDenied,
    TooLarge,
    ContextExpired,
    StaleNavigation,
    IdempotencyConflict,
    NotFound,
    RevisionConflict,
    LeaseExpired,
    AlreadyCommitted,
    NotReplayable,
    ToolUnavailable,
    NetworkError,
    RateLimited,
    AuthRequired,
    AuthRejected,
    BotBlocked,
    IpBlocked,
    GeoBlocked,
    FormatUnavailable,
    ExtractorChanged,
    ResourceChanged,
    DrmProtected,
    DiskFull,
    MergeFailed,
    Cancelled,
    InternalError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizeKind {
    Exact,
    Estimate,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub task_id: String,
    pub revision: DecimalU64,
    pub attempt_id: Option<String>,
    pub state: TaskState,
    pub phase: Option<TaskPhase>,
    pub reason: Option<String>,
    pub display_name: String,
    pub source_display: String,
    pub bytes_received: DecimalU64,
    pub total_bytes: Option<DecimalU64>,
    pub size_kind: SizeKind,
    pub progress_ratio: Option<f64>,
    pub component_set_id: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlOp {
    Pause,
    Resume,
    Cancel,
    Retry,
    RemoveRecord,
    DeleteFiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffDecision {
    ContinueDesktop,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoverHandoff {
    pub task_id: String,
    pub expected_revision: DecimalU64,
    pub decision: HandoffDecision,
    pub browser_cancel_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallerScope {
    pub os_user: String,
    pub origin_kind: CallerOriginKind,
    pub instance_id: String,
    pub profile_id: String,
    pub capabilities: Vec<CapabilityScope>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallerOriginKind {
    DesktopUi,
    NativeExtension,
    LocalCli,
    McpClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityScope {
    Read,
    Add,
    Control,
    DeleteFiles,
    InspectNetwork,
    ManageQueue,
    ManageLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaIntent {
    pub version: u16,
    pub kind: MediaKind,
    pub preset: MediaPreset,
    pub max_height: Option<u32>,
    pub codec_preference: Vec<String>,
    pub container: String,
    pub hdr: String,
    pub audio_language: String,
    pub subtitles: SubtitlePolicy,
    pub allow_quality_downgrade: bool,
    pub allow_transcode: bool,
    pub require_exact_format: bool,
    pub selected_format_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    VideoAudio,
    VideoOnly,
    AudioOnly,
    SubtitlesOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaPreset {
    HighestQuality,
    DeviceCompatible,
    AudioOnly,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitlePolicy {
    pub languages: Vec<String>,
    pub source: String,
    pub mode: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkProfile {
    pub version: u16,
    pub id: String,
    pub revision: DecimalU64,
    pub mode: NetworkMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<ProxyEndpoint>,
    pub auth_ref: Option<String>,
    pub dns: DnsPolicy,
    pub bypass_hosts: Vec<String>,
    pub direct_fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    System,
    Manual,
    Direct,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyEndpoint {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DnsPolicy {
    System,
    Local,
    Proxy,
}
