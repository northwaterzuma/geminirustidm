use mdm_app::{AppAction, RustCatchDesktopApp, UiDialog};
use mdm_ipc::TaskState;

#[test]
fn add_url_prepares_download_info_for_youtube_and_creates_task() {
    let mut app = RustCatchDesktopApp::default();

    assert!(app.prepare_add_url("https://youtu.be/l7sCL3geF0U"));
    assert!(matches!(app.open_dialog, Some(UiDialog::DownloadInfo)));
    assert_eq!(app.pending_media_id.as_deref(), Some("l7sCL3geF0U"));
    assert_eq!(app.pending_file_name, "youtube_l7sCL3geF0U.mp4");

    app.pending_start_now = false;
    assert!(app.confirm_pending_download());
    assert_eq!(app.coordinator.tasks.len(), 1);

    let task = app
        .coordinator
        .tasks
        .values()
        .next()
        .expect("task must exist");
    assert_eq!(task.source_kind, "page_url");
    assert_eq!(task.state, TaskState::Queued);
    assert!(task.media_intent.is_some());
    assert_eq!(
        app.add_url_history,
        vec!["https://youtu.be/l7sCL3geF0U".to_string()]
    );
    assert!(app.open_dialog.is_none());
}

#[test]
fn empty_url_reports_error_and_does_not_create_task() {
    let mut app = RustCatchDesktopApp::default();

    assert!(!app.prepare_add_url("   "));
    assert!(app.dialog_error.is_some());
    assert!(app.coordinator.tasks.is_empty());
}

#[test]
fn prepare_add_url_action_skips_the_extra_click_when_address_is_present() {
    let mut app = RustCatchDesktopApp::default();

    app.apply_action(AppAction::PrepareAddUrl(
        "https://youtu.be/l7sCL3geF0U".to_string(),
    ));

    assert!(matches!(app.open_dialog, Some(UiDialog::DownloadInfo)));
    assert_eq!(app.add_url_input, "https://youtu.be/l7sCL3geF0U");
}

#[test]
fn menu_actions_open_visible_dialogs() {
    let mut app = RustCatchDesktopApp::default();

    app.apply_action(AppAction::OpenAddUrl);
    assert!(matches!(app.open_dialog, Some(UiDialog::AddUrl)));

    app.apply_action(AppAction::OpenOptions);
    assert!(matches!(app.open_dialog, Some(UiDialog::Options)));

    app.apply_action(AppAction::OpenAbout);
    assert!(matches!(app.open_dialog, Some(UiDialog::About)));

    app.apply_action(AppAction::CloseDialog);
    assert!(app.open_dialog.is_none());
}
