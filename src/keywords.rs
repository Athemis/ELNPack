//! Keywords editor component with inline editing and modal dialog.

use eframe::egui;

/// Keywords editor with add/edit/remove functionality and modal dialog.
#[derive(Default)]
pub struct KeywordsEditor {
    keywords: Vec<String>,
    modal_open: bool,
    modal_input: String,
    editing_index: Option<usize>,
    editing_buffer: String,
}

impl KeywordsEditor {
    /// Get the current list of keywords.
    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    /// Render the keywords section UI and modal dialog.
    ///
    /// Returns a status message when keywords are added or removed, or an error message
    /// when validation fails (e.g., duplicate or empty keyword during edit).
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> Option<(String, bool)> {
        let mut status = None;

        egui::CollapsingHeader::new("Keywords")
            .default_open(true)
            .show(ui, |ui| {
                if ui
                    .add(egui::Button::new(format!(
                        "{} Add keyword(s)",
                        egui_phosphor::regular::PLUS
                    )))
                    .clicked()
                {
                    self.modal_open = true;
                    self.modal_input.clear();
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
                status = self.render_keywords_grid(ui);
            });

        // Render modal and capture its status/error
        if let Some(modal_result) = self.render_modal(ctx) {
            status = Some(modal_result);
        }

        status
    }

    /// Render the keywords grid with inline editing.
    fn render_keywords_grid(&mut self, ui: &mut egui::Ui) -> Option<(String, bool)> {
        let available = ui.available_width();
        let approx_chip_width = 180.0;
        let cols = (available / approx_chip_width).floor().max(1.0) as usize;

        let mut result = None;

        egui::Grid::new("keywords_grid")
            .num_columns(cols)
            .spacing(egui::vec2(8.0, 6.0))
            .min_col_width(120.0)
            .show(ui, |ui| {
                if self.keywords.is_empty() {
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
                for (i, kw) in self.keywords.clone().into_iter().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            if self.editing_index == Some(i) {
                                result = self.render_editing_keyword(ui, i);
                            } else if self.render_keyword_chip(ui, i, &kw) {
                                to_remove = Some(i);
                            }
                        });
                    });

                    if (i + 1) % cols == 0 {
                        ui.end_row();
                    }
                }

                if !self.keywords.len().is_multiple_of(cols) {
                    ui.end_row();
                }

                if let Some(idx) = to_remove {
                    self.keywords.remove(idx);
                    if self.editing_index == Some(idx) {
                        self.editing_index = None;
                        self.editing_buffer.clear();
                    }
                    result = Some(("Keyword removed".to_string(), false));
                }
            });

        result
    }

    /// Render a keyword chip with click-to-edit and remove button.
    ///
    /// Returns true if the remove button was clicked.
    fn render_keyword_chip(&mut self, ui: &mut egui::Ui, index: usize, keyword: &str) -> bool {
        let chip_resp = ui.add(
            egui::Button::new(keyword)
                .selected(false)
                .wrap()
                .min_size(egui::vec2(0.0, 0.0)),
        );
        if chip_resp.clicked() {
            self.editing_index = Some(index);
            self.editing_buffer = keyword.to_string();
        }

        ui.button(
            egui::RichText::new(egui_phosphor::regular::TRASH_SIMPLE)
                .color(egui::Color32::from_gray(140)),
        )
        .on_hover_text("Remove keyword")
        .clicked()
    }

    /// Render inline editing controls for a keyword.
    ///
    /// Returns an error message if validation fails.
    fn render_editing_keyword(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
    ) -> Option<(String, bool)> {
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.editing_buffer)
                .hint_text("Edit keyword")
                .desired_width(140.0),
        );

        let enter = response.lost_focus()
            && ui.input(|inp| inp.key_pressed(egui::Key::Enter) || inp.key_pressed(egui::Key::Tab));
        if enter {
            return self.commit_edit(index);
        }

        if ui.button("✔").on_hover_text("Save").clicked() {
            return self.commit_edit(index);
        }

        if ui.button("✕").on_hover_text("Cancel").clicked() {
            self.editing_index = None;
            self.editing_buffer.clear();
        }

        None
    }

    /// Render the add keywords modal dialog.
    ///
    /// Returns a status or error message when keywords are added or validation fails.
    fn render_modal(&mut self, ctx: &egui::Context) -> Option<(String, bool)> {
        if !self.modal_open {
            return None;
        }

        let mut result = None;

        egui::Window::new("Add keyword(s)")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Keyword(s)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.modal_input)
                        .hint_text("e.g., microscopy or microscopy, dataset"),
                );

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Add").clicked() {
                        let (message, should_close) = self.process_modal_input();
                        result = Some((message, false));
                        if should_close {
                            self.modal_open = false;
                            self.modal_input.clear();
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.modal_open = false;
                        self.modal_input.clear();
                    }
                });
            });

        result
    }

    /// Process comma-separated input from the modal dialog.
    ///
    /// Returns a status message and whether the modal should close.
    fn process_modal_input(&mut self) -> (String, bool) {
        let mut added_count = 0usize;
        let mut dup_count = 0usize;
        let mut empty_count = 0usize;

        for part in self.modal_input.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                empty_count += 1;
                continue;
            }

            let exists = self
                .keywords
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(trimmed));
            if exists {
                dup_count += 1;
                continue;
            }

            self.keywords.push(trimmed.to_string());
            added_count += 1;
        }

        let mut skipped_parts = Vec::new();
        if dup_count > 0 {
            skipped_parts.push(format!("{dup_count} duplicate(s)"));
        }
        if empty_count > 0 {
            skipped_parts.push(format!("{empty_count} empty entry/entries"));
        }

        let message = match (added_count, skipped_parts.is_empty()) {
            (a, false) if a > 0 => {
                format!(
                    "Added {a} keyword(s); skipped {}.",
                    skipped_parts.join(" and ")
                )
            }
            (a, true) if a > 0 => format!("Added {a} keyword(s)."),
            (_, _) => "No keywords added; skipped duplicates or empty entries.".to_string(),
        };

        (message, added_count > 0)
    }

    /// Commit an inline keyword edit with validation.
    ///
    /// Returns an error message if validation fails.
    fn commit_edit(&mut self, index: usize) -> Option<(String, bool)> {
        let new_kw = self.editing_buffer.trim();
        if new_kw.is_empty() {
            return Some(("Keyword cannot be empty.".into(), true));
        }

        let duplicate = self
            .keywords
            .iter()
            .enumerate()
            .any(|(i, existing)| i != index && existing.eq_ignore_ascii_case(new_kw));
        if duplicate {
            return Some(("Keyword already exists.".into(), true));
        }

        if let Some(slot) = self.keywords.get_mut(index) {
            *slot = new_kw.to_string();
        }
        self.editing_index = None;
        self.editing_buffer.clear();

        None
    }
}
