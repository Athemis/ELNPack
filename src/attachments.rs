//! Attachments panel handling selection, listing, and thumbnail previews.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use eframe::egui;
use egui_extras::image::load_svg_bytes_with_size;
use usvg::Options;

use crate::utils::{hash_file, sanitize_component};

/// User-selected attachment with original path and sanitized display name.
pub struct AttachmentItem {
    /// Original filesystem path to the attachment.
    pub path: PathBuf,
    /// Sanitized filename used for display and inside the archive.
    pub sanitized_name: String,
    pub mime: String,
    pub sha256: String,
    pub size: u64,
}

/// UI component that tracks attachments and renders previews when possible.
#[derive(Default)]
pub struct AttachmentsPanel {
    attachments: Vec<AttachmentItem>,
    thumbnail_cache: HashMap<PathBuf, egui::TextureHandle>,
    thumbnail_failures: HashSet<PathBuf>,
    hashes: HashSet<String>,
    editing_index: Option<usize>,
    editing_buffer: String,
}

impl AttachmentsPanel {
    /// Current list of attachments in selection order.
    pub fn attachments(&self) -> &[AttachmentItem] {
        &self.attachments
    }

    /// Render the attachments panel and return a status string to surface in the UI.
    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let mut status: Option<String> = None;

        let visuals = ui.visuals().clone();
        egui::Frame::new()
            .fill(visuals.panel_fill)
            .stroke(visuals.window_stroke())
            .inner_margin(8.0)
            .show(ui, |ui| {
                if self.attachments.is_empty() {
                    ui.label(
                        egui::RichText::new("No attachments").color(egui::Color32::from_gray(150)),
                    );
                } else if status.is_none() {
                    status = self.render_attachment_list(ui);
                }
            });

        status
    }

    /// Open a file picker and add selected files as attachments.
    ///
    /// Returns a short status message when files were added.
    pub fn add_via_dialog(&mut self) -> Option<String> {
        let files = rfd::FileDialog::new()
            .set_title("Select attachments")
            .pick_files()?;

        let mut added = 0usize;
        let mut skipped = 0usize;

        for file in files {
            if self.add_attachment(file) {
                added += 1;
            } else {
                skipped += 1;
            }
        }

        Some(match (added, skipped) {
            (a, 0) => format!("Added {} attachment(s)", a),
            (a, s) => format!("Added {} attachment(s); skipped {} duplicate(s)", a, s),
        })
    }

    /// Add a single attachment file, checking for duplicates by hash.
    ///
    /// Returns true if the file was added, false if it was a duplicate.
    fn add_attachment(&mut self, path: PathBuf) -> bool {
        let original_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("attachment-{}", self.attachments.len() + 1));
        let sanitized_name = sanitize_component(&original_name);

        let mime = guess_mime(&path);
        let sha256 = match hash_file(&path) {
            Ok(hash) => hash,
            Err(_) => "unavailable".to_string(),
        };
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);

        if sha256 != "unavailable" && self.hashes.contains(&sha256) {
            return false;
        }

        if sha256 != "unavailable" {
            self.hashes.insert(sha256.clone());
        }
        self.attachments.push(AttachmentItem {
            path,
            sanitized_name,
            mime,
            sha256,
            size,
        });
        true
    }

    fn render_attachment_list(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let mut to_remove = None;
        let mut status = None;

        for index in 0..self.attachments.len() {
            let (sanitized_name, original_name, path, mime, sha, size) = {
                let item = &self.attachments[index];
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
                let _thumb_slot = if let Some(texture) = self.get_thumbnail(ui.ctx(), &path) {
                    let size = texture.size_vec2();
                    let max = 96.0;
                    let scale = (max / size.x).min(max / size.y).min(1.0);
                    ui.add(egui::Image::new((texture.id(), size * scale))).rect
                } else {
                    // Reserve space so text aligns across rows even without thumbnails.
                    ui.allocate_space(egui::vec2(96.0, 72.0)).1
                };

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        if self.editing_index == Some(index) {
                            if let Some(msg) = self.render_editing_filename(ui, index) {
                                status = Some(msg);
                            }
                        } else {
                            // Optional warning icon if sanitized != original.
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

                            // Filename label (always shown the same way).
                            ui.label(sanitized_name.clone());

                            // Single edit button (always shown).
                            if ui
                                .button(
                                    egui::RichText::new(egui_phosphor::regular::PENCIL_SIMPLE)
                                        .color(egui::Color32::from_gray(140)),
                                )
                                .on_hover_text("Edit filename")
                                .clicked()
                            {
                                self.editing_index = Some(index);
                                self.editing_buffer = sanitized_name.clone();
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
                if removed.sha256 != "unavailable" {
                    self.hashes.remove(&removed.sha256);
                }
            }
            self.attachments.remove(index);
            return Some("Attachment removed".to_string());
        }

        status
    }

    /// Render inline editing controls for a filename.
    ///
    /// Returns a status message if validation fails or succeeds.
    fn render_editing_filename(&mut self, ui: &mut egui::Ui, index: usize) -> Option<String> {
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.editing_buffer)
                .hint_text("Edit filename")
                .desired_width(180.0),
        );

        let commit_via_keyboard = response.lost_focus()
            && ui.input(|inp| inp.key_pressed(egui::Key::Enter) || inp.key_pressed(egui::Key::Tab));

        if commit_via_keyboard {
            return self.commit_filename_edit(index);
        }

        if ui
            .button(egui_phosphor::regular::CHECK)
            .on_hover_text("Save")
            .clicked()
        {
            return self.commit_filename_edit(index);
        }

        if ui
            .button(egui_phosphor::regular::X)
            .on_hover_text("Cancel")
            .clicked()
        {
            self.editing_index = None;
            self.editing_buffer.clear();
        }

        None
    }

    /// Commit an inline filename edit with validation and sanitization.
    ///
    /// Returns a status message if validation fails or succeeds.
    fn commit_filename_edit(&mut self, index: usize) -> Option<String> {
        let raw = self.editing_buffer.trim();
        if raw.is_empty() {
            return Some("Filename cannot be empty.".into());
        }

        // Always run through sanitizer to keep names filesystem-safe.
        let sanitized = sanitize_component(raw);
        if sanitized.is_empty() {
            return Some("Filename is invalid after sanitization.".into());
        }

        // Avoid two attachments ending up with the same archive path.
        let duplicate = self
            .attachments
            .iter()
            .enumerate()
            .any(|(i, item)| i != index && item.sanitized_name == sanitized);
        if duplicate {
            return Some("Another attachment already uses this filename in the archive.".into());
        }

        if let Some(item) = self.attachments.get_mut(index) {
            item.sanitized_name = sanitized;
        }

        self.editing_index = None;
        self.editing_buffer.clear();

        Some("Attachment filename updated.".into())
    }

    /// Lazily load or reuse a thumbnail texture for an attachment path.
    ///
    /// Skips non-images and caches failures to avoid repeated decoding attempts.
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
}

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
fn is_svg(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

fn guess_mime(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_string()
}

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
fn load_image_thumbnail(path: &Path) -> Result<egui::ColorImage, String> {
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

    use super::{AttachmentsPanel, is_image, load_image_thumbnail};

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

        let mut panel = AttachmentsPanel::default();
        // Simulate adding files directly to avoid dialog.
        panel.add_attachment(path1.clone());
        panel.add_attachment(path2.clone());

        assert_eq!(
            panel.attachments.len(),
            1,
            "duplicate file should be skipped"
        );
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

        let mut panel = AttachmentsPanel::default();
        panel.add_attachment(path1);
        panel.add_attachment(path2);
        panel.add_attachment(path3);

        assert_eq!(panel.attachments.len(), 3);
        assert_eq!(panel.attachments[0].sanitized_name, "Cafe_draft.md");
        assert_eq!(panel.attachments[1].sanitized_name, "file.with.dots.tar.gz");
        assert_eq!(panel.attachments[2].sanitized_name, "normal-file_123.txt");
    }
}
