use mdm_app::RustCatchDesktopApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RustCatch Download Manager")
            .with_inner_size([1024.0, 640.0])
            .with_min_inner_size([720.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RustCatch",
        native_options,
        Box::new(|cc| {
            mdm_app::fonts::install_chinese_font(&cc.egui_ctx);
            Ok(Box::new(RustCatchDesktopApp::default()))
        }),
    )
}
