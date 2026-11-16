use crate::ui::{AppAction, RustCatchDesktopApp, UiDialog, compute_toolbar_states};
use mdm_ipc::TaskSnapshot;

impl eframe::App for RustCatchDesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut action: Option<AppAction> = None;

        egui::TopBottomPanel::top("idm_menu_and_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("任务(T)", |ui| {
                    if ui.button("添加地址... (Ctrl+N)").clicked() {
                        action = Some(AppAction::OpenAddUrl);
                    }
                    if ui.button("下载信息...").clicked() {
                        action = Some(AppAction::OpenDownloadInfo);
                    }
                    ui.separator();
                    if ui.button("开始队列").clicked() {
                        action = Some(AppAction::StartQueue);
                    }
                    if ui.button("停止队列").clicked() {
                        action = Some(AppAction::StopQueue);
                    }
                });
                ui.menu_button("文件(F)", |ui| {
                    if ui.button("新建下载任务").clicked() {
                        action = Some(AppAction::OpenAddUrl);
                    }
                    if ui.button("打开下载信息").clicked() {
                        action = Some(AppAction::OpenDownloadInfo);
                    }
                    ui.separator();
                    if ui.button("移除任务记录").clicked() {
                        action = Some(AppAction::RemoveSelected);
                    }
                });
                ui.menu_button("下载(D)", |ui| {
                    if ui.button("恢复 (Resume)").clicked() {
                        action = Some(AppAction::ResumeSelected);
                    }
                    if ui.button("停止 (Stop)").clicked() {
                        action = Some(AppAction::StopSelected);
                    }
                    ui.separator();
                    if ui.button("计划任务 / 队列").clicked() {
                        action = Some(AppAction::OpenScheduler);
                    }
                    if ui.button("站点抓取").clicked() {
                        action = Some(AppAction::OpenGrabber);
                    }
                });
                ui.menu_button("查看(V)", |ui| {
                    if ui.button("所有下载").clicked() {
                        action = Some(AppAction::SelectCategory(
                            "所有下载 (All Downloads)".to_string(),
                        ));
                    }
                    if ui.button("未完成").clicked() {
                        action = Some(AppAction::SelectCategory("未完成 (Unfinished)".to_string()));
                    }
                    if ui.button("已完成").clicked() {
                        action = Some(AppAction::SelectCategory("已完成 (Finished)".to_string()));
                    }
                    if ui.button("视频").clicked() {
                        action = Some(AppAction::SelectCategory("视频 (Video)".to_string()));
                    }
                });
                ui.menu_button("帮助(H)", |ui| {
                    if ui.button("关于 RustCatch").clicked() {
                        action = Some(AppAction::OpenAbout);
                    }
                });
                ui.separator();
                ui.label(format!(
                    "队列: {}",
                    if self.queue_running {
                        "运行中"
                    } else {
                        "已停止"
                    }
                ));
            });
            ui.separator();

            let selected: Vec<TaskSnapshot> = self
                .selected_task_ids
                .iter()
                .filter_map(|id| {
                    self.coordinator
                        .tasks
                        .get(id)
                        .map(|task| task.to_snapshot())
                })
                .collect();
            let any_running = self.coordinator.tasks.values().any(|task| {
                matches!(
                    task.state,
                    mdm_ipc::TaskState::Running
                        | mdm_ipc::TaskState::Resolving
                        | mdm_ipc::TaskState::Finishing
                )
            });
            let states = compute_toolbar_states(&selected, any_running);

            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.add_url_input);
                if ui
                    .add_enabled(states.can_add, egui::Button::new("添加地址 (Add URL)"))
                    .clicked()
                {
                    action = Some(AppAction::OpenAddUrl);
                }
                if ui
                    .add_enabled(states.can_resume, egui::Button::new("恢复 (Resume)"))
                    .clicked()
                {
                    action = Some(AppAction::ResumeSelected);
                }
                if ui
                    .add_enabled(states.can_stop, egui::Button::new("停止 (Stop)"))
                    .clicked()
                {
                    action = Some(AppAction::StopSelected);
                }
                if ui
                    .add_enabled(
                        states.can_delete_record,
                        egui::Button::new("删除记录 (Delete)"),
                    )
                    .clicked()
                {
                    action = Some(AppAction::RemoveSelected);
                }
                ui.separator();
                if ui
                    .add_enabled(states.can_open_options, egui::Button::new("选项"))
                    .clicked()
                {
                    action = Some(AppAction::OpenOptions);
                }
                if ui
                    .add_enabled(states.can_open_scheduler, egui::Button::new("计划任务"))
                    .clicked()
                {
                    action = Some(AppAction::OpenScheduler);
                }
                if ui
                    .add_enabled(states.can_start_queue, egui::Button::new("开始队列"))
                    .clicked()
                {
                    action = Some(AppAction::StartQueue);
                }
                if ui
                    .add_enabled(states.can_stop_queue, egui::Button::new("停止队列"))
                    .clicked()
                {
                    action = Some(AppAction::StopQueue);
                }
                if ui
                    .add_enabled(states.can_open_grabber, egui::Button::new("站点抓取"))
                    .clicked()
                {
                    action = Some(AppAction::OpenGrabber);
                }
            });
        });

        egui::SidePanel::left("idm_category_tree")
            .exact_width(180.0)
            .show(ctx, |ui| {
                ui.heading("分类 (Categories)");
                for category in [
                    "所有下载 (All Downloads)",
                    "未完成 (Unfinished)",
                    "已完成 (Finished)",
                    "视频 (Video)",
                    "音乐 (Music)",
                    "压缩包 (Compressed)",
                    "文档 (Documents)",
                    "程序 (Programs)",
                    "主队列 (Main Queue)",
                    "同步队列 (Sync Queue)",
                ] {
                    if ui
                        .selectable_label(self.selected_category == category, category)
                        .clicked()
                    {
                        action = Some(AppAction::SelectCategory(category.to_string()));
                    }
                }
            });

        egui::TopBottomPanel::bottom("idm_status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status_message);
                    ui.separator();
                    ui.label(format!("任务总数: {}", self.coordinator.tasks.len()));
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("任务列表 (紧凑视图 24px 行高)");
            ui.horizontal(|ui| {
                ui.label("提示：单击选中，Ctrl/Shift 多选；右键打开任务菜单");
            });
            let rows: Vec<(String, TaskSnapshot)> = self
                .coordinator
                .tasks
                .values()
                .map(|task| (task.task_id.clone(), task.to_snapshot()))
                .collect();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (task_id, snap) in rows {
                    let is_selected = self.selected_task_ids.contains(&task_id);
                    let response = ui.selectable_label(
                        is_selected,
                        format!(
                            "{} | {:?} | {} B | {}",
                            snap.display_name,
                            snap.state,
                            snap.bytes_received.0,
                            snap.source_display
                        ),
                    );
                    if response.clicked() {
                        let modifiers = ui.input(|input| input.modifiers);
                        if modifiers.ctrl || modifiers.shift {
                            if let Some(position) =
                                self.selected_task_ids.iter().position(|id| id == &task_id)
                            {
                                self.selected_task_ids.remove(position);
                            } else {
                                self.selected_task_ids.push(task_id.clone());
                            }
                        } else {
                            self.selected_task_ids = vec![task_id.clone()];
                        }
                    }
                    response.context_menu(|ui| {
                        if ui.button("恢复").clicked() {
                            action = Some(AppAction::ResumeSelected);
                        }
                        if ui.button("停止").clicked() {
                            action = Some(AppAction::StopSelected);
                        }
                        if ui.button("下载信息").clicked() {
                            action = Some(AppAction::OpenDownloadInfo);
                        }
                        if ui.button("移除记录").clicked() {
                            action = Some(AppAction::RemoveSelected);
                        }
                    });
                }
            });
        });

        if let Some(action) = action {
            self.apply_action(action);
        }
        self.show_dialogs(ctx);
    }
}

impl RustCatchDesktopApp {
    fn show_dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.open_dialog.take() else {
            return;
        };
        let reopen = dialog.clone();
        let mut keep_open = true;
        let mut next_dialog: Option<UiDialog> = None;

        match dialog {
            UiDialog::AddUrl => {
                let mut url = self.add_url_input.clone();
                let error = self.dialog_error.clone().unwrap_or_default();
                let mut submit = false;
                let mut cancel = false;
                egui::Window::new("添加地址 (Add URL)")
                    .open(&mut keep_open)
                    .resizable(false)
                    .default_width(560.0)
                    .show(ctx, |ui| {
                        ui.label("输入 HTTP/HTTPS 直链或视频页面地址（支持 YouTube watch / shorts / youtu.be）");
                        ui.text_edit_singleline(&mut url);
                        if !error.is_empty() {
                            ui.colored_label(egui::Color32::from_rgb(200, 40, 40), &error);
                        }
                        if !self.add_url_history.is_empty() {
                            ui.separator();
                            ui.label("历史地址：");
                            for item in self.add_url_history.iter().rev().take(5) {
                                ui.label(item);
                            }
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("下一步 (Next)").clicked() {
                                submit = true;
                            }
                            if ui.button("取消").clicked() {
                                cancel = true;
                            }
                        });
                    });
                self.add_url_input = url;
                self.dialog_error = if error.is_empty() { None } else { Some(error) };
                if submit {
                    let submitted_url = self.add_url_input.clone();
                    if self.prepare_add_url(&submitted_url) {
                        next_dialog = Some(UiDialog::DownloadInfo);
                    } else {
                        next_dialog = Some(UiDialog::AddUrl);
                    }
                } else if cancel {
                    keep_open = false;
                    self.dialog_error = None;
                    self.status_message = "已取消添加地址".to_string();
                }
            }
            UiDialog::DownloadInfo => {
                let pending_url = self.pending_url.clone();
                let pending_error = self.dialog_error.clone().unwrap_or_default();
                let mut file_name = self.pending_file_name.clone();
                let mut directory = self.pending_directory.clone();
                let mut category = self.pending_category.clone();
                let mut queue_id = self.pending_queue_id.clone();
                let mut start_now = self.pending_start_now;
                let error = pending_error;
                let mut submit = false;
                let mut cancel = false;
                let categories = [
                    "所有下载 (All Downloads)",
                    "未完成 (Unfinished)",
                    "已完成 (Finished)",
                    "视频 (Video)",
                    "音乐 (Music)",
                    "压缩包 (Compressed)",
                    "文档 (Documents)",
                    "程序 (Programs)",
                ];
                egui::Window::new("下载信息 (Download Info)")
                    .open(&mut keep_open)
                    .resizable(false)
                    .default_width(560.0)
                    .show(ctx, |ui| {
                        ui.label("地址");
                        ui.label(&pending_url);
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("文件名");
                            ui.text_edit_singleline(&mut file_name);
                        });
                        ui.horizontal(|ui| {
                            ui.label("保存目录");
                            ui.text_edit_singleline(&mut directory);
                        });
                        egui::ComboBox::from_label("分类")
                            .selected_text(&category)
                            .show_ui(ui, |ui| {
                                for item in categories {
                                    ui.selectable_value(&mut category, item.to_string(), item);
                                }
                            });
                        egui::ComboBox::from_label("队列")
                            .selected_text(&queue_id)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut queue_id, "main".to_string(), "main");
                                ui.selectable_value(&mut queue_id, "sync".to_string(), "sync");
                            });
                        ui.checkbox(&mut start_now, "立即下载");
                        if !error.is_empty() {
                            ui.colored_label(egui::Color32::from_rgb(200, 40, 40), &error);
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("确定 (OK)").clicked() {
                                submit = true;
                            }
                            if ui.button("取消").clicked() {
                                cancel = true;
                            }
                        });
                    });
                self.pending_file_name = file_name;
                self.pending_directory = directory;
                self.pending_category = category;
                self.pending_queue_id = queue_id;
                self.pending_start_now = start_now;
                self.dialog_error = if error.is_empty() { None } else { Some(error) };
                if submit {
                    if self.confirm_pending_download() {
                        keep_open = false;
                    } else {
                        next_dialog = Some(UiDialog::DownloadInfo);
                    }
                } else if cancel {
                    keep_open = false;
                    self.dialog_error = None;
                    self.status_message = "已取消下载信息确认".to_string();
                }
            }
            UiDialog::Options => {
                let mut close_to_tray = self.close_to_tray;
                let mut close = false;
                egui::Window::new("选项 (Options)")
                    .open(&mut keep_open)
                    .resizable(false)
                    .default_width(420.0)
                    .show(ctx, |ui| {
                        ui.checkbox(&mut close_to_tray, "关闭窗口时最小化到托盘");
                        ui.label("常规 / 文件类型 / 连接 / 保存目录 / 下载 / 代理 / 站点登录 / 声音：配置页将随 P5 继续补齐");
                        if ui.button("关闭").clicked() {
                            close = true;
                        }
                    });
                self.close_to_tray = close_to_tray;
                if close {
                    keep_open = false;
                }
            }
            UiDialog::QueueScheduler => {
                let queue_status = if self.queue_running {
                    "运行中"
                } else {
                    "已停止"
                };
                let task_rows: Vec<String> = self
                    .coordinator
                    .tasks
                    .values()
                    .map(|task| {
                        format!(
                            "{} | {:?} | {}",
                            task.task_id,
                            task.state,
                            task.output
                                .queue_id
                                .clone()
                                .unwrap_or_else(|| "main".to_string())
                        )
                    })
                    .collect();
                let mut start = false;
                let mut stop = false;
                egui::Window::new("计划任务 / 队列")
                    .open(&mut keep_open)
                    .default_width(520.0)
                    .show(ctx, |ui| {
                        ui.label(format!("当前队列状态: {}", queue_status));
                        ui.horizontal(|ui| {
                            if ui.button("开始队列").clicked() {
                                start = true;
                            }
                            if ui.button("停止队列").clicked() {
                                stop = true;
                            }
                        });
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .show(ui, |ui| {
                                for row in task_rows {
                                    ui.label(row);
                                }
                            });
                    });
                if start {
                    self.apply_action(AppAction::StartQueue);
                }
                if stop {
                    self.apply_action(AppAction::StopQueue);
                }
            }
            UiDialog::SiteGrabber => {
                let mut close = false;
                egui::Window::new("站点抓取 (Site Grabber)")
                    .open(&mut keep_open)
                    .resizable(false)
                    .default_width(460.0)
                    .show(ctx, |ui| {
                        ui.label(
                            "四步站点抓取向导状态：视图模型已就绪，实际抓取规则仍按 P5-T02 补齐。",
                        );
                        if ui.button("返回").clicked() {
                            close = true;
                        }
                    });
                if close {
                    keep_open = false;
                }
            }
            UiDialog::About => {
                let mut close = false;
                egui::Window::new("关于 RustCatch")
                    .open(&mut keep_open)
                    .resizable(false)
                    .default_width(420.0)
                    .show(ctx, |ui| {
                        ui.heading("RustCatch Download Manager");
                        ui.label("Rust + egui/eframe 下载管理器原型");
                        ui.label(
                            "编译与测试由 GitHub Actions 执行；GUI 真机行为仍按 needs_env 记录。",
                        );
                        if ui.button("关闭").clicked() {
                            close = true;
                        }
                    });
                if close {
                    keep_open = false;
                }
            }
        }

        if let Some(next) = next_dialog {
            self.open_dialog = Some(next);
        } else if keep_open {
            self.open_dialog = Some(reopen);
        }
    }
}
