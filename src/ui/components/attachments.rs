// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Attachments panel refactored for MVU-style updates.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use eframe::egui;
use egui_extras::image::load_svg_bytes_with_size;
use usvg::Options;

use crate::models::attachment::Attachment;
use crate::utils::sanitize_component;

/// User-selected attachment with original path and sanitized display name.
pub struct AttachmentItem {
    /// Original filesystem path to the attachment.
    pub path: PathBuf,
    /// Sanitized filename used for display and inside the archive.
    pub sanitized_name: String,
    /// Detected MIME type.
    pub mime: String,
    /// SHA-256 digest of the file contents or `"unavailable"` on failure.
    pub sha256: String,
    /// File size in bytes.
    pub size: u64,
}

impl AttachmentItem {
    /// Convert into the domain attachment model used by archive logic.
    pub fn to_domain(&self) -> Attachment {
        Attachment::new(
            self.path.clone(),
            self.sanitized_name.clone(),
            self.mime.clone(),
            self.sha256.clone(),
            self.size,
        )
    }
}

/// MVU state for the attachments picker and thumbnail cache.
#[derive(Default)]
pub struct AttachmentsModel {
    attachments: Vec<AttachmentItem>,
    thumbnail_cache: HashMap<PathBuf, egui::TextureHandle>,
    thumbnail_failures: HashSet<PathBuf>,
    hashes: HashSet<String>,
    editing_index: Option<usize>,
    editing_buffer: String,
}

/// Messages emitted by the attachments view.
// Debug omitted because TextureHandle is not Debug.
pub enum AttachmentsMsg {
    RequestPickFiles,
    FilesPicked(Vec<PathBuf>),
    LoadThumbnail(PathBuf),
    HashComputed {
        path: PathBuf,
        sha256: String,
        size: u64,
        mime: String,
    },
    ThumbnailReady {
        path: PathBuf,
        texture: egui::TextureHandle,
    },
    ThumbnailFailed {
        path: PathBuf,
    },
    Remove(usize),
    StartEdit(usize),
    EditInputChanged(String),
    CommitEdit,
    CancelEdit,
}

/// Side-effectful commands that can be run off the UI path.
pub enum AttachmentsCommand {
    PickFiles,
    HashFile { path: PathBuf },
    LoadThumbnail { path: PathBuf },
}

/// User-facing events for status/error surfaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentsEvent {
    /// Message text to display.
    pub message: String,
    /// Whether the message represents an error.
    pub is_error: bool,
}

impl AttachmentsModel {
    /// Current list of attachments in selection order.
    pub fn attachments(&self) -> &[AttachmentItem] {
        &self.attachments
    }

    /// Convenience helper for tests to add a path directly.
    #[cfg(test)]
    pub fn add_path(&mut self, path: PathBuf) -> bool {
        // Simulate hashing step to reuse validation path.
        let sha256 = crate::utils::hash_file(&path).unwrap_or_else(|_| "unavailable".into());
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        let mime = guess_mime(&path);
        add_attachment_with_meta(self, path, sha256, size, mime)
    }
}

/// Apply a message to the attachments model. Returns a user-facing event when relevant.
pub fn update(
    model: &mut AttachmentsModel,
    msg: AttachmentsMsg,
    cmds: &mut Vec<AttachmentsCommand>,
) -> Option<AttachmentsEvent> {
    match msg {
        AttachmentsMsg::RequestPickFiles => {
            cmds.push(AttachmentsCommand::PickFiles);
            None
        }
        AttachmentsMsg::FilesPicked(paths) => {
            for path in paths {
                cmds.push(AttachmentsCommand::HashFile { path });
            }
            Some(AttachmentsEvent {
                message: "Processing attachments...".into(),
                is_error: false,
            })
        }
        AttachmentsMsg::LoadThumbnail(path) => {
            cmds.push(AttachmentsCommand::LoadThumbnail { path });
            None
        }
        AttachmentsMsg::HashComputed {
            path,
            sha256,
            size,
            mime,
        } => {
            let added = add_attachment_with_meta(model, path, sha256, size, mime);
            Some(AttachmentsEvent {
                message: if added {
                    "Attachment added".to_string()
                } else {
                    "Attachment skipped (duplicate or invalid)".to_string()
                },
                is_error: !added,
            })
        }
        AttachmentsMsg::ThumbnailReady { path, texture } => {
            model.thumbnail_cache.insert(path, texture);
            None
        }
        AttachmentsMsg::ThumbnailFailed { path } => {
            model.thumbnail_failures.insert(path);
            None
        }
        AttachmentsMsg::Remove(index) => {
            remove_attachment(model, index);
            Some(AttachmentsEvent {
                message: "Attachment removed".to_string(),
                is_error: false,
            })
        }
        AttachmentsMsg::StartEdit(index) => {
            model.editing_index = Some(index);
            model.editing_buffer = model
                .attachments
                .get(index)
                .map(|a| a.sanitized_name.clone())
                .unwrap_or_default();
            None
        }
        AttachmentsMsg::EditInputChanged(text) => {
            model.editing_buffer = text;
            None
        }
        AttachmentsMsg::CommitEdit => commit_filename_edit(model),
        AttachmentsMsg::CancelEdit => {
            model.editing_index = None;
            model.editing_buffer.clear();
            None
        }
    }
}

/// Render the attachments panel and return any messages triggered by user interaction.
pub fn view(ui: &mut egui::Ui, model: &AttachmentsModel) -> Vec<AttachmentsMsg> {
    let mut msgs = Vec::new();

    let add_resp = ui.add(egui::Button::new(format!(
        "{} Add files",
        egui_phosphor::regular::PLUS
    )));
    let add_resp = add_resp.on_hover_text("Add files");
    if add_resp.clicked() {
        msgs.push(AttachmentsMsg::RequestPickFiles);
    }

    ui.add_space(6.0);

    let visuals = ui.visuals().clone();
    egui::Frame::new()
        .fill(visuals.panel_fill)
        .stroke(visuals.window_stroke())
        .inner_margin(8.0)
        .show(ui, |ui| {
            if model.attachments.is_empty() {
                ui.label(
                    egui::RichText::new("No attachments").color(egui::Color32::from_gray(150)),
                );
            } else {
                render_attachment_list(ui, model, &mut msgs);
            }
        });

    msgs
}

/// Render the list of attachments with thumbnails and controls.
fn render_attachment_list(
    ui: &mut egui::Ui,
    model: &AttachmentsModel,
    msgs: &mut Vec<AttachmentsMsg>,
) {
    for index in 0..model.attachments.len() {
        let (sanitized_name, original_name, path, mime, sha, size) = {
            let item = &model.attachments[index];
            let original_name = item
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("attachment-{}", index + 1));
            (
                item.sanitized_name.clone(),
                original_name,
                item.path.clone(),
                item.mime.clone(),
                item.sha256.clone(),
                item.size,
            )
        };

        ui.horizontal(|ui| {
            let _thumb_slot = if let Some(texture) = model.thumbnail_cache.get(&path) {
                let size = texture.size_vec2();
                let max = 96.0;
                let scale = (max / size.x).min(max / size.y).min(1.0);
                ui.add(egui::Image::new((texture.id(), size * scale))).rect
            } else {
                if !model.thumbnail_failures.contains(&path) && is_image(&path) {
                    msgs.push(AttachmentsMsg::LoadThumbnail(path.clone()));
                }
                ui.allocate_space(egui::vec2(96.0, 72.0)).1
            };

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if model.editing_index == Some(index) {
                        render_editing_filename(ui, model, msgs);
                    } else {
                        if sanitized_name != original_name {
                            ui.label(
                                egui::RichText::new(egui_phosphor::regular::WARNING)
                                    .color(egui::Color32::from_rgb(232, 89, 12)),
                            )
                            .on_hover_cursor(egui::CursorIcon::Help)
                            .on_hover_text(format!(
                                "Filename sanitized:\n{} {} {}",
                                original_name,
                                egui_phosphor::regular::ARROW_RIGHT,
                                sanitized_name
                            ));
                        }

                        ui.label(sanitized_name.clone());

                        if ui
                            .button(
                                egui::RichText::new(egui_phosphor::regular::PENCIL_SIMPLE)
                                    .color(egui::Color32::from_gray(140)),
                            )
                            .on_hover_text("Edit filename")
                            .clicked()
                        {
                            msgs.push(AttachmentsMsg::StartEdit(index));
                        }
                    }
                });
                ui.label(
                    egui::RichText::new(path.to_string_lossy())
                        .small()
                        .color(egui::Color32::from_gray(102)),
                );
                ui.label(
                    egui::RichText::new(format!("{} | sha256 {}", mime, sha))
                        .small()
                        .color(egui::Color32::from_gray(90)),
                );
                ui.label(
                    egui::RichText::new(format_bytes(size))
                        .small()
                        .color(egui::Color32::from_gray(90)),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new(egui_phosphor::regular::TRASH_SIMPLE))
                    .on_hover_text("Remove attached file")
                    .clicked()
                {
                    msgs.push(AttachmentsMsg::Remove(index));
                }
            });
        });

        if index < model.attachments.len() - 1 {
            ui.separator();
        }
    }
}

/// Inline filename edit UI with save/cancel controls.
fn render_editing_filename(
    ui: &mut egui::Ui,
    model: &AttachmentsModel,
    msgs: &mut Vec<AttachmentsMsg>,
) {
    let mut buffer = model.editing_buffer.clone();
    let response = ui.add(
        egui::TextEdit::singleline(&mut buffer)
            .hint_text("Edit filename")
            .desired_width(180.0),
    );

    if response.changed() {
        msgs.push(AttachmentsMsg::EditInputChanged(buffer.clone()));
    }

    let commit_via_keyboard = response.lost_focus()
        && ui.input(|inp| inp.key_pressed(egui::Key::Enter) || inp.key_pressed(egui::Key::Tab));

    if commit_via_keyboard {
        msgs.push(AttachmentsMsg::CommitEdit);
        return;
    }

    if ui
        .button(egui_phosphor::regular::CHECK)
        .on_hover_text("Save")
        .clicked()
    {
        msgs.push(AttachmentsMsg::CommitEdit);
    }

    if ui
        .button(egui_phosphor::regular::X)
        .on_hover_text("Cancel")
        .clicked()
    {
        msgs.push(AttachmentsMsg::CancelEdit);
    }
}

/// Insert a new attachment if it does not collide by sanitized name or hash.
fn add_attachment_with_meta(
    model: &mut AttachmentsModel,
    path: PathBuf,
    sha256: String,
    size: u64,
    mime: String,
) -> bool {
    let original_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("attachment-{}", model.attachments.len() + 1));
    let sanitized_name = sanitize_component(&original_name);

    if model
        .attachments
        .iter()
        .any(|item| item.sanitized_name == sanitized_name)
    {
        return false;
    }

    if sha256 != "unavailable" && model.hashes.contains(&sha256) {
        return false;
    }

    if sha256 != "unavailable" {
        model.hashes.insert(sha256.clone());
    }
    model.attachments.push(AttachmentItem {
        path,
        sanitized_name,
        mime,
        sha256,
        size,
    });
    true
}

/// Remove an attachment and associated cache entries safely.
fn remove_attachment(model: &mut AttachmentsModel, index: usize) {
    if let Some(removed) = model.attachments.get(index) {
        model.thumbnail_cache.remove(&removed.path);
        model.thumbnail_failures.remove(&removed.path);
        if removed.sha256 != "unavailable" {
            model.hashes.remove(&removed.sha256);
        }
    }
    if index < model.attachments.len() {
        model.attachments.remove(index);
    }
}

/// Validate and commit a sanitized filename edit, returning a feedback event.
fn commit_filename_edit(model: &mut AttachmentsModel) -> Option<AttachmentsEvent> {
    let index = model.editing_index?;

    let raw = model.editing_buffer.trim();
    if raw.is_empty() {
        return Some(AttachmentsEvent {
            message: "Filename cannot be empty.".into(),
            is_error: true,
        });
    }

    let sanitized = sanitize_component(raw);
    if sanitized.is_empty() {
        return Some(AttachmentsEvent {
            message: "Filename is invalid after sanitization.".into(),
            is_error: true,
        });
    }

    let duplicate = model
        .attachments
        .iter()
        .enumerate()
        .any(|(i, item)| i != index && item.sanitized_name == sanitized);
    if duplicate {
        return Some(AttachmentsEvent {
            message: "Another attachment already uses this filename in the archive.".into(),
            is_error: true,
        });
    }

    if let Some(item) = model.attachments.get_mut(index) {
        item.sanitized_name = sanitized;
    }

    model.editing_index = None;
    model.editing_buffer.clear();

    Some(AttachmentsEvent {
        message: "Attachment filename updated.".into(),
        is_error: false,
    })
}

/// Return true when the path extension is a supported raster or SVG image.
/// Return true when the path extension is a supported raster or SVG image.
fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "tiff" | "tif" | "gif" | "webp" | "svg"
            )
        })
}

/// Return true when the path extension is SVG.
/// Return true when the path extension is SVG.
fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

pub(crate) fn guess_mime(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_string()
}

/// Human-readable formatting for byte sizes with binary units.
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Load and resize an image to a thumbnail-friendly `ColorImage`.
pub(crate) fn load_image_thumbnail(path: &Path) -> Result<egui::ColorImage, String> {
    const MAX: u32 = 256;

    if is_svg(path) {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let hint = egui::SizeHint::Size {
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use eframe::egui::Color32;
    use image::{ImageBuffer, Rgba};
    use tempfile::TempDir;

    use super::{AttachmentsModel, is_image, load_image_thumbnail};

    // Ensures extension filtering matches documented formats and rejects others.
    #[test]
    fn is_image_recognizes_supported_extensions() {
        assert!(is_image(Path::new("photo.PNG")));
        assert!(is_image(Path::new("diagram.svg")));
        assert!(!is_image(Path::new("notes.txt")));
    }

    // Raster thumbnails should retain aspect ratio and respect max bounds.
    #[test]
    fn load_image_thumbnail_handles_raster_image() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("thumb.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(10, 12, Rgba([0, 255, 0, 255]));
        img.save(&path).expect("png saved");

        let thumb = load_image_thumbnail(&path).expect("thumbnail created");

        assert!(thumb.size[0] <= 256 && thumb.size[1] <= 256);
        let aspect = thumb.size[0] as f32 / thumb.size[1] as f32;
        let expected_aspect = 10.0 / 12.0;
        assert!((aspect - expected_aspect).abs() < 0.05);
    }

    // SVG input should rasterize successfully within size limits.
    #[test]
    fn load_image_thumbnail_renders_svg() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("icon.svg");
        let svg = r"<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><rect width='16' height='16' fill='red'/></svg>";
        fs::write(&path, svg).expect("svg saved");

        let thumb = load_image_thumbnail(&path).expect("thumbnail created");

        assert!(thumb.size[0] <= 256 && thumb.size[1] <= 256);
        assert!(thumb.pixels.iter().any(|p| *p != Color32::TRANSPARENT));
    }

    // Invalid image data should yield an error instead of panicking.
    #[test]
    fn load_image_thumbnail_errors_on_invalid_image() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("invalid.png");
        fs::write(&path, b"not an image").expect("file written");

        let result = load_image_thumbnail(&path);

        assert!(result.is_err());
    }

    #[test]
    fn add_via_dialog_skips_duplicates_by_hash() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("dup1.bin");
        let path2 = tmp.path().join("dup2.bin");
        fs::write(&path1, b"same-bytes").unwrap();
        fs::write(&path2, b"same-bytes").unwrap();

        let mut model = AttachmentsModel::default();
        // Simulate adding files directly to avoid dialog.
        model.add_path(path1.clone());
        model.add_path(path2.clone());

        assert_eq!(
            model.attachments.len(),
            1,
            "duplicate file should be skipped"
        );
    }

    // Sanitized filename collisions should be rejected to avoid archive path clashes.
    #[test]
    fn add_attachment_rejects_sanitized_name_collision() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("report.txt");
        let path2 = tmp.path().join("report..txt"); // sanitizes to same name
        fs::write(&path1, b"a").unwrap();
        fs::write(&path2, b"b").unwrap();

        let mut model = AttachmentsModel::default();
        assert!(model.add_path(path1));
        assert!(
            !model.add_path(path2),
            "second attachment with same sanitized name should be skipped"
        );
        assert_eq!(model.attachments.len(), 1);
    }

    // Verifies that sanitized_name is computed correctly for various filename patterns.
    #[test]
    fn add_attachment_sanitizes_filenames() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("Café (draft).md");
        let path2 = tmp.path().join("file...with..dots.tar.gz");
        let path3 = tmp.path().join("normal-file_123.txt");
        fs::write(&path1, b"test1").unwrap();
        fs::write(&path2, b"test2").unwrap();
        fs::write(&path3, b"test3").unwrap();

        let mut model = AttachmentsModel::default();
        model.add_path(path1);
        model.add_path(path2);
        model.add_path(path3);

        assert_eq!(model.attachments.len(), 3);
        assert_eq!(model.attachments[0].sanitized_name, "Cafe_draft.md");
        assert_eq!(model.attachments[1].sanitized_name, "file.with.dots.tar.gz");
        assert_eq!(model.attachments[2].sanitized_name, "normal-file_123.txt");
    }
}
