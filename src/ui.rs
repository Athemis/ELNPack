use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDate, Utc};
use eframe::egui;
use egui::RichText;
use egui::SizeHint;
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui_extras::DatePickerButton;
use egui_extras::image::load_svg_bytes_with_size;
use usvg::Options;
use time::{Date, Month, OffsetDateTime, Time};

use crate::archive::{build_and_write_archive, ensure_extension, suggested_archive_name};

pub struct AttachmentItem {
    pub name: String,
    pub path: PathBuf,
}

fn is_image(path: &Path) -> bool {
    matches!(path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "tiff" | "tif" | "gif" | "webp" | "svg")
    )
}

fn load_image_thumbnail(path: &Path) -> Result<egui::ColorImage, String> {
    const MAX: u32 = 256;
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let hint = SizeHint::Size {
            width: MAX,
            height: MAX,
            maintain_aspect_ratio: true,
        };
        let options = Options::default();
        return load_svg_bytes_with_size(&bytes, hint, &options).map_err(|e| e.to_string());
    }

    let dyn_img = image::open(path).map_err(|e| e.to_string())?;
    let resized = dyn_img.thumbnail(MAX, MAX).to_rgba8();
    let size = [resized.width() as usize, resized.height() as usize];
    let pixels = resized.into_raw();
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, &pixels))
}

fn format_two(n: i32) -> String {
    format!("{:02}", n.clamp(0, 99))
}

pub struct ElnPackApp {
    entry_title: String,
    body_text: String,
    attachments: Vec<AttachmentItem>,
    status_text: String,
    performed_date: NaiveDate,
    performed_hour: i32,
    performed_minute: i32,
    thumbnail_cache: HashMap<PathBuf, egui::TextureHandle>,
    thumbnail_failures: HashSet<PathBuf>,
    heading_level: u8,
    body_cursor: Option<CCursorRange>,
    body_cursor_override: Option<CCursorRange>,
    code_choice: CodeChoice,
    list_choice: ListChoice,
}

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
            body_text: String::new(),
            attachments: Vec::new(),
            status_text: String::new(),
            performed_date: today,
            performed_hour,
            performed_minute,
            thumbnail_cache: HashMap::new(),
            thumbnail_failures: HashSet::new(),
            heading_level: 1,
            body_cursor: None,
            body_cursor_override: None,
            code_choice: CodeChoice::Inline,
            list_choice: ListChoice::Unordered,
        }
    }
}

impl eframe::App for ElnPackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_title_input(ui);
                ui.add_space(12.0);

                self.render_description_input(ui);
                ui.add_space(12.0);

                self.render_performed_at_input(ui);
                ui.add_space(12.0);

                self.render_attachments_section(ui);
                ui.add_space(12.0);

                self.render_action_buttons(ui);
                ui.add_space(8.0);

                self.render_status(ui);
            });
        });
    }
}

impl ElnPackApp {
    fn render_theme_controls(&mut self, ui: &mut egui::Ui) {
        ui.add_space(2.0);
        egui::widgets::global_dark_light_mode_switch(ui);
    }

    fn build_performed_at(&self) -> Result<OffsetDateTime, String> {
        let month = Month::try_from(self.performed_date.month() as u8)
            .map_err(|_| "Month must be 1-12".to_string())?;

        let date = Date::from_calendar_date(
            self.performed_date.year(),
            month,
            self.performed_date.day() as u8,
        )
        .map_err(|_| "Invalid calendar date".to_string())?;

        if !(0..=23).contains(&self.performed_hour) {
            return Err("Hour must be 0-23".into());
        }
        if !(0..=59).contains(&self.performed_minute) {
            return Err("Minute must be 0-59".into());
        }

        let time = Time::from_hms(self.performed_hour as u8, self.performed_minute as u8, 0)
            .map_err(|_| "Invalid time".to_string())?;

        Ok(date.with_time(time).assume_utc())
    }

    fn render_title_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Title");
        ui.add_space(4.0);
        ui.text_edit_singleline(&mut self.entry_title);
    }

    fn render_description_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Main Text (Markdown)");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Heading");
            egui::ComboBox::from_id_salt("heading_picker")
                .selected_text(format!("H{}", self.heading_level))
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
            ui.separator();
            if ui.button("B").clicked() {
                self.insert_snippet_at_cursor("**bold**");
            }
            if ui.button("I").clicked() {
                self.insert_snippet_at_cursor("_italic_");
            }
            ui.label(format!("{} Code", egui_phosphor::regular::CODE_SIMPLE));
            egui::ComboBox::from_id_salt("code_picker")
                .selected_text(match self.code_choice {
                    CodeChoice::Inline => RichText::new(format!(
                        "{} Inline",
                        egui_phosphor::regular::CODE_SIMPLE
                    )),
                    CodeChoice::Block => RichText::new(format!(
                        "{} Block",
                        egui_phosphor::regular::CODE_BLOCK
                    )),
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            &mut self.code_choice,
                            CodeChoice::Inline,
                            RichText::new(format!(
                                "{} Inline",
                                egui_phosphor::regular::CODE_SIMPLE
                            )),
                        )
                        .clicked()
                    {
                        self.insert_snippet_at_cursor("`code`");
                    }
                    if ui
                        .selectable_value(
                            &mut self.code_choice,
                            CodeChoice::Block,
                            RichText::new(format!(
                                "{} Block",
                                egui_phosphor::regular::CODE_BLOCK
                            )),
                        )
                        .clicked()
                    {
                        self.insert_block_snippet_at_cursor("```\ncode\n```");
                    }
                });
            if ui.button("Link").clicked() {
                self.insert_snippet_at_cursor("[text](https://example.com)");
            }
            ui.label(format!("{} List", egui_phosphor::regular::LIST_DASHES));
            egui::ComboBox::from_id_salt("list_picker")
                .selected_text(match self.list_choice {
                    ListChoice::Unordered => RichText::new(format!(
                        "{} Bulleted",
                        egui_phosphor::regular::LIST_DASHES
                    )),
                    ListChoice::Ordered => RichText::new(format!(
                        "{} Numbered",
                        egui_phosphor::regular::LIST_NUMBERS
                    )),
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            &mut self.list_choice,
                            ListChoice::Unordered,
                            RichText::new(format!(
                                "{} Bulleted",
                                egui_phosphor::regular::LIST_DASHES
                            )),
                        )
                        .clicked()
                    {
                        self.insert_block_snippet_at_cursor("\n- item");
                    }
                    if ui
                        .selectable_value(
                            &mut self.list_choice,
                            ListChoice::Ordered,
                            RichText::new(format!(
                                "{} Numbered",
                                egui_phosphor::regular::LIST_NUMBERS
                            )),
                        )
                        .clicked()
                    {
                        self.insert_block_snippet_at_cursor("\n1. first");
                    }
                });
            if ui.button("Quote").clicked() {
                self.insert_block_snippet_at_cursor("\n> quote");
            }
            if ui.button("Image").clicked() {
                self.insert_snippet_at_cursor("![alt text](path/to/image.png)");
            }
            if ui.button("Strike").clicked() {
                self.insert_snippet_at_cursor("~~text~~");
            }
            if ui.button("Rule").clicked() {
                self.insert_block_snippet_at_cursor("\n---\n");
            }
        });
        ui.add_space(4.0);
        let body_id = ui.id().with("body_text_edit");
        if let Some(state) = TextEditState::load(ui.ctx(), body_id) {
            self.body_cursor = state.cursor.char_range();
        }
        let mut output = egui::TextEdit::multiline(&mut self.body_text)
            .id_source(body_id)
            .desired_width(f32::INFINITY)
            .desired_rows(8)
            .show(ui);
        if let Some(override_range) = self.body_cursor_override.take() {
            output.state.cursor.set_char_range(Some(override_range.clone()));
            self.body_cursor = Some(override_range);
        } else {
            self.body_cursor = output
                .state
                .cursor
                .char_range()
                .or_else(|| Some(CCursorRange::one(CCursor::new(self.body_text.chars().count()))));
        }
        output.state.store(ui.ctx(), body_id);
    }

    fn render_performed_at_input(&mut self, ui: &mut egui::Ui) {
        ui.label("Performed at (UTC)");
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("📆 Date");
            ui.add(DatePickerButton::new(&mut self.performed_date).show_icon(false));

            ui.label("🕒 Time");
            ui.add(
                egui::DragValue::new(&mut self.performed_hour)
                    .range(0..=23)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            );
            ui.add(
                egui::DragValue::new(&mut self.performed_minute)
                    .range(0..=59)
                    .speed(0.1)
                    .clamp_existing_to_range(true)
                    .custom_formatter(|v, _| format_two(v as i32)),
            );

            if ui.button("Use current time").clicked() {
                let now = Utc::now();
                let offset_now = OffsetDateTime::from_unix_timestamp(now.timestamp())
                    .expect("Unix timestamp conversion must succeed");
                self.performed_date = now.date_naive();
                self.performed_hour = offset_now.hour() as i32;
                self.performed_minute = offset_now.minute() as i32;
            }
        });

        ui.label(
            egui::RichText::new("Example: 2025-11-24 14:05 (UTC)")
                .small()
                .color(egui::Color32::from_gray(120)),
        );
    }

    fn render_attachments_section(&mut self, ui: &mut egui::Ui) {
        ui.label("Attachments");
        ui.add_space(4.0);

        egui::Frame::new()
            .fill(egui::Color32::from_gray(250))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(221)))
            .inner_margin(8.0)
            .show(ui, |ui| {
                let available_height = ui.available_height().min(200.0);

                egui::ScrollArea::vertical()
                    .max_height(available_height)
                    .show(ui, |ui| {
                        if self.attachments.is_empty() {
                            ui.label(
                                egui::RichText::new("No attachments")
                                    .color(egui::Color32::from_gray(150)),
                            );
                        } else {
                            self.render_attachment_list(ui);
                        }
                    });
            });
    }

    fn render_attachment_list(&mut self, ui: &mut egui::Ui) {
        let mut to_remove = None;

        for index in 0..self.attachments.len() {
            let (name, path) = {
                let item = &self.attachments[index];
                (item.name.clone(), item.path.clone())
            };
            ui.horizontal(|ui| {
                if let Some(texture) = self.get_thumbnail(ui.ctx(), &path) {
                    let size = texture.size_vec2();
                    let max = 96.0;
                    let scale = (max / size.x).min(max / size.y).min(1.0);
                    ui.add(egui::Image::new((texture.id(), size * scale)));
                }

                ui.vertical(|ui| {
                    ui.label(&name);
                    ui.label(
                        egui::RichText::new(path.to_string_lossy())
                            .small()
                            .color(egui::Color32::from_gray(102)),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Remove").clicked() {
                        to_remove = Some(index);
                    }
                });
            });

            if index < self.attachments.len() - 1 {
                ui.separator();
            }
        }

        if let Some(index) = to_remove {
            if let Some(removed) = self.attachments.get(index) {
                self.thumbnail_cache.remove(&removed.path);
                self.thumbnail_failures.remove(&removed.path);
            }
            self.attachments.remove(index);
            self.status_text = "Attachment removed".to_string();
        }
    }

    fn render_action_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Add files").clicked() {
                self.add_attachments();
            }

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

    fn render_status(&self, ui: &mut egui::Ui) {
        if !self.status_text.is_empty() {
            ui.label(egui::RichText::new(&self.status_text).color(egui::Color32::from_gray(68)));
        }
    }

    fn add_attachments(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .set_title("Select attachments")
            .pick_files()
        {
            let added = files.len();
            for file in files {
                let name = file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| format!("attachment-{}", self.attachments.len() + 1));

                self.attachments.push(AttachmentItem { name, path: file });
            }
            self.status_text = format!("Added {} attachment(s)", added);
        }
    }

    fn get_thumbnail(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
        if let Some(handle) = self.thumbnail_cache.get(path) {
            return Some(handle.clone());
        }

        if self.thumbnail_failures.contains(path) {
            return None;
        }

        if !is_image(path) {
            self.thumbnail_failures.insert(path.to_path_buf());
            return None;
        }

        let image = match load_image_thumbnail(path) {
            Ok(img) => img,
            Err(_) => {
                self.thumbnail_failures.insert(path.to_path_buf());
                return None;
            }
        };

        let texture = ctx.load_texture(
            format!("thumb-{}", path.display()),
            image,
            egui::TextureOptions::default(),
        );
        self.thumbnail_cache
            .insert(path.to_path_buf(), texture.clone());
        Some(texture)
    }

    fn save_archive(&mut self) {
        let title = self.entry_title.trim();
        let body = self.body_text.trim();

        let performed_at = match self.build_performed_at() {
            Ok(dt) => dt,
            Err(err) => {
                self.status_text = format!("Invalid date/time: {}", err);
                return;
            }
        };

        if title.is_empty() {
            self.status_text = "Please enter a title.".to_string();
            return;
        }

        let default_name = suggested_archive_name(title);
        let dialog = rfd::FileDialog::new()
            .set_title("Save ELN archive")
            .add_filter("ELN archive", &["eln"])
            .set_file_name(&default_name);

        let Some(selected_path) = dialog.save_file() else {
            self.status_text = "Save cancelled.".to_string();
            return;
        };

        let output_path = ensure_extension(selected_path, "eln");
        let attachment_paths: Vec<PathBuf> =
            self.attachments.iter().map(|a| a.path.clone()).collect();

        match build_and_write_archive(&output_path, title, body, &attachment_paths, performed_at) {
            Ok(_) => {
                self.status_text = format!("Archive saved: {}", output_path.display());
            }
            Err(err) => {
                self.status_text = format!("Error: {}", err);
            }
        }
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
            if !self.body_text.ends_with('\n') && !self.body_text.is_empty() {
                insertion = format!("\n{insertion}");
            }
        } else if !self.body_text.is_empty() && !self.body_text.ends_with(char::is_whitespace) {
            insertion.insert(0, ' ');
        }

        let (start_char, end_char) = if let Some(range) = &self.body_cursor {
            let sorted = if range.is_sorted() { range.clone() } else { CCursorRange::two(range.secondary, range.primary) };
            (sorted.primary.index, sorted.secondary.index)
        } else {
            let len = self.body_text.chars().count();
            (len, len)
        };

        let start = char_to_byte(&self.body_text, start_char);
        let end = char_to_byte(&self.body_text, end_char);

        self.body_text.replace_range(start..end, &insertion);

        let new_pos = start_char + insertion.chars().count();
        let new_range = CCursorRange::one(CCursor::new(new_pos));
        self.body_cursor = Some(new_range);
        self.body_cursor_override = self.body_cursor.clone();
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
