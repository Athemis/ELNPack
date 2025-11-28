// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Markdown editor rewritten for MVU-style updates.

use eframe::egui;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui_phosphor::regular;

/// Code insertion style preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeChoice {
    Inline,
    Block,
}

/// List insertion style preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListChoice {
    Unordered,
    Ordered,
}

/// Math insertion style preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathChoice {
    Inline,
    Display,
}

/// Editor state for the markdown component, including cursor selection metadata.
#[derive(Clone, Debug)]
pub struct MarkdownModel {
    /// Raw markdown content.
    pub text: String,
    /// Current heading level for insertions (1-6).
    pub heading_level: u8,
    /// Current cursor selection from egui.
    pub cursor: Option<CCursorRange>,
    /// Explicit cursor override applied after mutations.
    pub cursor_override: Option<CCursorRange>,
    /// Preferred code insertion style.
    pub code_choice: CodeChoice,
    /// Preferred list insertion style.
    pub list_choice: ListChoice,
    /// Preferred math insertion style.
    pub math_choice: MathChoice,
    /// Rows to use when inserting a table.
    pub table_rows: u8,
    /// Columns to use when inserting a table.
    pub table_cols: u8,
}

impl Default for MarkdownModel {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleKind {
    /// Apply bold formatting.
    Bold,
    /// Apply italic formatting.
    Italic,
    /// Apply strikethrough formatting.
    Strikethrough,
    /// Apply underline formatting.
    Underline,
    /// Insert a link template.
    Link,
    /// Prefix selection with a quote block.
    Quote,
    /// Insert an image template.
    Image,
    /// Insert a horizontal rule.
    Rule,
    /// Apply inline code formatting.
    CodeInline,
    /// Insert a fenced code block.
    CodeBlock,
    /// Insert an unordered list item.
    ListUnordered,
    /// Insert an ordered list item.
    ListOrdered,
    /// Insert inline math delimiters.
    MathInline,
    /// Insert display math delimiters.
    MathDisplay,
}

/// Messages emitted by the markdown view to mutate state.
#[derive(Clone, Debug)]
pub enum MarkdownMsg {
    SetText(String),
    SetCursor(Option<CCursorRange>),
    ClearCursorOverride,
    SetHeadingLevel(u8),
    InsertHeading(u8),
    SetCodeChoice(CodeChoice),
    SetListChoice(ListChoice),
    SetMathChoice(MathChoice),
    ApplyStyle(StyleKind),
    InsertTable { rows: u8, cols: u8 },
    SetTableRows(u8),
    SetTableCols(u8),
}

/// Update the markdown model in response to a message.
pub fn update(model: &mut MarkdownModel, msg: MarkdownMsg) {
    match msg {
        MarkdownMsg::SetText(text) => model.text = text,
        MarkdownMsg::SetCursor(cursor) => model.cursor = cursor,
        MarkdownMsg::ClearCursorOverride => model.cursor_override = None,
        MarkdownMsg::SetHeadingLevel(level) => model.heading_level = level.clamp(1, 6),
        MarkdownMsg::InsertHeading(level) => insert_heading(model, level),
        MarkdownMsg::SetCodeChoice(choice) => model.code_choice = choice,
        MarkdownMsg::SetListChoice(choice) => model.list_choice = choice,
        MarkdownMsg::SetMathChoice(choice) => model.math_choice = choice,
        MarkdownMsg::ApplyStyle(kind) => apply_style_kind(model, kind),
        MarkdownMsg::InsertTable { rows, cols } => insert_table_at_cursor(model, rows, cols),
        MarkdownMsg::SetTableRows(rows) => model.table_rows = rows.clamp(1, 100),
        MarkdownMsg::SetTableCols(cols) => model.table_cols = cols.clamp(1, 20),
    }
}

/// Render the toolbar and text area, emitting messages instead of mutating state directly.
pub fn view(model: &MarkdownModel, ui: &mut egui::Ui) -> Vec<MarkdownMsg> {
    let mut msgs = Vec::new();

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            // Headings
            let heading_resp = egui::ComboBox::from_id_salt("heading_picker")
                .width(40.0)
                .selected_text(heading_icon(model.heading_level))
                .show_ui(ui, |ui| {
                    for lvl in 1..=6u8 {
                        let icon = heading_icon(lvl);
                        if ui
                            .selectable_label(model.heading_level == lvl, icon)
                            .clicked()
                        {
                            msgs.push(MarkdownMsg::SetHeadingLevel(lvl));
                            msgs.push(MarkdownMsg::InsertHeading(lvl));
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
                msgs.push(MarkdownMsg::ApplyStyle(StyleKind::Bold));
            }
            if ui
                .button(egui_phosphor::regular::TEXT_ITALIC)
                .on_hover_text("Italic")
                .clicked()
            {
                msgs.push(MarkdownMsg::ApplyStyle(StyleKind::Italic));
            }
            if ui
                .button(egui_phosphor::regular::TEXT_STRIKETHROUGH)
                .on_hover_text("Strikethrough")
                .clicked()
            {
                msgs.push(MarkdownMsg::ApplyStyle(StyleKind::Strikethrough));
            }
            if ui
                .button(egui_phosphor::regular::TEXT_UNDERLINE)
                .on_hover_text("Underline")
                .clicked()
            {
                msgs.push(MarkdownMsg::ApplyStyle(StyleKind::Underline));
            }

            // Code
            let code_resp = egui::ComboBox::from_id_salt("code_picker")
                .width(40.0)
                .selected_text(match model.code_choice {
                    CodeChoice::Inline => egui_phosphor::regular::CODE_SIMPLE,
                    CodeChoice::Block => egui_phosphor::regular::CODE_BLOCK,
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(model.code_choice, CodeChoice::Inline),
                            egui_phosphor::regular::CODE_SIMPLE,
                        )
                        .on_hover_text("Inline code")
                        .clicked()
                    {
                        msgs.push(MarkdownMsg::SetCodeChoice(CodeChoice::Inline));
                        msgs.push(MarkdownMsg::ApplyStyle(StyleKind::CodeInline));
                    }
                    if ui
                        .selectable_label(
                            matches!(model.code_choice, CodeChoice::Block),
                            egui_phosphor::regular::CODE_BLOCK,
                        )
                        .on_hover_text("Code block")
                        .clicked()
                    {
                        msgs.push(MarkdownMsg::SetCodeChoice(CodeChoice::Block));
                        msgs.push(MarkdownMsg::ApplyStyle(StyleKind::CodeBlock));
                    }
                });
            code_resp.response.on_hover_text("Code");

            // Lists
            let list_resp = egui::ComboBox::from_id_salt("list_picker")
                .width(40.0)
                .selected_text(match model.list_choice {
                    ListChoice::Unordered => egui_phosphor::regular::LIST_DASHES,
                    ListChoice::Ordered => egui_phosphor::regular::LIST_NUMBERS,
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(model.list_choice, ListChoice::Unordered),
                            egui_phosphor::regular::LIST_DASHES,
                        )
                        .on_hover_text("Bulleted list")
                        .clicked()
                    {
                        msgs.push(MarkdownMsg::SetListChoice(ListChoice::Unordered));
                        msgs.push(MarkdownMsg::ApplyStyle(StyleKind::ListUnordered));
                    }
                    if ui
                        .selectable_label(
                            matches!(model.list_choice, ListChoice::Ordered),
                            egui_phosphor::regular::LIST_NUMBERS,
                        )
                        .on_hover_text("Numbered list")
                        .clicked()
                    {
                        msgs.push(MarkdownMsg::SetListChoice(ListChoice::Ordered));
                        msgs.push(MarkdownMsg::ApplyStyle(StyleKind::ListOrdered));
                    }
                });
            list_resp.response.on_hover_text("List");

            // Other inserts
            if ui
                .button(egui_phosphor::regular::LINK_SIMPLE)
                .on_hover_text("Link")
                .clicked()
            {
                msgs.push(MarkdownMsg::ApplyStyle(StyleKind::Link));
            }
            if ui
                .button(egui_phosphor::regular::QUOTES)
                .on_hover_text("Quote")
                .clicked()
            {
                msgs.push(MarkdownMsg::ApplyStyle(StyleKind::Quote));
            }
            if ui
                .button(egui_phosphor::regular::IMAGE_SQUARE)
                .on_hover_text("Image")
                .clicked()
            {
                msgs.push(MarkdownMsg::ApplyStyle(StyleKind::Image));
            }
            let table_resp = egui::ComboBox::from_id_salt("table_picker")
                .width(80.0)
                .selected_text(format!(
                    "{} {}×{}",
                    egui_phosphor::regular::TABLE,
                    model.table_rows,
                    model.table_cols
                ))
                .show_ui(ui, |ui| {
                    table_size_picker(ui, model, msgs.as_mut());
                });
            table_resp
                .response
                .on_hover_text("Insert table (choose size)");

            if ui
                .button(egui_phosphor::regular::RULER)
                .on_hover_text("Rule")
                .clicked()
            {
                msgs.push(MarkdownMsg::ApplyStyle(StyleKind::Rule));
            }

            let math_resp = egui::ComboBox::from_id_salt("math_picker")
                .width(40.0)
                .selected_text(match model.math_choice {
                    MathChoice::Inline => format!("{} $", egui_phosphor::regular::FUNCTION),
                    MathChoice::Display => format!("{} $$", egui_phosphor::regular::FUNCTION),
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(model.math_choice, MathChoice::Inline),
                            format!("{} $", egui_phosphor::regular::FUNCTION),
                        )
                        .on_hover_text("Inline math")
                        .clicked()
                    {
                        msgs.push(MarkdownMsg::SetMathChoice(MathChoice::Inline));
                        msgs.push(MarkdownMsg::ApplyStyle(StyleKind::MathInline));
                    }
                    if ui
                        .selectable_label(
                            matches!(model.math_choice, MathChoice::Display),
                            format!("{} $$", egui_phosphor::regular::FUNCTION),
                        )
                        .on_hover_text("Display math")
                        .clicked()
                    {
                        msgs.push(MarkdownMsg::SetMathChoice(MathChoice::Display));
                        msgs.push(MarkdownMsg::ApplyStyle(StyleKind::MathDisplay));
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
                    msgs.push(MarkdownMsg::SetCursor(state.cursor.char_range()));
                }

                // Calculate desired rows based on available height
                let line_height = ui.text_style_height(&egui::TextStyle::Monospace);
                let desired_rows = (ui.available_height() / line_height).max(1.0) as usize;

                let mut buffer = model.text.clone();
                let mut output = egui::TextEdit::multiline(&mut buffer)
                    .code_editor()
                    .id_source(body_id)
                    .desired_width(f32::INFINITY)
                    .desired_rows(desired_rows)
                    .show(ui);

                if buffer != model.text {
                    msgs.push(MarkdownMsg::SetText(buffer));
                }

                if let Some(override_range) = model.cursor_override {
                    output.state.cursor.set_char_range(Some(override_range));
                    msgs.push(MarkdownMsg::SetCursor(Some(override_range)));
                    msgs.push(MarkdownMsg::ClearCursorOverride);
                } else {
                    msgs.push(MarkdownMsg::SetCursor(
                        output.state.cursor.char_range().or_else(|| {
                            Some(CCursorRange::one(CCursor::new(model.text.chars().count())))
                        }),
                    ));
                }

                output.state.store(ui.ctx(), body_id);
            });
    });

    msgs
}

/// Map a heading level to its phosphor icon glyph.
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

/// Insert a Markdown heading at the current selection, normalizing content.
fn insert_heading(model: &mut MarkdownModel, level: u8) {
    let level = level.clamp(1, 6);
    model.heading_level = level;
    let hashes = "#".repeat(level as usize);
    let (_, _, selected) = selection(model);
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

    apply_style(model, &format!("{} ", hashes), "\n", content, true);
}

/// Dispatch a style action into a concrete insertion around the selection.
fn apply_style_kind(model: &mut MarkdownModel, kind: StyleKind) {
    match kind {
        StyleKind::Bold => apply_style(model, "**", "**", "bold", false),
        StyleKind::Italic => apply_style(model, "_", "_", "italic", false),
        StyleKind::Strikethrough => apply_style(model, "~~", "~~", "text", false),
        StyleKind::Underline => apply_style(model, "<u>", "</u>", "text", false),
        StyleKind::Link => apply_style(model, "[", "](https://example.com)", "text", false),
        StyleKind::Quote => apply_style(model, "\n> ", "", "quote", true),
        StyleKind::Image => apply_style(model, "![", "](path/to/image.png)", "alt text", false),
        StyleKind::Rule => apply_style(model, "\n---\n", "", "", true),
        StyleKind::CodeInline => apply_style(model, "`", "`", "code", false),
        StyleKind::CodeBlock => apply_style(model, "```\n", "\n```", "code", true),
        StyleKind::ListUnordered => apply_style(model, "\n- ", "", "item", true),
        StyleKind::ListOrdered => apply_style(model, "\n1. ", "", "first", true),
        StyleKind::MathInline => apply_style(model, "$", "$", "a+b=c", true),
        StyleKind::MathDisplay => apply_style(model, "$$", "$$", "a+b=c", true),
    }
}

/// UI control for choosing table dimensions before insertion.
fn table_size_picker(ui: &mut egui::Ui, model: &MarkdownModel, msgs: &mut Vec<MarkdownMsg>) {
    const MAX_ROWS: u8 = 100;
    const MAX_COLS: u8 = 20;
    let mut rows = model.table_rows;
    let mut cols = model.table_cols;

    egui::Grid::new("table_size_grid")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.label("Rows");
            if ui
                .add(
                    egui::DragValue::new(&mut rows)
                        .range(1..=MAX_ROWS)
                        .speed(0.2),
                )
                .changed()
            {
                msgs.push(MarkdownMsg::SetTableRows(rows));
            }
            ui.end_row();

            ui.label("Columns");
            if ui
                .add(
                    egui::DragValue::new(&mut cols)
                        .range(1..=MAX_COLS)
                        .speed(0.2),
                )
                .changed()
            {
                msgs.push(MarkdownMsg::SetTableCols(cols));
            }
            ui.end_row();
        });

    if ui
        .button(format!("{} Insert table", egui_phosphor::regular::PLUS))
        .clicked()
    {
        let rows = rows.clamp(1, MAX_ROWS);
        let cols = cols.clamp(1, MAX_COLS);
        msgs.push(MarkdownMsg::InsertTable { rows, cols });
        ui.close();
    }
}

/// Build a Markdown table snippet with padded headers for readability.
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

/// Insert a generated table at the current cursor and update selection metadata.
fn insert_table_at_cursor(model: &mut MarkdownModel, rows: u8, cols: u8) {
    let (_, end_char, _) = selection(model);

    let insert_byte = char_to_byte(&model.text, end_char);
    let mut insertion = String::new();

    if !model.text.is_empty() && (insert_byte == 0 || !model.text[..insert_byte].ends_with('\n')) {
        insertion.push('\n');
    }

    insertion.push_str(&table_snippet(rows, cols));

    model.text.insert_str(insert_byte, &insertion);

    let new_pos = end_char + insertion.chars().count();
    let new_range = CCursorRange::one(CCursor::new(new_pos));
    model.cursor = Some(new_range);
    model.cursor_override = model.cursor;
}

/// Return (start, end, selected text) for the current cursor range.
fn selection(model: &MarkdownModel) -> (usize, usize, String) {
    let (start_char, end_char) = if let Some(range) = &model.cursor {
        let (a, b) = (range.primary.index, range.secondary.index);
        (a.min(b), a.max(b))
    } else {
        let len = model.text.chars().count();
        (len, len)
    };

    let selected = if start_char < end_char {
        model
            .text
            .chars()
            .skip(start_char)
            .take(end_char - start_char)
            .collect::<String>()
    } else {
        String::new()
    };

    (start_char, end_char, selected)
}

/// Apply a prefix/suffix insertion around the current selection, updating cursor placement.
fn apply_style(
    model: &mut MarkdownModel,
    prefix: &str,
    suffix: &str,
    placeholder: &str,
    ensure_newline: bool,
) {
    let (start_char, end_char, selected) = selection(model);

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
        if !model.text.ends_with('\n') && !model.text.is_empty() {
            insertion = format!("\n{insertion}");
        }
    } else if selected.is_empty()
        && !model.text.is_empty()
        && start_char == end_char
        && !model.text[..char_to_byte(&model.text, start_char)].ends_with(char::is_whitespace)
    {
        insertion.insert(0, ' ');
    }

    let start = char_to_byte(&model.text, start_char);
    let end = char_to_byte(&model.text, end_char);

    model.text.replace_range(start..end, &insertion);

    let new_pos = start_char + insertion.chars().count();
    let new_range = CCursorRange::one(CCursor::new(new_pos));
    model.cursor = Some(new_range);
    model.cursor_override = model.cursor;
}

/// Convert a character index to a byte index, clamping to the end when out of bounds.
/// Convert a character index to a byte index, clamping to the string end.
fn char_to_byte(text: &str, char_idx: usize) -> usize {
    if char_idx == text.chars().count() {
        return text.len();
    }
    text.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or_else(|| text.len())
}
