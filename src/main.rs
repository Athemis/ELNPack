mod archive;
mod editor;
mod ui;

use eframe::egui;
use egui_phosphor::Variant;

fn main() -> eframe::Result<()> {
    // Register Phosphor icon font.
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, Variant::Regular);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "elnPack",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(ui::ElnPackApp::default()))
        }),
    )
}
