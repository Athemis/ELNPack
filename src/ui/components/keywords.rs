// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Keywords editor refactored to an MVU-friendly shape.

use eframe::egui;

/// UI model for keywords, kept free of side effects.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct KeywordsModel {
    keywords: Vec<String>,
    modal_open: bool,
    modal_input: String,
    editing_index: Option<usize>,
    editing_buffer: String,
}

/// Messages emitted by the keywords view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeywordsMsg {
    OpenModal,
    CloseModal,
    ModalInputChanged(String),
    AddFromModal,
    StartEdit(usize),
    EditInputChanged(String),
    CommitEdit,
    CancelEdit,
    Remove(usize),
}

/// User-facing feedback surfaced to the status bar or error modal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeywordsEvent {
    /// Text shown in the status bar/modal.
    pub message: String,
    /// Whether the message represents an error.
    pub is_error: bool,
}

impl KeywordsModel {
    /// Current keywords as a slice.
    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }
}

/// Apply a message to the model. Returns a feedback event when relevant.
pub fn update(model: &mut KeywordsModel, msg: KeywordsMsg) -> Option<KeywordsEvent> {
    match msg {
        KeywordsMsg::OpenModal => {
            model.modal_open = true;
            model.modal_input.clear();
            None
        }
        KeywordsMsg::CloseModal => {
            model.modal_open = false;
            model.modal_input.clear();
            None
        }
        KeywordsMsg::ModalInputChanged(text) => {
            model.modal_input = text;
            None
        }
        KeywordsMsg::AddFromModal => {
            let (message, added_any) = process_modal_input(model);
            if added_any {
                model.modal_open = false;
                model.modal_input.clear();
            }
            Some(KeywordsEvent {
                message,
                is_error: false,
            })
        }
        KeywordsMsg::StartEdit(index) => {
            model.editing_index = Some(index);
            model.editing_buffer = model.keywords.get(index).cloned().unwrap_or_default();
            None
        }
        KeywordsMsg::EditInputChanged(text) => {
            model.editing_buffer = text;
            None
        }
        KeywordsMsg::CommitEdit => commit_edit(model),
        KeywordsMsg::CancelEdit => {
            model.editing_index = None;
            model.editing_buffer.clear();
            None
        }
        KeywordsMsg::Remove(index) => {
            if index < model.keywords.len() {
                model.keywords.remove(index);
                if model.editing_index == Some(index) {
                    model.editing_index = None;
                    model.editing_buffer.clear();
                }
                return Some(KeywordsEvent {
                    message: "Keyword removed".to_string(),
                    is_error: false,
                });
            }
            None
        }
    }
}

/// Render the keywords UI and return any messages triggered by user interaction.
pub fn view(ui: &mut egui::Ui, ctx: &egui::Context, model: &KeywordsModel) -> Vec<KeywordsMsg> {
    let mut msgs = Vec::new();

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
                msgs.push(KeywordsMsg::OpenModal);
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
            render_keywords_grid(ui, model, &mut msgs);
        });

    if model.modal_open {
        render_modal(ctx, model, &mut msgs);
    }

    msgs
}

/// Display keywords in a responsive grid, wiring chip actions into messages.
fn render_keywords_grid(ui: &mut egui::Ui, model: &KeywordsModel, msgs: &mut Vec<KeywordsMsg>) {
    let available = ui.available_width();
    let approx_chip_width = 180.0;
    let cols = (available / approx_chip_width).floor().max(1.0) as usize;

    egui::Grid::new("keywords_grid")
        .num_columns(cols)
        .spacing(egui::vec2(8.0, 6.0))
        .min_col_width(120.0)
        .show(ui, |ui| {
            if model.keywords.is_empty() {
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

            for (i, kw) in model.keywords.iter().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        if model.editing_index == Some(i) {
                            render_editing_keyword(ui, model, msgs);
                        } else {
                            render_keyword_chip(ui, i, kw, msgs);
                        }
                    });
                });

                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }

            if !model.keywords.len().is_multiple_of(cols) {
                ui.end_row();
            }
        });
}

/// Render a single keyword chip with inline delete/edit affordances.
fn render_keyword_chip(
    ui: &mut egui::Ui,
    index: usize,
    keyword: &str,
    msgs: &mut Vec<KeywordsMsg>,
) {
    let chip_resp = ui.add(
        egui::Button::new(keyword)
            .selected(false)
            .wrap()
            .min_size(egui::vec2(0.0, 0.0)),
    );
    if chip_resp.clicked() {
        // Signal caller to start editing this keyword.
        ui.memory_mut(|mem| mem.request_focus(chip_resp.id));
        msgs.push(KeywordsMsg::StartEdit(index));
    }

    if ui
        .button(
            egui::RichText::new(egui_phosphor::regular::TRASH_SIMPLE)
                .color(egui::Color32::from_gray(140)),
        )
        .on_hover_text("Remove keyword")
        .clicked()
    {
        msgs.push(KeywordsMsg::Remove(index));
    }
}

/// Render the inline editing UI for a keyword row.
fn render_editing_keyword(ui: &mut egui::Ui, model: &KeywordsModel, msgs: &mut Vec<KeywordsMsg>) {
    let mut buffer = model.editing_buffer.clone();
    let response = ui.add(
        egui::TextEdit::singleline(&mut buffer)
            .hint_text("Edit keyword")
            .desired_width(140.0),
    );

    if response.changed() {
        msgs.push(KeywordsMsg::EditInputChanged(buffer.clone()));
    }

    let enter = response.lost_focus()
        && ui.input(|inp| inp.key_pressed(egui::Key::Enter) || inp.key_pressed(egui::Key::Tab));
    if enter {
        msgs.push(KeywordsMsg::CommitEdit);
        return;
    }

    if ui
        .button(egui_phosphor::regular::CHECK)
        .on_hover_text("Save")
        .clicked()
    {
        msgs.push(KeywordsMsg::CommitEdit);
    }

    if ui
        .button(egui_phosphor::regular::X)
        .on_hover_text("Cancel")
        .clicked()
    {
        msgs.push(KeywordsMsg::CancelEdit);
    }
}

/// Show the add-keywords modal window when requested.
fn render_modal(ctx: &egui::Context, model: &KeywordsModel, msgs: &mut Vec<KeywordsMsg>) {
    let mut input = model.modal_input.clone();

    egui::Window::new("Add keyword(s)")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label("Keyword(s)");
            let resp = ui.add(
                egui::TextEdit::singleline(&mut input)
                    .hint_text("e.g., microscopy or microscopy, dataset"),
            );

            if resp.changed() {
                msgs.push(KeywordsMsg::ModalInputChanged(input.clone()));
            }

            // Add keywords on Enter key for better UX
            if resp.lost_focus() && ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                msgs.push(KeywordsMsg::AddFromModal);
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Add").clicked() {
                    msgs.push(KeywordsMsg::AddFromModal);
                }

                if ui.button("Cancel").clicked() {
                    msgs.push(KeywordsMsg::CloseModal);
                }
            });
        });
}

/// Split modal input on commas, add unique keywords, and return a status message plus added flag.
fn process_modal_input(model: &mut KeywordsModel) -> (String, bool) {
    let mut added_count = 0usize;
    let mut dup_count = 0usize;
    let mut empty_count = 0usize;

    for part in model.modal_input.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            empty_count += 1;
            continue;
        }

        let exists = model
            .keywords
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(trimmed));
        if exists {
            dup_count += 1;
            continue;
        }

        model.keywords.push(trimmed.to_string());
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

/// Validate and commit an inline keyword edit, returning a feedback event on error.
fn commit_edit(model: &mut KeywordsModel) -> Option<KeywordsEvent> {
    let index = model.editing_index?;
    let new_kw = model.editing_buffer.trim();
    if new_kw.is_empty() {
        return Some(KeywordsEvent {
            message: "Keyword cannot be empty.".into(),
            is_error: true,
        });
    }

    let duplicate = model
        .keywords
        .iter()
        .enumerate()
        .any(|(i, existing)| i != index && existing.eq_ignore_ascii_case(new_kw));
    if duplicate {
        return Some(KeywordsEvent {
            message: "Keyword already exists.".into(),
            is_error: true,
        });
    }

    if let Some(slot) = model.keywords.get_mut(index) {
        *slot = new_kw.to_string();
    }
    model.editing_index = None;
    model.editing_buffer.clear();

    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::*;

    #[test]
    fn add_from_modal_adds_and_flags_duplicates() {
        let mut model = KeywordsModel::default();
        model.modal_open = true;
        model.modal_input = "microscopy, microscopy, , dataset".into();

        let event = update(&mut model, KeywordsMsg::AddFromModal).expect("event expected");

        assert_eq!(model.keywords, vec!["microscopy", "dataset"]);
        assert!(!event.is_error); // added at least one
    }

    #[test]
    fn commit_edit_rejects_duplicates() {
        let mut model = KeywordsModel {
            keywords: vec!["one".into(), "two".into()],
            modal_open: false,
            modal_input: String::new(),
            editing_index: Some(0),
            editing_buffer: "two".into(),
        };

        let event = commit_edit(&mut model).expect("should return error event");

        assert!(event.is_error);
        assert_eq!(event.message, "Keyword already exists.");
        assert_eq!(model.keywords, vec!["one", "two"]);
    }

    #[test]
    fn remove_keyword_updates_model() {
        let mut model = KeywordsModel {
            keywords: vec!["one".into(), "two".into()],
            ..Default::default()
        };

        let event = update(&mut model, KeywordsMsg::Remove(0)).expect("event expected");

        assert_eq!(model.keywords, vec!["two"]);
        assert_eq!(event.message, "Keyword removed");
    }
}
