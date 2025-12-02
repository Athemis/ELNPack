// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Reusable toggle switch widget (adapted from egui demo).

use eframe::egui;

/// Draw a compact toggle switch. Returns the response (clicked toggles the bool).
pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    // Match egui spacing better than hard-coding arbitrary sizes.
    let spacing = ui.style().spacing.interact_size;
    let desired_size = egui::vec2(spacing.x.max(32.0), spacing.y.max(18.0));
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rail = visuals.bg_fill;
        let rail_off = ui.visuals().widgets.inactive.bg_fill;
        let fill = egui::Color32::from_rgba_unmultiplied(
            egui::lerp(rail_off.r() as f32..=rail.r() as f32, how_on) as u8,
            egui::lerp(rail_off.g() as f32..=rail.g() as f32, how_on) as u8,
            egui::lerp(rail_off.b() as f32..=rail.b() as f32, how_on) as u8,
            egui::lerp(rail_off.a() as f32..=rail.a() as f32, how_on) as u8,
        );

        ui.painter()
            .rect_filled(rect.expand(visuals.expansion), rect.height() * 0.45, fill);

        let circle_x = egui::lerp((rect.left() + 8.0)..=(rect.right() - 8.0), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 6.5, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}
