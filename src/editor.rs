//! Markdown editor widget with toolbar helpers for common formatting.

use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui_phosphor::regular;

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum MathChoice {
    Inline,
    Display,
}

/// Rich-text-like markdown editor state and behaviors.
pub struct MarkdownEditor {
    text: String,
    heading_level: u8,
    cursor: Option<CCursorRange>,
    cursor_override: Option<CCursorRange>,
    code_choice: CodeChoice,
    list_choice: ListChoice,
    math_choice: MathChoice,
    table_rows: u8,
    table_cols: u8,
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
            math_choice: MathChoice::Inline,
            table_rows: 2,
            table_cols: 2,
        }
    }
}

impl MarkdownEditor {
    /// Current editor contents as plain text.
    pub fn text(&self) -> &str {
        &self.text
    }

    fn heading_icon(level: u8) -> &'static str {
        match level {
            1 => regular::TEXT_H_ONE,
            2 => regular::TEXT_H_TWO,
            3 => regular::TEXT_H_THREE,
            4 => regular::TEXT_H_FOUR,
            5 => regular::TEXT_H_FIVE,
            6 => regular::TEXT_H_SIX,
            _ => regular::TEXT_H,
        }
    }

    /// Render the toolbar and text area, applying cursor-aware insertions.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                // Headings
                let heading_resp = egui::ComboBox::from_id_salt("heading_picker")
                    .width(40.0)
                    .selected_text(Self::heading_icon(self.heading_level))
                    .show_ui(ui, |ui| {
                        for lvl in 1..=6u8 {
                            if ui
                                .selectable_value(
                                    &mut self.heading_level,
                                    lvl,
                                    Self::heading_icon(lvl),
                                )
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
                    .button(egui_phosphor::regular::TEXT_BOLDER)
                    .on_hover_text("Bold")
                    .clicked()
                {
                    self.apply_style("**", "**", "bold", false);
                }
                if ui
                    .button(egui_phosphor::regular::TEXT_ITALIC)
                    .on_hover_text("Italic")
                    .clicked()
                {
                    self.apply_style("_", "_", "italic", false);
                }
                if ui
                    .button(egui_phosphor::regular::TEXT_STRIKETHROUGH)
                    .on_hover_text("Strikethrough")
                    .clicked()
                {
                    self.apply_style("~~", "~~", "text", false);
                }
                if ui
                    .button(egui_phosphor::regular::TEXT_UNDERLINE)
                    .on_hover_text("Underline")
                    .clicked()
                {
                    self.apply_style("<u>", "</u>", "text", false);
                }

                // Code
                let code_resp = egui::ComboBox::from_id_salt("code_picker")
                    .width(40.0)
                    .selected_text(match self.code_choice {
                        CodeChoice::Inline => egui_phosphor::regular::CODE_SIMPLE,
                        CodeChoice::Block => egui_phosphor::regular::CODE_BLOCK,
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut self.code_choice,
                                CodeChoice::Inline,
                                egui_phosphor::regular::CODE_SIMPLE,
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
                                egui_phosphor::regular::CODE_BLOCK,
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
                        ListChoice::Unordered => egui_phosphor::regular::LIST_DASHES,
                        ListChoice::Ordered => egui_phosphor::regular::LIST_NUMBERS,
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut self.list_choice,
                                ListChoice::Unordered,
                                egui_phosphor::regular::LIST_DASHES,
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
                                egui_phosphor::regular::LIST_NUMBERS,
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
                    .button(egui_phosphor::regular::LINK_SIMPLE)
                    .on_hover_text("Link")
                    .clicked()
                {
                    self.apply_style("[", "](https://example.com)", "text", false);
                }
                if ui
                    .button(egui_phosphor::regular::QUOTES)
                    .on_hover_text("Quote")
                    .clicked()
                {
                    self.apply_style("\n> ", "", "quote", true);
                }
                if ui
                    .button(egui_phosphor::regular::IMAGE_SQUARE)
                    .on_hover_text("Image")
                    .clicked()
                {
                    self.apply_style("![", "](path/to/image.png)", "alt text", false);
                }
                let table_resp = egui::ComboBox::from_id_salt("table_picker")
                    .width(80.0)
                    .selected_text(format!(
                        "{} {}×{}",
                        egui_phosphor::regular::TABLE,
                        self.table_rows,
                        self.table_cols
                    ))
                    .show_ui(ui, |ui| {
                        self.table_size_picker(ui);
                    });
                table_resp
                    .response
                    .on_hover_text("Insert table (choose size)");
                if ui
                    .button(egui_phosphor::regular::RULER)
                    .on_hover_text("Rule")
                    .clicked()
                {
                    self.apply_style("\n---\n", "", "", true);
                }
                // Math
                let math_resp = egui::ComboBox::from_id_salt("math_picker")
                    .width(40.0)
                    .selected_text(match self.math_choice {
                        MathChoice::Inline => format!("{} $", egui_phosphor::regular::FUNCTION),
                        MathChoice::Display => format!("{} $$", egui_phosphor::regular::FUNCTION),
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut self.math_choice,
                                MathChoice::Inline,
                                format!("{} $", egui_phosphor::regular::FUNCTION),
                            )
                            .on_hover_text("Inline math")
                            .clicked()
                        {
                            self.apply_style("$", "$", "a+b=c", true);
                        }
                        if ui
                            .selectable_value(
                                &mut self.math_choice,
                                MathChoice::Display,
                                format!("{} $$", egui_phosphor::regular::FUNCTION),
                            )
                            .on_hover_text("Display math")
                            .clicked()
                        {
                            self.apply_style("$$", "$$", "a+b=c", true);
                        }
                    });
                math_resp.response.on_hover_text("Math");
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

                    // Calculate desired rows based on available height
                    let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
                    let desired_rows = (ui.available_height() / line_height).max(1.0) as usize;

                    // Render the TextEdit filling available space and capture output
                    let mut output = egui::TextEdit::multiline(&mut self.text)
                        .code_editor()
                        .id_source(body_id)
                        .desired_width(f32::INFINITY)
                        .desired_rows(desired_rows)
                        .show(ui);

                    // Apply cursor override if we set one during toolbar actions
                    if let Some(override_range) = self.cursor_override.take() {
                        output.state.cursor.set_char_range(Some(override_range));
                        self.cursor = Some(override_range);
                    } else {
                        // Update our internal cursor from the TextEdit's state
                        self.cursor = output.state.cursor.char_range().or_else(|| {
                            Some(CCursorRange::one(CCursor::new(self.text.chars().count())))
                        });
                    }
                    output.state.store(ui.ctx(), body_id);
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

    fn table_snippet(rows: u8, cols: u8) -> String {
        let rows = rows.max(1);
        let cols = cols.max(1);

        let mut s = String::new();

        // Precompute header labels and their individual widths.
        let mut header_labels = Vec::new();
        let mut column_widths = Vec::new();
        for col in 1..=cols {
            let label = format!("Column {}", col);
            // One leading space before the label and one trailing space after it.
            let width = label.len() + 2;
            header_labels.push(label);
            column_widths.push(width);
        }

        // Header row
        for (label, &width) in header_labels.iter().zip(column_widths.iter()) {
            s.push('|');
            // Leading space + label + trailing padding spaces to fill this column's width.
            s.push(' ');
            s.push_str(label);
            let used = 1 + label.len();
            if width > used {
                for _ in 0..(width - used) {
                    s.push(' ');
                }
            }
        }
        s.push('|');
        s.push('\n');

        // Separator row
        for &width in &column_widths {
            s.push('|');
            for _ in 0..width {
                s.push('-');
            }
        }
        s.push('|');
        s.push('\n');

        // Body rows
        for _ in 1..=rows {
            for &width in &column_widths {
                s.push('|');
                for _ in 0..width {
                    s.push(' ');
                }
            }
            s.push('|');
            s.push('\n');
        }

        s.push('\n');
        s
    }

    fn insert_table_at_cursor(&mut self, rows: u8, cols: u8) {
        let (_, end_char, _) = self.selection();

        let insert_byte = char_to_byte(&self.text, end_char);
        let mut insertion = String::new();

        if !self.text.is_empty() && (insert_byte == 0 || !self.text[..insert_byte].ends_with('\n'))
        {
            insertion.push('\n');
        }

        insertion.push_str(&Self::table_snippet(rows, cols));

        self.text.insert_str(insert_byte, &insertion);

        let new_pos = end_char + insertion.chars().count();
        let new_range = CCursorRange::one(CCursor::new(new_pos));
        self.cursor = Some(new_range);
        self.cursor_override = self.cursor;
    }

    fn table_size_picker(&mut self, ui: &mut egui::Ui) {
        const MAX_ROWS: u8 = 100;
        const MAX_COLS: u8 = 20;

        egui::Grid::new("table_size_grid")
            .num_columns(2)
            .spacing(egui::vec2(8.0, 4.0))
            .show(ui, |ui| {
                ui.label("Rows");
                ui.add(
                    egui::DragValue::new(&mut self.table_rows)
                        .range(1..=MAX_ROWS)
                        .speed(0.2),
                );
                ui.end_row();

                ui.label("Columns");
                ui.add(
                    egui::DragValue::new(&mut self.table_cols)
                        .range(1..=MAX_COLS)
                        .speed(0.2),
                );
                ui.end_row();
            });

        if ui
            .button(format!("{} Insert table", egui_phosphor::regular::PLUS))
            .clicked()
        {
            let rows = self.table_rows.max(1).min(MAX_ROWS);
            let cols = self.table_cols.max(1).min(MAX_COLS);
            self.insert_table_at_cursor(rows, cols);
            self.table_rows = rows;
            self.table_cols = cols;
            ui.close();
        }
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
