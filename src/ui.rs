//! Top-level egui application shell for composing an ELN entry.
//! Handles layout, form controls, and wiring to archive creation.

use eframe::egui;
use time::OffsetDateTime;

use crate::archive::{
    ArchiveGenre, AttachmentMeta, build_and_write_archive, ensure_extension, suggested_archive_name,
};
use crate::attachments::AttachmentsPanel;
use crate::datetime_picker::DateTimePicker;
use crate::editor::MarkdownEditor;
use crate::keywords::KeywordsEditor;

/// Stateful egui application for building and exporting ELN entries.
pub struct ElnPackApp {
    entry_title: String,
    markdown: MarkdownEditor,
    attachments: AttachmentsPanel,
    keywords: KeywordsEditor,
    datetime: DateTimePicker,
    status_text: String,
    error_modal: Option<String>,
    archive_genre: ArchiveGenre,
}

impl Default for ElnPackApp {
    fn default() -> Self {
        Self {
            entry_title: String::new(),
            markdown: MarkdownEditor::default(),
            attachments: AttachmentsPanel::default(),
            keywords: KeywordsEditor::default(),
            datetime: DateTimePicker::default(),
            status_text: String::new(),
            error_modal: None,
            archive_genre: ArchiveGenre::Experiment,
        }
    }
}

impl eframe::App for ElnPackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_spacing(ctx);

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

                if let Some((msg, is_error)) = self.keywords.ui(ui, ctx) {
                    if is_error {
                        self.error_modal = Some(msg);
                    } else {
                        self.status_text = msg;
                    }
                }
                ui.add_space(12.0);

                self.render_attachments_section(ui);
                ui.add_space(12.0);

                self.render_action_buttons(ui);
                ui.add_space(8.0);
            });
        });
    }
}

impl ElnPackApp {
    fn ensure_spacing(&self, ctx: &egui::Context) {
        ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        });
    }

    fn render_theme_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        egui::widgets::global_theme_preference_switch(ui);
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
                    self.datetime.ui(ui);
                    ui.end_row();
                });

            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Times are shown in your local time zone and stored as UTC in the archive.",
                )
                .small()
                .color(egui::Color32::from_gray(110)),
            );
        });
    }

    /// Helper that formats a label with a PLUS icon prefix.
    fn plus_label(&self, text: &str) -> egui::RichText {
        egui::RichText::new(format!("{} {}", egui_phosphor::regular::PLUS, text))
    }

    /// Render attachments as a collapsible section in the main column.
    fn render_attachments_section(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Attachments")
            .default_open(true)
            .show(ui, |ui| {
                if ui
                    .add(egui::Button::new(self.plus_label("Add files")))
                    .on_hover_text("Add files")
                    .clicked()
                    && let Some(msg) = self.attachments.add_via_dialog()
                {
                    self.status_text = msg;
                }

                ui.add_space(6.0);
                if let Some(msg) = self.attachments.ui(ui) {
                    self.status_text = msg;
                }
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
        for kw in self.keywords.keywords() {
            keywords_set.insert(kw.to_string());
        }

        let performed_at = self
            .datetime
            .to_offset_datetime()
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
        let attachment_meta: Vec<AttachmentMeta> = self
            .attachments
            .attachments()
            .iter()
            .map(|a| AttachmentMeta {
                path: a.path.clone(),
                mime: a.mime.clone(),
                sha256: a.sha256.clone(),
                size: a.size,
            })
            .collect();

        match build_and_write_archive(
            &output_path,
            &title,
            &body,
            &attachment_meta,
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
