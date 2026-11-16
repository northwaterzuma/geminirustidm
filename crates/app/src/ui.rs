//! IDM Classic Light UI View-Model and egui/eframe Window Bindings (UI-01 .. UI-12).

use crate::task_coordinator::TaskCoordinator;
use mdm_ipc::{CallerOriginKind, CallerScope, CapabilityScope, ControlOp, TaskSnapshot, TaskState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdmWindowId {
    Ui01Main,
    Ui02AddUrl,
    Ui03DownloadInfo,
    Ui04Progress,
    Ui05VideoSelector,
    Ui06CompletionProperties,
    Ui07Options,
    Ui08QueueScheduler,
    Ui09BrowserFloatingBar,
    Ui10BatchGrabberWizard,
    Ui11RecoveryDiagnostics,
    Ui12DesktopTrayBasket,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarButtonStates {
    pub can_add: bool,
    pub can_resume: bool,
    pub can_stop: bool,
    pub can_stop_all: bool,
    pub can_delete_record: bool,
    pub can_open_options: bool,
    pub can_open_scheduler: bool,
    pub can_start_queue: bool,
    pub can_stop_queue: bool,
    pub can_open_grabber: bool,
}

/// Computes IDM classic toolbar button enable/disable states based on current selection.
pub fn compute_toolbar_states(selected: &[TaskSnapshot], any_running: bool) -> ToolbarButtonStates {
    let can_resume = selected
        .iter()
        .any(|t| matches!(t.state, TaskState::Paused | TaskState::Failed | TaskState::Queued));
    let can_stop = selected
        .iter()
        .any(|t| matches!(t.state, TaskState::Running | TaskState::Resolving | TaskState::Finishing));
    let can_delete_record = !selected.is_empty();

    ToolbarButtonStates {
        can_add: true,
        can_resume,
        can_stop,
        can_stop_all: any_running,
        can_delete_record,
        can_open_options: true,
        can_open_scheduler: true,
        can_start_queue: true,
        can_stop_queue: true,
        can_open_grabber: true,
    }
}

pub struct RustCatchDesktopApp {
    pub coordinator: TaskCoordinator,
    pub selected_category: String,
    pub selected_task_ids: Vec<String>,
    pub add_url_input: String,
    pub close_to_tray: bool,
    pub status_message: String,
}

impl Default for RustCatchDesktopApp {
    fn default() -> Self {
        Self {
            coordinator: TaskCoordinator::new(),
            selected_category: "所有下载 (All Downloads)".to_string(),
            selected_task_ids: Vec::new(),
            add_url_input: String::new(),
            close_to_tray: true,
            status_message: "就绪 (Ready) | 全局速度: 0 B/s".to_string(),
        }
    }
}

impl RustCatchDesktopApp {
    pub fn ui_caller() -> CallerScope {
        CallerScope {
            os_user: "local-desktop-user".to_string(),
            origin_kind: CallerOriginKind::DesktopUi,
            instance_id: "desktop-ui".to_string(),
            profile_id: "default".to_string(),
            capabilities: vec![
                CapabilityScope::Read,
                CapabilityScope::Add,
                CapabilityScope::Control,
                CapabilityScope::DeleteFiles,
                CapabilityScope::InspectNetwork,
                CapabilityScope::ManageQueue,
                CapabilityScope::ManageLimits,
            ],
        }
    }

    /// Keyboard Delete triggers `RemoveRecord` only (never `DeleteFiles` unless explicitly confirmed with Shift+Delete).
    pub fn handle_delete_key(&mut self, shift_delete_files: bool, now_epoch_ms: u64) {
        let caller = Self::ui_caller();
        let ids = self.selected_task_ids.clone();
        for id in ids {
            if let Some(rec) = self.coordinator.tasks.get(&id) {
                let rev = rec.revision;
                if shift_delete_files {
                    let _ = self.coordinator.control_task(&caller, "ui-del-files", &id, rev, ControlOp::DeleteFiles, now_epoch_ms);
                }
                let next_rev = self.coordinator.tasks.get(&id).map(|r| r.revision).unwrap_or(rev);
                let _ = self.coordinator.control_task(&caller, "ui-rm-rec", &id, next_rev, ControlOp::RemoveRecord, now_epoch_ms);
            }
        }
        self.selected_task_ids.clear();
    }
}
