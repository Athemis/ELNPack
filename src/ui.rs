//! Top-level egui application shell for composing an ELN entry.
//! Handles layout, form controls, and wiring to archive creation.

use std::path::PathBuf;

use chrono::{Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};
use eframe::egui;
use egui_extras::DatePickerButton;
use time::OffsetDateTime;

use crate::archive::{
    ArchiveGenre, build_and_write_archive, ensure_extension, suggested_archive_name,
};
use crate::attachments::AttachmentsPanel;
use crate::editor::MarkdownEditor;

fn format_two(n: i32) -> String {
    format!("{:02}", n.clamp(0, 99))
}

/// Stateful egui application for building and exporting ELN entries.
pub struct ElnPackApp {
    entry_title: String,
    markdown: MarkdownEditor,
    attachments: AttachmentsPanel,
    status_text: String,
    error_modal: Option<String>,
    archive_genre: ArchiveGenre,
    keywords_list: Vec<String>,
    new_keyword_modal_open: bool,
    new_keyword_input: String,
    editing_keyword: Option<usize>,
    editing_buffer: String,
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
            error_modal: None,
            archive_genre: ArchiveGenre::Experiment,
            keywords_list: Vec::new(),
            new_keyword_modal_open: false,
            new_keyword_input: String::new(),
            editing_keyword: None,
            editing_buffer: String::new(),
            performed_date: today,
            performed_hour,
            performed_minute,
        }
    }
}

impl eframe::App for ElnPackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Consistent spacing across the main form.
        ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        });

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

        self.render_error_modal(ctx);
        self.render_add_keyword_modal(ctx);

        egui::SidePanel::right("side_panel")
            .resizable(true)
            .default_width(260.0)
            .width_range(200.0..=360.0)
            .show(ctx, |ui| {
                ui.heading("Attachments");
                ui.add_space(6.0);
                if ui
                    .add(egui::Button::new(egui::RichText::new(format!(
                        "{} Add files",
                        egui_phosphor::regular::PLUS
                    ))))
                    .clicked()
                {
                    if let Some(msg) = self.attachments.add_via_dialog() {
                        self.status_text = msg;
                    }
                }
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                if let Some(msg) = self.attachments.ui(ui) {
                    self.status_text = msg;
                }
            });

        egui::TopBottomPanel::bottom("status_panel")
            .resizable(false)
            .show(ctx, |ui| {
                self.render_status(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_title_input(ui);
                ui.add_space(12.0);

                self.render_meta_group(ui);
                ui.add_space(12.0);

                self.render_description_input(ui);
                ui.add_space(12.0);

                self.render_keywords_section(ui);
                ui.add_space(12.0);

                self.render_action_buttons(ui);
                ui.add_space(8.0);
            });
        });
    }
}

impl ElnPackApp {
    fn render_theme_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        egui::widgets::global_theme_preference_switch(ui);
    }

    /// Build a validated `OffsetDateTime` from the form controls.
    ///
    /// Returns an error string suitable for display when an input is out of range
    /// (e.g., hours outside 0-23 or invalid calendar dates).
    fn build_performed_at(&self) -> Result<OffsetDateTime, String> {
        if !(0..=23).contains(&self.performed_hour) {
            return Err("Hour must be 0-23".into());
        }
        if !(0..=59).contains(&self.performed_minute) {
            return Err("Minute must be 0-59".into());
        }

        let naive = chrono::NaiveDate::from_ymd_opt(
            self.performed_date.year(),
            self.performed_date.month(),
            self.performed_date.day(),
        )
        .and_then(|d| d.and_hms_opt(self.performed_hour as u32, self.performed_minute as u32, 0))
        .ok_or_else(|| "Invalid calendar date or time".to_string())?;

        let local_dt = Local
            .from_local_datetime(&naive)
            .single()
            .ok_or_else(|| "Invalid local date/time (likely skipped by offset)".to_string())?;
        let utc_ts = local_dt.with_timezone(&Utc).timestamp();
        let utc_dt = OffsetDateTime::from_unix_timestamp(utc_ts)
            .map_err(|e| format!("Failed to construct timestamp: {e}"))?;
        Ok(utc_dt)
    }

    /// Render the entry title field.
    fn render_title_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Title");
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.entry_title)
                .hint_text("e.g., Cell viability assay day 3"),
        );
    }

    /// Render the markdown editor field and toolbar.
    fn render_description_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Main Text (Markdown)");
        ui.add_space(4.0);
        self.markdown.ui(ui);
    }

    /// Grouped metadata block with entry type and performed-at controls.
    fn render_meta_group(&mut self, ui: &mut egui::Ui) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("meta_grid")
                .num_columns(2)
                .spacing(egui::vec2(8.0, 10.0))
                .min_col_width(140.0)
                .show(ui, |ui| {
                    ui.label("Entry type");
                    self.render_entry_type(ui);
                    ui.end_row();

                    ui.label("Performed at");
                    self.render_performed_at_compact(ui);
                    ui.end_row();
                });
        });
    }

    /// Render keywords list and controls within the main form.
    fn render_keywords_section(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Keywords")
            .default_open(true)
            .show(ui, |ui| {
                if ui.button("+ Add keyword").clicked() {
                    self.new_keyword_modal_open = true;
                    self.new_keyword_input.clear();
                }

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "Tip: Paste comma-separated keywords in the dialog; they will be split safely.",
                    )
                    .small()
                    .color(egui::Color32::from_gray(110)),
                );

                ui.add_space(8.0);
                let available = ui.available_width();
                let approx_chip_width = 180.0;
                let cols = (available / approx_chip_width).floor().max(1.0) as usize;

                egui::Grid::new("keywords_grid")
                    .num_columns(cols)
                    .spacing(egui::vec2(8.0, 6.0))
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        if self.keywords_list.is_empty() {
                            ui.label(
                                egui::RichText::new("No keywords added yet.")
                                    .italics()
                                    .color(egui::Color32::from_gray(110)),
                            );
                            for _ in 1..cols {
                                ui.label("");
                            }
                            ui.end_row();
                            return;
                        }

                        let mut to_remove: Option<usize> = None;
                        for (i, kw) in self.keywords_list.clone().into_iter().enumerate() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    if self.editing_keyword == Some(i) {
                                        let response = ui.add(
                                            egui::TextEdit::singleline(&mut self.editing_buffer)
                                                .hint_text("Edit keyword")
                                                .desired_width(140.0),
                                        );

                                        let enter = response.lost_focus()
                                            && ui.input(|inp| {
                                                inp.key_pressed(egui::Key::Enter)
                                                    || inp.key_pressed(egui::Key::Tab)
                                            });
                                        if enter {
                                            self.commit_keyword_edit(i);
                                        }

                                        if ui.button("✔").on_hover_text("Save").clicked() {
                                            self.commit_keyword_edit(i);
                                        }

                                        if ui.button("✕").on_hover_text("Cancel").clicked() {
                                            self.editing_keyword = None;
                                            self.editing_buffer.clear();
                                        }
                                    } else {
                                        let chip_resp = ui.add(
                                            egui::Button::new(&kw)
                                                .selected(false)
                                                .wrap()
                                                .min_size(egui::vec2(0.0, 0.0)),
                                        );
                                        if chip_resp.clicked() {
                                            self.editing_keyword = Some(i);
                                            self.editing_buffer = kw.clone();
                                        }

                                        if ui
                                            .button(
                                                egui::RichText::new(egui_phosphor::regular::TRASH_SIMPLE)
                                                    .color(egui::Color32::from_gray(140)),
                                            )
                                            .on_hover_text("Remove keyword")
                                            .clicked()
                                        {
                                            to_remove = Some(i);
                                        }
                                    }
                                });
                            });

                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }

                        if self.keywords_list.len() % cols != 0 {
                            ui.end_row();
                        }

                        if let Some(idx) = to_remove {
                            self.keywords_list.remove(idx);
                            if self.editing_keyword == Some(idx) {
                                self.editing_keyword = None;
                                self.editing_buffer.clear();
                            }
                        }
                    });
            });
    }

    /// Render entry type selection (segmented buttons).
    fn render_entry_type(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let exp = egui::Button::new("Experiment")
                .selected(matches!(self.archive_genre, ArchiveGenre::Experiment));
            if ui.add(exp).clicked() {
                self.archive_genre = ArchiveGenre::Experiment;
            }
            let res = egui::Button::new("Resource")
                .selected(matches!(self.archive_genre, ArchiveGenre::Resource));
            if ui.add(res).clicked() {
                self.archive_genre = ArchiveGenre::Resource;
            }
        });
    }

    /// Compact date/time controls for meta grid.
    fn render_performed_at_compact(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(DatePickerButton::new(&mut self.performed_date).show_icon(true));
            ui.add_space(8.0);
            ui.add(
                egui::DragValue::new(&mut self.performed_hour)
                    .range(0..=23)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            );
            ui.label(":");
            ui.add(
                egui::DragValue::new(&mut self.performed_minute)
                    .range(0..=59)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            );
            ui.add_space(8.0);
            if ui
                .button(egui::RichText::new(format!(
                    "{} Now",
                    egui_phosphor::regular::CLOCK
                )))
                .on_hover_text("Set date/time to your current local time (stored as UTC)")
                .clicked()
            {
                let now = Local::now();
                self.performed_date = now.date_naive();
                self.performed_hour = now.hour() as i32;
                self.performed_minute = now.minute() as i32;
            }
        });
    }

    /// Render a simple modal window for error messages.
    fn render_error_modal(&mut self, ctx: &egui::Context) {
        if let Some(message) = self.error_modal.clone() {
            egui::Window::new("Validation error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(message);
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        self.error_modal = None;
                    }
                });
        }
    }

    /// Render modal to add a single keyword safely.
    fn render_add_keyword_modal(&mut self, ctx: &egui::Context) {
        if !self.new_keyword_modal_open {
            return;
        }

        egui::Window::new("Add keyword(s)")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Keyword(s)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_keyword_input)
                        .hint_text("e.g., microscopy or microscopy, dataset"),
                );

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        let mut added_any = false;
                        let mut saw_duplicate = false;
                        let mut saw_empty_segment = false;

                        for part in self.new_keyword_input.split(',') {
                            let trimmed = part.trim();
                            if trimmed.is_empty() {
                                if !self.new_keyword_input.is_empty() {
                                    saw_empty_segment = true;
                                }
                                continue;
                            }

                            let exists = self
                                .keywords_list
                                .iter()
                                .any(|existing| existing.eq_ignore_ascii_case(trimmed));
                            if exists {
                                saw_duplicate = true;
                                continue;
                            }

                            self.keywords_list.push(trimmed.to_string());
                            added_any = true;
                        }

                        if saw_empty_segment || saw_duplicate {
                            let mut issues = Vec::new();
                            if saw_empty_segment {
                                issues.push("empty entries (extra commas)");
                            }
                            if saw_duplicate {
                                issues.push("duplicates");
                            }
                            let msg = format!(
                                "Some keywords were skipped due to {}.",
                                issues.join(" and ")
                            );
                            self.error_modal = Some(msg);
                        }
                        if added_any && !saw_empty_segment {
                            self.new_keyword_modal_open = false;
                            self.new_keyword_input.clear();
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.new_keyword_modal_open = false;
                        self.new_keyword_input.clear();
                    }
                });
            });
    }

    /// Commit an inline keyword edit with validation.
    fn commit_keyword_edit(&mut self, idx: usize) {
        let new_kw = self.editing_buffer.trim();
        if new_kw.is_empty() {
            self.error_modal = Some("Keyword cannot be empty.".into());
            return;
        }

        let duplicate = self
            .keywords_list
            .iter()
            .enumerate()
            .any(|(i, existing)| i != idx && existing.eq_ignore_ascii_case(new_kw));
        if duplicate {
            self.error_modal = Some("Keyword already exists.".into());
            return;
        }

        if let Some(slot) = self.keywords_list.get_mut(idx) {
            *slot = new_kw.to_string();
        }
        self.editing_keyword = None;
        self.editing_buffer.clear();
    }

    /// Render the file dialog buttons and trigger archive saving.
    fn render_action_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
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

    /// Render latest status/error message when present.
    fn render_status(&self, ui: &mut egui::Ui) {
        if !self.status_text.is_empty() {
            ui.label(egui::RichText::new(&self.status_text).color(egui::Color32::from_gray(68)));
        }
    }

    /// Validate form fields before opening the save dialog.
    ///
    /// Returns trimmed title/body, performed_at timestamp, and parsed keywords.
    /// On failure, returns a user-facing error message.
    fn validate_before_save(
        &mut self,
    ) -> Result<(String, String, OffsetDateTime, Vec<String>), String> {
        let title = self.entry_title.trim().to_string();
        let body = self.markdown.text().trim().to_string();

        if title.is_empty() {
            return Err("Please enter a title.".into());
        }

        let mut keywords_set = std::collections::BTreeSet::new();
        for kw in &self.keywords_list {
            keywords_set.insert(kw.to_string());
        }

        let performed_at = self
            .build_performed_at()
            .map_err(|err| format!("Invalid date/time: {}", err))?;

        let keywords: Vec<String> = keywords_set.into_iter().collect();
        Ok((title, body, performed_at, keywords))
    }

    /// Validate inputs, open the save dialog, and write the archive.
    fn save_archive(&mut self) {
        let (title, body, performed_at, keywords) = match self.validate_before_save() {
            Ok(data) => data,
            Err(msg) => {
                self.status_text = msg.clone();
                self.error_modal = Some(msg);
                return;
            }
        };

        if title.is_empty() {
            // Defensive: should be caught by validation above.
            self.status_text = "Please enter a title.".to_string();
            return;
        }

        let default_name = suggested_archive_name(&title);
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

        match build_and_write_archive(
            &output_path,
            &title,
            &body,
            &attachment_paths,
            performed_at,
            self.archive_genre,
            &keywords,
        ) {
            Ok(_) => {
                self.status_text = format!("Archive saved: {}", output_path.display());
            }
            Err(err) => {
                self.status_text = format!("Error: {}", err);
            }
        }
    }
}
