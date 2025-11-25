use std::path::PathBuf;

use chrono::{Datelike, NaiveDate, Utc};
use eframe::egui;
use egui_extras::DatePickerButton;
use time::{Date, Month, OffsetDateTime, Time};

use crate::archive::{build_and_write_archive, ensure_extension, suggested_archive_name};
use crate::attachments::AttachmentsPanel;
use crate::editor::MarkdownEditor;

fn format_two(n: i32) -> String {
    format!("{:02}", n.clamp(0, 99))
}

pub struct ElnPackApp {
    entry_title: String,
    markdown: MarkdownEditor,
    attachments: AttachmentsPanel,
    status_text: String,
    performed_date: NaiveDate,
    performed_hour: i32,
    performed_minute: i32,
}

impl Default for ElnPackApp {
    fn default() -> Self {
        let now = Utc::now();
        let today = now.date_naive();
        let offset_now = OffsetDateTime::from_unix_timestamp(now.timestamp())
            .expect("Unix timestamp conversion must succeed");

        let performed_hour = offset_now.hour() as i32;
        let performed_minute = offset_now.minute() as i32;

        Self {
            entry_title: String::new(),
            markdown: MarkdownEditor::default(),
            attachments: AttachmentsPanel::default(),
            status_text: String::new(),
            performed_date: today,
            performed_hour,
            performed_minute,
        }
    }
}

impl eframe::App for ElnPackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("ELN Entry");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.render_theme_controls(ui);
                });
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_title_input(ui);
                ui.add_space(12.0);

                self.render_description_input(ui);
                ui.add_space(12.0);

                self.render_performed_at_input(ui);
                ui.add_space(12.0);

                self.render_attachments_section(ui);
                ui.add_space(12.0);

                self.render_action_buttons(ui);
                ui.add_space(8.0);

                self.render_status(ui);
            });
        });
    }
}

impl ElnPackApp {
    fn render_theme_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        egui::widgets::global_theme_preference_switch(ui);
    }

    fn build_performed_at(&self) -> Result<OffsetDateTime, String> {
        let month = Month::try_from(self.performed_date.month() as u8)
            .map_err(|_| "Month must be 1-12".to_string())?;

        let date = Date::from_calendar_date(
            self.performed_date.year(),
            month,
            self.performed_date.day() as u8,
        )
        .map_err(|_| "Invalid calendar date".to_string())?;

        if !(0..=23).contains(&self.performed_hour) {
            return Err("Hour must be 0-23".into());
        }
        if !(0..=59).contains(&self.performed_minute) {
            return Err("Minute must be 0-59".into());
        }

        let time = Time::from_hms(self.performed_hour as u8, self.performed_minute as u8, 0)
            .map_err(|_| "Invalid time".to_string())?;

        Ok(date.with_time(time).assume_utc())
    }

    fn render_title_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Title");
        ui.add_space(4.0);
        ui.text_edit_singleline(&mut self.entry_title);
    }

    fn render_description_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Main Text (Markdown)");
        ui.add_space(4.0);
        self.markdown.ui(ui);
    }

    fn render_performed_at_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Performed at (UTC)");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("📆 Date");
            ui.add(DatePickerButton::new(&mut self.performed_date).show_icon(false));

            ui.label("🕒 Time");
            ui.add(
                egui::DragValue::new(&mut self.performed_hour)
                    .range(0..=23)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            );
            ui.add(
                egui::DragValue::new(&mut self.performed_minute)
                    .range(0..=59)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            );

            if ui.button("Use current time").clicked() {
                let now = Utc::now();
                let offset_now = OffsetDateTime::from_unix_timestamp(now.timestamp())
                    .expect("Unix timestamp conversion must succeed");
                self.performed_date = now.date_naive();
                self.performed_hour = offset_now.hour() as i32;
                self.performed_minute = offset_now.minute() as i32;
            }
        });

        ui.label(
            egui::RichText::new("Example: 2025-11-24 14:05 (UTC)")
                .small()
                .color(egui::Color32::from_gray(120)),
        );
    }

    fn render_attachments_section(&mut self, ui: &mut egui::Ui) {
        if let Some(msg) = self.attachments.ui(ui) {
            self.status_text = msg;
        }
    }

    fn render_action_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Add files").clicked() {
                if let Some(msg) = self.attachments.add_via_dialog() {
                    self.status_text = msg;
                }
            }

            let save_button = egui::Button::new("Save archive");
            let save_enabled = !self.entry_title.trim().is_empty();

            if ui
                .add_enabled(save_enabled, save_button)
                .on_disabled_hover_text("Please enter a title")
                .clicked()
            {
                self.save_archive();
            }
        });
    }

    fn render_status(&self, ui: &mut egui::Ui) {
        if !self.status_text.is_empty() {
            ui.label(egui::RichText::new(&self.status_text).color(egui::Color32::from_gray(68)));
        }
    }

    fn save_archive(&mut self) {
        let title = self.entry_title.trim();
        let body = self.markdown.text().trim();

        let performed_at = match self.build_performed_at() {
            Ok(dt) => dt,
            Err(err) => {
                self.status_text = format!("Invalid date/time: {}", err);
                return;
            }
        };

        if title.is_empty() {
            self.status_text = "Please enter a title.".to_string();
            return;
        }

        let default_name = suggested_archive_name(title);
        let dialog = rfd::FileDialog::new()
            .set_title("Save ELN archive")
            .add_filter("ELN archive", &["eln"])
            .set_file_name(&default_name);

        let Some(selected_path) = dialog.save_file() else {
            self.status_text = "Save cancelled.".to_string();
            return;
        };

        let output_path = ensure_extension(selected_path, "eln");
        let attachment_paths: Vec<PathBuf> = self
            .attachments
            .attachments()
            .iter()
            .map(|a| a.path.clone())
            .collect();

        match build_and_write_archive(&output_path, title, body, &attachment_paths, performed_at) {
            Ok(_) => {
                self.status_text = format!("Archive saved: {}", output_path.display());
            }
            Err(err) => {
                self.status_text = format!("Error: {}", err);
            }
        }
    }
}
