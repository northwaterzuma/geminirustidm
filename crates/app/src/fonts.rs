//! 中文字体加载：egui 默认字体（ProggyClean/Hack）不含 CJK 字形，
//! 不注册中文字体时菜单/状态栏的中文会显示为豆腐块或乱码。
//! 打包字体为 Noto Sans SC（OFL-1.1）的 GB2312 常用字子集，保证离线可用。

use eframe::egui::{FontData, FontDefinitions, FontFamily};

const CJK_FONT: &[u8] = include_bytes!("../../../assets/fonts/NotoSansSC-subset.ttf");

/// 把中文字体设为 Proportional 与 Monospace 的首选字体。
pub fn install_chinese_font(ctx: &eframe::egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("NotoSansSC".to_owned(), FontData::from_static(CJK_FONT));

    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let list = fonts.families.entry(family).or_default();
        list.insert(0, "NotoSansSC".to_owned());
    }

    ctx.set_fonts(fonts);
}
