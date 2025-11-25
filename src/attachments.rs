//! Attachments panel handling selection, listing, and thumbnail previews.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use eframe::egui;
use egui_extras::image::load_svg_bytes_with_size;
use usvg::Options;

/// User-selected attachment with display name and filesystem path.
pub struct AttachmentItem {
    pub name: String,
    pub path: PathBuf,
}

/// UI component that tracks attachments and renders previews when possible.
pub struct AttachmentsPanel {
    attachments: Vec<AttachmentItem>,
    thumbnail_cache: HashMap<PathBuf, egui::TextureHandle>,
    thumbnail_failures: HashSet<PathBuf>,
}

impl Default for AttachmentsPanel {
    fn default() -> Self {
        Self {
            attachments: Vec::new(),
            thumbnail_cache: HashMap::new(),
            thumbnail_failures: HashSet::new(),
        }
    }
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
                let available_height = ui.available_height().min(200.0);

                egui::ScrollArea::vertical()
                    .max_height(available_height)
                    .show(ui, |ui| {
                        if self.attachments.is_empty() {
                            ui.label(
                                egui::RichText::new("No attachments")
                                    .color(egui::Color32::from_gray(150)),
                            );
                        } else if status.is_none() {
                            status = self.render_attachment_list(ui);
                        }
                    });
            });

        status
    }

    /// Open a file picker and add selected files as attachments.
    ///
    /// Returns a short status message when files were added.
    pub fn add_via_dialog(&mut self) -> Option<String> {
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
            return Some(format!("Added {} attachment(s)", added));
        }
        None
    }

    fn render_attachment_list(&mut self, ui: &mut egui::Ui) -> Option<String> {
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
            return Some("Attachment removed".to_string());
        }

        None
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
    matches!(path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "tiff" | "tif" | "gif" | "webp" | "svg")
    )
}

/// Load and resize an image to a thumbnail-friendly `ColorImage`.
fn load_image_thumbnail(path: &Path) -> Result<egui::ColorImage, String> {
    const MAX: u32 = 256;
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
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
    use std::time::{SystemTime, UNIX_EPOCH};

    use eframe::egui::Color32;
    use image::{ImageBuffer, Rgba};

    use super::is_image;
    use super::load_image_thumbnail;

    fn unique_path(filename: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("elnpack-test-{nanos}-{filename}"))
    }

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
        let path = unique_path("thumb.png");
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(10, 12, Rgba([0, 255, 0, 255]));
        img.save(&path).expect("png saved");

        let thumb = load_image_thumbnail(&path).expect("thumbnail created");

        assert!(thumb.size[0] <= 256 && thumb.size[1] <= 256);
        let aspect = thumb.size[0] as f32 / thumb.size[1] as f32;
        let expected_aspect = 10.0 / 12.0;
        assert!((aspect - expected_aspect).abs() < 0.05);

        let _ = fs::remove_file(&path);
    }

    // SVG input should rasterize successfully within size limits.
    #[test]
    fn load_image_thumbnail_renders_svg() {
        let path = unique_path("icon.svg");
        let svg = r#"<svg xmlns='http://www.w3.org/2000/svg' width='16' height='16'><rect width='16' height='16' fill='red'/></svg>"#;
        fs::write(&path, svg).expect("svg saved");

        let thumb = load_image_thumbnail(&path).expect("thumbnail created");

        assert!(thumb.size[0] <= 256 && thumb.size[1] <= 256);
        assert!(thumb.pixels.iter().any(|p| *p != Color32::TRANSPARENT));

        let _ = fs::remove_file(&path);
    }

    // Invalid image data should yield an error instead of panicking.
    #[test]
    fn load_image_thumbnail_errors_on_invalid_image() {
        let path = unique_path("invalid.png");
        fs::write(&path, b"not an image").expect("file written");

        let result = load_image_thumbnail(&path);

        assert!(result.is_err());

        let _ = fs::remove_file(&path);
    }
}
