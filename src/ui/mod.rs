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
    /// Drives a single UI frame: processes incoming messages and commands, updates the model, and renders the top bar, status, and central content panels.
    ///
    /// This method:
    /// - Drains messages produced by background command workers into the app inbox and decrements the pending command counter.
    /// - Processes inbox messages, converting decoded thumbnails into egui textures and applying other messages to the MVU model, emitting resulting commands to worker threads and incrementing the pending counter for each sent command.
    /// - Renders the application's top bar (title, theme controls, save button, body-format toggle), a centered error modal when validation errors exist, a bottom status panel, and the scrollable central content (title, metadata, description/editor, keywords, extra fields, and attachments). Subviews may produce additional messages which are appended to the inbox for the next update cycle.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Typical usage is driven by the eframe runtime; this shows the intended call site.
    /// // let mut app = ElnPackApp::default();
    /// // let ctx: egui::Context = /* provided by eframe */ unimplemented!();
    /// // let mut frame: eframe::Frame = /* provided by eframe */ unimplemented!();
    /// // app.update(&ctx, &mut frame);
    /// ```
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

    /// Renders a small spacer followed by the global theme preference switch in the provided UI.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Called from an egui UI context (e.g. inside `eframe::App::update`).
    /// # use eframe::egui;
    /// # fn example(ui: &mut egui::Ui) {
    /// let mut app = crate::ElnPackApp::default();
    /// app.render_theme_controls(ui);
    /// # }
    /// ```
    fn render_theme_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        egui::widgets::global_theme_preference_switch(ui);
    }

    /// Render the "Save ELN archive" button in the top bar and handle the save-file dialog interaction.
    ///
    /// The button is enabled only when the entry title is non-empty (after trimming) and no extra fields are invalid.
    /// When clicked:
    /// - opens a native save dialog filtered for `.eln` files with a suggested archive name derived from the title,
    /// - ensures the selected file path has the `.eln` extension,
    /// - pushes `Msg::SaveRequested(path)` into the app inbox if the user chose a file, or `Msg::SaveCancelled` if the dialog was cancelled.
    /// When disabled, hovering the button shows "Please enter a title and fix required/invalid fields".
    ///
    /// # Examples
    ///
    /// ```
    /// // Illustrates the enabling condition used by the save button.
    /// let mut app = crate::ElnPackApp::default();
    /// // initially title is empty -> save disabled
    /// assert!(app.model.entry_title.trim().is_empty());
    /// assert!(!app.model.extra_fields.has_invalid_fields());
    ///
    /// // set a title -> would enable save (assuming extra fields are valid)
    /// app.model.entry_title = "My experiment".to_string();
    /// let save_enabled = !app.model.entry_title.trim().is_empty()
    ///     && !app.model.extra_fields.has_invalid_fields();
    /// assert!(save_enabled);
    /// ```
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

    /// Renders an "Attachments" collapsible section and forwards messages produced by the attachments view into the app inbox as `Msg::Attachments`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Called from within the app UI code:
    /// // self.render_attachments_section(ui, &ctx);
    /// ```
    fn render_attachments_section(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        egui::CollapsingHeader::new("Attachments")
            .default_open(true)
            .show(ui, |ui| {
                let att_msgs = attachments::view(ui, &self.model.attachments);
                self.inbox
                    .extend(att_msgs.into_iter().map(Msg::Attachments));
            });
    }

    /// Renders the "Extra fields" section and forwards any messages it produces into the application's inbox.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut app = ElnPackApp::default();
    /// // In real usage `ui` is provided by egui during the UI pass.
    /// let mut ui: egui::Ui = unimplemented!();
    /// app.render_extra_fields_section(&mut ui);
    /// ```
    fn render_extra_fields_section(&mut self, ui: &mut egui::Ui) {
        let msgs = extra_fields::view(ui, &self.model.extra_fields);
        self.inbox.extend(msgs.into_iter().map(Msg::ExtraFields));
    }

    /// Render segmented controls to choose the entry's archive genre.
    ///
    /// Displays two side-by-side buttons labeled "Experiment" and "Resource".
    /// The currently selected genre is highlighted; clicking a button enqueues
    /// a `Msg::SetGenre` message with the corresponding `ArchiveGenre`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Inside an egui paint callback with `ui: &mut egui::Ui` and `app: &mut ElnPackApp`:
    /// app.render_entry_type(ui);
    /// ```
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