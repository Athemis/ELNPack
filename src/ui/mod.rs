// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Top-level egui application shell for composing an ELN entry.
//! Handles layout, form controls, and wiring to archive creation.

pub mod components;

use eframe::egui;

use crate::logic::eln::{ArchiveGenre, ensure_extension, suggested_archive_name};
use crate::mvu::{self, AppModel, Command, Msg};
use crate::ui::components::{attachments, datetime_picker, extra_fields, keywords, markdown};

/// Stateful egui application for building and exporting ELN entries.
pub struct ElnPackApp {
    model: AppModel,
    inbox: Vec<Msg>,
    cmd_tx: crossbeam_channel::Sender<Command>,
    msg_rx: crossbeam_channel::Receiver<Msg>,
}

impl Default for ElnPackApp {
    fn default() -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<Command>();
        let (msg_tx, msg_rx) = crossbeam_channel::unbounded::<Msg>();

        let threads = std::thread::available_parallelism()
            .map(|n| n.get().max(2))
            .unwrap_or(2);
        for _ in 0..threads {
            let cmd_rx = cmd_rx.clone();
            let msg_tx = msg_tx.clone();
            std::thread::spawn(move || {
                for cmd in cmd_rx.iter() {
                    let msg = mvu::run_command(cmd);
                    let _ = msg_tx.send(msg);
                }
            });
        }

        Self {
            model: AppModel {
                archive_genre: ArchiveGenre::Experiment,
                body_format: crate::logic::eln::BodyFormat::Html,
                ..Default::default()
            },
            inbox: Vec::new(),
            cmd_tx,
            msg_rx,
        }
    }
}

impl eframe::App for ElnPackApp {
    /// Main application update loop: processes worker messages, applies MVU updates, and renders the UI.
    ///
    /// This method:
    /// - Drains messages produced by background command workers and enqueues them for processing.
    /// - Processes pending messages, converting decoded thumbnails into UI textures and applying MVU updates which may produce new commands sent to workers.
    /// - Renders the top bar, error modal, bottom status panel, and the central scrolling content (title, meta group, description, keywords, extra fields, and attachments).
    ///
    /// # Parameters
    ///
    /// - `ctx` - The egui context used to build and render UI components.
    /// - `_frame` - The eframe frame provided by the runtime (unused by this implementation).
    ///
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_spacing(ctx);

        // Pull messages produced by the command worker.
        while let Ok(msg) = self.msg_rx.try_recv() {
            self.model.pending_commands = self.model.pending_commands.saturating_sub(1);
            self.inbox.push(msg);
        }

        // Process pending messages until exhausted.
        let mut msgs = std::mem::take(&mut self.inbox);
        while let Some(msg) = msgs.pop() {
            match msg {
                mvu::Msg::ThumbnailDecoded { path, image } => {
                    let texture = ctx.load_texture(
                        format!("thumb-{}", path.display()),
                        image,
                        egui::TextureOptions::default(),
                    );
                    msgs.push(mvu::Msg::ThumbnailReady { path, texture });
                }
                other => {
                    let mut commands = Vec::new();
                    mvu::update(&mut self.model, other, &mut commands);
                    for cmd in commands {
                        if self.cmd_tx.send(cmd).is_ok() {
                            self.model.pending_commands += 1;
                        }
                    }
                }
            }
        }
        self.inbox = msgs;

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("ELN Entry");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.render_theme_controls(ui);
                    ui.separator();
                    self.render_help_button(ui);
                    ui.separator();
                    self.render_save_button(ui);
                    ui.separator();
                    self.render_body_format_toggle(ui);
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

                let kw_msgs = keywords::view(ui, ctx, &self.model.keywords);
                self.inbox.extend(kw_msgs.into_iter().map(Msg::Keywords));
                ui.add_space(12.0);

                self.render_extra_fields_section(ui);
                ui.add_space(12.0);

                self.render_attachments_section(ui, ctx);
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

    /// Render the global theme preference control with a small vertical gap.
    ///
    /// Adds a 2.0-point vertical spacer, then inserts egui's built-in global theme
    /// preference switch into the provided UI.
    ///
    fn render_theme_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        egui::widgets::global_theme_preference_switch(ui);
    }

    /// Render a compact help button that opens the hosted user guide in a browser tab.
    fn render_help_button(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        let button = egui::Button::new(format!("{} Help", egui_phosphor::regular::QUESTION));
        if ui
            .add(button)
            .on_hover_text("Open the ELNPack user guide")
            .clicked()
        {
            self.inbox.push(Msg::OpenHelp);
        }
    }

    /// Renders the "Save ELN archive" button and, when activated, opens a file-save dialog to request saving the current entry.
    ///
    /// The button is enabled only when the entry title is not empty and there are no invalid extra fields. When the user selects a file the chosen path is normalized to have the `.eln` extension and a `Msg::SaveRequested(path)` is queued; if the dialog is cancelled a `Msg::SaveCancelled` is queued.
    ///
    fn render_save_button(&mut self, ui: &mut egui::Ui) {
        let save_enabled = !self.model.entry_title.trim().is_empty()
            && !self.model.extra_fields.has_invalid_fields();
        let button = egui::Button::new(format!(
            "{} Save ELN archive",
            egui_phosphor::regular::FLOPPY_DISK
        ));

        if ui
            .add_enabled(save_enabled, button)
            .on_disabled_hover_text("Please enter a title and fix required/invalid fields")
            .clicked()
        {
            let default_name = suggested_archive_name(&self.model.entry_title);
            let dialog = rfd::FileDialog::new()
                .set_title("Save ELN archive")
                .add_filter("ELN archive", &["eln"])
                .set_file_name(&default_name);

            if let Some(path) = dialog.save_file() {
                let output_path = ensure_extension(path, "eln");
                self.inbox.push(Msg::SaveRequested(output_path));
            } else {
                self.inbox.push(Msg::SaveCancelled);
            }
        }
    }

    /// Render the entry title field.
    fn render_title_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Title");
        ui.add_space(4.0);
        let mut title = self.model.entry_title.clone();
        if ui
            .add(
                egui::TextEdit::singleline(&mut title)
                    .hint_text("e.g., Cell viability assay day 3"),
            )
            .changed()
        {
            self.inbox.push(Msg::EntryTitleChanged(title));
        }
    }

    /// Render the markdown editor field and toolbar.
    fn render_description_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Main Text");
        ui.label(
            egui::RichText::new("Use Markdown to format text.")
                .small()
                .color(egui::Color32::from_gray(110)),
        );
        ui.add_space(4.0);
        let md_msgs = markdown::view(&self.model.markdown, ui);
        self.inbox.extend(md_msgs.into_iter().map(Msg::Markdown));
    }
    fn render_body_format_toggle(&mut self, ui: &mut egui::Ui) {
        let mut choice = self.model.body_format;
        ui.horizontal(|ui| {
            let md_label = format!("{} Markdown", egui_phosphor::regular::MARKDOWN_LOGO);
            ui.selectable_value(
                &mut choice,
                crate::logic::eln::BodyFormat::Markdown,
                md_label,
            )
            .on_hover_text("Store the raw markdown in the archive metadata");
            let html_label = format!("{} HTML", egui_phosphor::regular::FILE_HTML);
            ui.selectable_value(&mut choice, crate::logic::eln::BodyFormat::Html, html_label)
                .on_hover_text("Convert markdown to HTML in the archive metadata");
            ui.label("Export as");
        });
        if choice != self.model.body_format {
            self.inbox.push(Msg::SetBodyFormat(choice));
        }
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
                    let dt_msgs = datetime_picker::view(&self.model.datetime, ui);
                    self.inbox.extend(dt_msgs.into_iter().map(Msg::DateTime));
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

    /// Render attachments as a collapsible section in the main column.
    fn render_attachments_section(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        egui::CollapsingHeader::new("Attachments")
            .default_open(true)
            .show(ui, |ui| {
                let att_msgs = attachments::view(ui, &self.model.attachments);
                self.inbox
                    .extend(att_msgs.into_iter().map(Msg::Attachments));
            });
    }

    /// Renders the "Extra Fields" section of the UI and forwards any produced `Msg::ExtraFields` messages into the app inbox.
    ///
    /// The view is produced by `extra_fields::view` and each returned message is wrapped and appended to `self.inbox`.
    ///
    fn render_extra_fields_section(&mut self, ui: &mut egui::Ui) {
        let msgs = extra_fields::view(ui, &self.model.extra_fields);
        self.inbox.extend(msgs.into_iter().map(Msg::ExtraFields));
    }

    /// Renders a two-button segmented control for selecting the entry's archive genre.
    ///
    /// The currently selected genre is highlighted; clicking a button enqueues a `Msg::SetGenre` corresponding to the chosen genre.
    ///
    fn render_entry_type(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let exp = egui::Button::new("Experiment")
                .selected(matches!(self.model.archive_genre, ArchiveGenre::Experiment));
            if ui.add(exp).clicked() {
                self.inbox.push(Msg::SetGenre(ArchiveGenre::Experiment));
            }
            let res = egui::Button::new("Resource")
                .selected(matches!(self.model.archive_genre, ArchiveGenre::Resource));
            if ui.add(res).clicked() {
                self.inbox.push(Msg::SetGenre(ArchiveGenre::Resource));
            }
        });
    }

    /// Render a simple modal window for error messages.
    fn render_error_modal(&mut self, ctx: &egui::Context) {
        if let Some(message) = self.model.error.clone() {
            egui::Window::new("Validation error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(message);
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        self.inbox.push(Msg::DismissError);
                    }
                });
        }
    }

    /// Render latest status/error message when present.
    fn render_status(&self, ui: &mut egui::Ui) {
        if let Some(text) = &self.model.status {
            let display = if self.model.pending_commands > 0 {
                format!("{}  ({} working…)", text, self.model.pending_commands)
            } else {
                text.to_string()
            };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(display).color(egui::Color32::from_gray(68)));
                if self.model.pending_commands > 0 {
                    ui.add(egui::Spinner::new().size(14.0))
                        .on_hover_text(format!(
                            "{} task(s) running in background",
                            self.model.pending_commands
                        ));
                }
            });
        }
    }
}
