use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::RichText;

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
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
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
                self.insert_snippet_at_cursor("**bold**");
            }
            if ui
                .button(RichText::new(egui_phosphor::regular::TEXT_ITALIC))
                .on_hover_text("Italic")
                .clicked()
            {
                self.insert_snippet_at_cursor("_italic_");
            }
            if ui
                .button(RichText::new(egui_phosphor::regular::TEXT_STRIKETHROUGH))
                .on_hover_text("Strikethrough")
                .clicked()
            {
                self.insert_snippet_at_cursor("~~text~~");
            }
            if ui
                .button(RichText::new(egui_phosphor::regular::TEXT_UNDERLINE))
                .on_hover_text("Underline")
                .clicked()
            {
                self.insert_snippet_at_cursor("<u>text</u>");
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
                        self.insert_snippet_at_cursor("`code`");
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
                        self.insert_block_snippet_at_cursor("```\ncode\n```");
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
                        self.insert_block_snippet_at_cursor("\n- item");
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
                        self.insert_block_snippet_at_cursor("\n1. first");
                    }
                });
            list_resp.response.on_hover_text("List");

            // Other inserts
            if ui
                .button(RichText::new(egui_phosphor::regular::LINK_SIMPLE))
                .on_hover_text("Link")
                .clicked()
            {
                self.insert_snippet_at_cursor("[text](https://example.com)");
            }
            if ui
                .button(RichText::new(egui_phosphor::regular::QUOTES))
                .on_hover_text("Quote")
                .clicked()
            {
                self.insert_block_snippet_at_cursor("\n> quote");
            }
            if ui
                .button(RichText::new(egui_phosphor::regular::IMAGE_SQUARE))
                .on_hover_text("Image")
                .clicked()
            {
                self.insert_snippet_at_cursor("![alt text](path/to/image.png)");
            }
            if ui
                .button(RichText::new(egui_phosphor::regular::RULER))
                .on_hover_text("Rule")
                .clicked()
            {
                self.insert_block_snippet_at_cursor("\n---\n");
            }
        });

        ui.add_space(4.0);
        let body_id = ui.id().with("body_text_edit");
        if let Some(state) = TextEditState::load(ui.ctx(), body_id) {
            self.cursor = state.cursor.char_range();
        }
        let mut output = egui::TextEdit::multiline(&mut self.text)
            .id_source(body_id)
            .desired_width(f32::INFINITY)
            .desired_rows(8)
            .show(ui);
        if let Some(override_range) = self.cursor_override.take() {
            output.state.cursor.set_char_range(Some(override_range.clone()));
            self.cursor = Some(override_range);
        } else {
            self.cursor = output
                .state
                .cursor
                .char_range()
                .or_else(|| Some(CCursorRange::one(CCursor::new(self.text.chars().count()))));
        }
        output.state.store(ui.ctx(), body_id);
    }

    fn insert_snippet_at_cursor(&mut self, snippet: &str) {
        self.insert_at_cursor(snippet, false);
    }

    fn insert_block_snippet_at_cursor(&mut self, snippet: &str) {
        self.insert_at_cursor(snippet, true);
    }

    fn insert_heading_at_cursor(&mut self, level: u8) {
        let level = level.clamp(1, 6);
        let hashes = "#".repeat(level as usize);
        let block = format!("{} Title\n", hashes);
        self.insert_block_snippet_at_cursor(&block);
    }

    fn insert_at_cursor(&mut self, snippet: &str, ensure_newline: bool) {
        let mut insertion = snippet.to_string();
        if ensure_newline {
            if !insertion.ends_with('\n') {
                insertion.push('\n');
            }
            if !self.text.ends_with('\n') && !self.text.is_empty() {
                insertion = format!("\n{insertion}");
            }
        } else if !self.text.is_empty() && !self.text.ends_with(char::is_whitespace) {
            insertion.insert(0, ' ');
        }

        let (start_char, end_char) = if let Some(range) = &self.cursor {
            let sorted = if range.is_sorted() {
                range.clone()
            } else {
                CCursorRange::two(range.secondary, range.primary)
            };
            (sorted.primary.index, sorted.secondary.index)
        } else {
            let len = self.text.chars().count();
            (len, len)
        };

        let start = char_to_byte(&self.text, start_char);
        let end = char_to_byte(&self.text, end_char);

        self.text.replace_range(start..end, &insertion);

        let new_pos = start_char + insertion.chars().count();
        let new_range = CCursorRange::one(CCursor::new(new_pos));
        self.cursor = Some(new_range);
        self.cursor_override = self.cursor.clone();
    }
}

fn char_to_byte(text: &str, char_idx: usize) -> usize {
    if char_idx == text.chars().count() {
        return text.len();
    }
    text.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or_else(|| text.len())
}
