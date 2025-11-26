//! Application entry point wiring egui/eframe to launch the ELNPack UI.

mod archive;
mod attachments;
mod datetime_picker;
mod editor;
mod keywords;
mod ui;
mod utils;

use eframe::egui;
use egui_phosphor::Variant;

/// Bootstrap the desktop application and run the main egui event loop.
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
        "ELNPack",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(ui::ElnPackApp::default()))
        }),
    )
}
