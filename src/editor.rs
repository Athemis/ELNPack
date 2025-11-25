//! Markdown editor widget with toolbar helpers for common formatting.

use eframe::egui;
use egui::RichText;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CodeChoice {
    Inline,
    Block,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListChoice {
    Unordered,
    Ordered,
}

/// Rich-text-like markdown editor state and behaviors.
pub struct MarkdownEditor {
    text: String,
    heading_level: u8,
    cursor: Option<CCursorRange>,
    cursor_override: Option<CCursorRange>,
    code_choice: CodeChoice,
    list_choice: ListChoice,
}

impl Default for MarkdownEditor {
    fn default() -> Self {
        Self {
            text: String::new(),
            heading_level: 1,
            cursor: None,
            cursor_override: None,
            code_choice: CodeChoice::Inline,
            list_choice: ListChoice::Unordered,
        }
    }
}

impl MarkdownEditor {
    /// Current editor contents as plain text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Render the toolbar and text area, applying cursor-aware insertions.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                // Headings
                let heading_resp = egui::ComboBox::from_id_salt("heading_picker")
                    .width(40.0)
                    .selected_text(RichText::new(format!("H{}", self.heading_level)).strong())
                    .show_ui(ui, |ui| {
                        for lvl in 1..=6u8 {
                            if ui
                                .selectable_value(&mut self.heading_level, lvl, format!("H{}", lvl))
                                .clicked()
                            {
                                self.insert_heading_at_cursor(lvl);
                            }
                        }
                    });
                heading_resp.response.on_hover_text("Heading");
                ui.separator();

                // Inline styles
                if ui
                    .button(RichText::new(egui_phosphor::regular::TEXT_BOLDER))
                    .on_hover_text("Bold")
                    .clicked()
                {
                    self.apply_style("**", "**", "bold", false);
                }
                if ui
                    .button(RichText::new(egui_phosphor::regular::TEXT_ITALIC))
                    .on_hover_text("Italic")
                    .clicked()
                {
                    self.apply_style("_", "_", "italic", false);
                }
                if ui
                    .button(RichText::new(egui_phosphor::regular::TEXT_STRIKETHROUGH))
                    .on_hover_text("Strikethrough")
                    .clicked()
                {
                    self.apply_style("~~", "~~", "text", false);
                }
                if ui
                    .button(RichText::new(egui_phosphor::regular::TEXT_UNDERLINE))
                    .on_hover_text("Underline")
                    .clicked()
                {
                    self.apply_style("<u>", "</u>", "text", false);
                }

                // Code
                let code_resp = egui::ComboBox::from_id_salt("code_picker")
                    .width(40.0)
                    .selected_text(match self.code_choice {
                        CodeChoice::Inline => RichText::new(egui_phosphor::regular::CODE_SIMPLE),
                        CodeChoice::Block => RichText::new(egui_phosphor::regular::CODE_BLOCK),
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut self.code_choice,
                                CodeChoice::Inline,
                                RichText::new(egui_phosphor::regular::CODE_SIMPLE),
                            )
                            .on_hover_text("Inline code")
                            .clicked()
                        {
                            self.apply_style("`", "`", "code", false);
                        }
                        if ui
                            .selectable_value(
                                &mut self.code_choice,
                                CodeChoice::Block,
                                RichText::new(egui_phosphor::regular::CODE_BLOCK),
                            )
                            .on_hover_text("Code block")
                            .clicked()
                        {
                            self.apply_style("```\n", "\n```", "code", true);
                        }
                    });
                code_resp.response.on_hover_text("Code");

                // Lists
                let list_resp = egui::ComboBox::from_id_salt("list_picker")
                    .width(40.0)
                    .selected_text(match self.list_choice {
                        ListChoice::Unordered => RichText::new(egui_phosphor::regular::LIST_DASHES),
                        ListChoice::Ordered => RichText::new(egui_phosphor::regular::LIST_NUMBERS),
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut self.list_choice,
                                ListChoice::Unordered,
                                RichText::new(egui_phosphor::regular::LIST_DASHES),
                            )
                            .on_hover_text("Bulleted list")
                            .clicked()
                        {
                            self.apply_style("\n- ", "", "item", true);
                        }
                        if ui
                            .selectable_value(
                                &mut self.list_choice,
                                ListChoice::Ordered,
                                RichText::new(egui_phosphor::regular::LIST_NUMBERS),
                            )
                            .on_hover_text("Numbered list")
                            .clicked()
                        {
                            self.apply_style("\n1. ", "", "first", true);
                        }
                    });
                list_resp.response.on_hover_text("List");

                // Other inserts
                if ui
                    .button(RichText::new(egui_phosphor::regular::LINK_SIMPLE))
                    .on_hover_text("Link")
                    .clicked()
                {
                    self.apply_style("[", "](https://example.com)", "text", false);
                }
                if ui
                    .button(RichText::new(egui_phosphor::regular::QUOTES))
                    .on_hover_text("Quote")
                    .clicked()
                {
                    self.apply_style("\n> ", "", "quote", true);
                }
                if ui
                    .button(RichText::new(egui_phosphor::regular::IMAGE_SQUARE))
                    .on_hover_text("Image")
                    .clicked()
                {
                    self.apply_style("![", "](path/to/image.png)", "alt text", false);
                }
                if ui
                    .button(RichText::new(egui_phosphor::regular::RULER))
                    .on_hover_text("Rule")
                    .clicked()
                {
                    self.apply_style("\n---\n", "", "", true);
                }
            });

            ui.add_space(4.0);

            egui::Resize::default()
                .id_salt("markdown_editor_resize")
                .resizable([false, true])
                .default_size([ui.available_width(), 200.0])
                .min_size([ui.available_width(), 100.0])
                .max_size([ui.available_width(), f32::INFINITY])
                .show(ui, |ui| {
                    let body_id = ui.id().with("body_text_edit");

                    // Load existing state before rendering
                    if let Some(state) = TextEditState::load(ui.ctx(), body_id) {
                        self.cursor = state.cursor.char_range();
                    }

                    // Render the TextEdit filling available space
                    ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::multiline(&mut self.text)
                            .code_editor()
                            .id_source(body_id),
                    );

                    // Load state again after rendering to capture any changes
                    if let Some(mut state) = TextEditState::load(ui.ctx(), body_id) {
                        // Apply cursor override if we set one during toolbar actions
                        if let Some(override_range) = self.cursor_override.take() {
                            state.cursor.set_char_range(Some(override_range));
                            self.cursor = Some(override_range);
                            state.store(ui.ctx(), body_id);
                        } else {
                            // Update our internal cursor from the TextEdit's state
                            self.cursor = state.cursor.char_range().or_else(|| {
                                Some(CCursorRange::one(CCursor::new(self.text.chars().count())))
                            });
                        }
                    }
                });
        });
    }

    fn insert_heading_at_cursor(&mut self, level: u8) {
        let level = level.clamp(1, 6);
        let hashes = "#".repeat(level as usize);
        let (_, _, selected) = self.selection();
        let cleaned = selected
            .trim()
            .trim_start_matches('#')
            .trim_start()
            .to_string();
        let content = if cleaned.is_empty() {
            "Title"
        } else {
            cleaned.as_str()
        };

        self.apply_style(&format!("{} ", hashes), "\n", content, true);
    }

    fn selection(&self) -> (usize, usize, String) {
        let (start_char, end_char) = if let Some(range) = &self.cursor {
            let (a, b) = (range.primary.index, range.secondary.index);
            (a.min(b), a.max(b))
        } else {
            let len = self.text.chars().count();
            (len, len)
        };

        let selected = if start_char < end_char {
            self.text
                .chars()
                .skip(start_char)
                .take(end_char - start_char)
                .collect::<String>()
        } else {
            String::new()
        };

        (start_char, end_char, selected)
    }

    fn apply_style(&mut self, prefix: &str, suffix: &str, placeholder: &str, ensure_newline: bool) {
        let (start_char, end_char, selected) = self.selection();

        let mut insertion = format!(
            "{}{}{}",
            prefix,
            if selected.is_empty() {
                placeholder
            } else {
                &selected
            },
            suffix
        );

        if ensure_newline {
            if !insertion.ends_with('\n') {
                insertion.push('\n');
            }
            if !self.text.ends_with('\n') && !self.text.is_empty() {
                insertion = format!("\n{insertion}");
            }
        } else if selected.is_empty()
            && !self.text.is_empty()
            && start_char == end_char
            && !self.text[..char_to_byte(&self.text, start_char)].ends_with(char::is_whitespace)
        {
            insertion.insert(0, ' ');
        }

        let start = char_to_byte(&self.text, start_char);
        let end = char_to_byte(&self.text, end_char);

        self.text.replace_range(start..end, &insertion);

        let new_pos = start_char + insertion.chars().count();
        let new_range = CCursorRange::one(CCursor::new(new_pos));
        self.cursor = Some(new_range);
        self.cursor_override = self.cursor;
    }
}

/// Convert a character index to a byte index, clamping to the end when out of bounds.
fn char_to_byte(text: &str, char_idx: usize) -> usize {
    if char_idx == text.chars().count() {
        return text.len();
    }
    text.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or_else(|| text.len())
}
