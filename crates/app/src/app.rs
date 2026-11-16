use crate::ui::RustCatchDesktopApp;
use eframe::egui;

impl eframe::App for RustCatchDesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("idm_menu_and_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("任务(T)  文件(F)  下载(D)  查看(V)  帮助(H)");
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.add_url_input);
                if ui.button("添加地址 (Add URL)").clicked() && !self.add_url_input.trim().is_empty() {
                    self.status_message = format!("已打开下载信息窗: {}", self.add_url_input);
                }
                if ui.button("恢复 (Resume)").clicked() {
                    self.status_message = "已发送恢复指令".to_string();
                }
                if ui.button("停止 (Stop)").clicked() {
                    self.status_message = "已发送暂停指令".to_string();
                }
                if ui.button("删除记录 (Delete)").clicked() {
                    self.handle_delete_key(false, 1_794_815_091_000);
                }
                ui.label("选项 | 计划任务 | 开始队列 | 停止队列 | 站点抓取");
            });
        });

        egui::SidePanel::left("idm_category_tree")
            .exact_width(180.0)
            .show(ctx, |ui| {
                ui.heading("分类 (Categories)");
                for cat in [
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
                    if ui.selectable_label(self.selected_category == cat, cat).clicked() {
                        self.selected_category = cat.to_string();
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                for task in self.coordinator.tasks.values() {
                    let snap = task.to_snapshot();
                    ui.horizontal(|ui| {
                        ui.label(&snap.display_name);
                        ui.label(format!("{:?}", snap.state));
                        ui.label(format!("{} B", snap.bytes_received));
                        ui.label(&snap.source_display);
                    });
                }
            });
        });
    }
}
