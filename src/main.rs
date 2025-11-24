mod archive;
mod ui;

use eframe::Theme;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        follow_system_theme: true,
        default_theme: Theme::Dark,
        ..Default::default()
    };

    eframe::run_native(
        "elnPack",
        options,
        Box::new(|_cc| Ok(Box::new(ui::ElnPackApp::default()))),
    )
}
