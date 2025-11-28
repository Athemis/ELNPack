//! Root Model-View-Update kernel wiring component state, messages, and commands.

use std::path::PathBuf;

use crate::logic::eln::{ArchiveGenre, build_and_write_archive};
use crate::models::attachment::Attachment;
use crate::models::keywords::Keywords;
use crate::ui::components::attachments::{
    self, AttachmentsCommand, AttachmentsModel, AttachmentsMsg,
};
use crate::ui::components::datetime_picker::{self, DateTimeModel, DateTimeMsg};
use crate::ui::components::keywords::{self, KeywordsModel, KeywordsMsg};
use crate::ui::components::markdown::{MarkdownModel, MarkdownMsg};

/// Top-level application state.
#[derive(Default)]
pub struct AppModel {
    pub entry_title: String,
    pub archive_genre: ArchiveGenre,
    pub markdown: MarkdownModel,
    pub attachments: AttachmentsModel,
    pub keywords: KeywordsModel,
    pub datetime: DateTimeModel,
    pub status: Option<String>,
    pub error: Option<String>,
    pub pending_commands: usize,
}

/// Application messages routed through the update function.
pub enum Msg {
    EntryTitleChanged(String),
    SetGenre(ArchiveGenre),
    SaveRequested(PathBuf),
    SaveCancelled,
    SaveCompleted(Result<PathBuf, String>),
    ThumbnailDecoded {
        path: PathBuf,
        image: eframe::egui::ColorImage,
    },
    ThumbnailReady {
        path: PathBuf,
        texture: eframe::egui::TextureHandle,
    },
    DismissError,
    Markdown(MarkdownMsg),
    Attachments(AttachmentsMsg),
    Keywords(KeywordsMsg),
    DateTime(DateTimeMsg),
}

/// Commands represent side-effects executed between frames.
pub enum Command {
    PickFiles,
    HashFile { path: PathBuf, retry: bool },
    LoadThumbnail { path: PathBuf, retry: bool },
    SaveArchive(SavePayload),
}

/// Captured, validated data for saving.
pub struct SavePayload {
    pub output: PathBuf,
    pub title: String,
    pub body: String,
    pub attachments: Vec<Attachment>,
    pub performed_at: time::OffsetDateTime,
    pub genre: ArchiveGenre,
    pub keywords: Vec<String>,
}

/// Update the application model and enqueue commands.
pub fn update(model: &mut AppModel, msg: Msg, cmds: &mut Vec<Command>) {
    match msg {
        Msg::EntryTitleChanged(text) => model.entry_title = text,
        Msg::SetGenre(genre) => model.archive_genre = genre,
        Msg::DismissError => model.error = None,
        Msg::Markdown(m) => {
            crate::ui::components::markdown::update(&mut model.markdown, m);
        }
        Msg::Attachments(m) => {
            let mut att_cmds = Vec::new();
            if let Some(event) = attachments::update(&mut model.attachments, m, &mut att_cmds) {
                surface_event(model, event.message, event.is_error);
            }
            for c in att_cmds {
                match c {
                    AttachmentsCommand::PickFiles => cmds.push(Command::PickFiles),
                    AttachmentsCommand::HashFile { path } => {
                        cmds.push(Command::HashFile { path, retry: false })
                    }
                    AttachmentsCommand::LoadThumbnail { path } => {
                        cmds.push(Command::LoadThumbnail { path, retry: false })
                    }
                }
            }
        }
        Msg::ThumbnailDecoded { path, image } => {
            // Actual texture creation must happen in ui.rs where ctx is available.
            // Here we just store the decoded image in a temporary placeholder via AttachmentsMsg.
            // This Msg variant should be transformed before reaching update; keeping a no-op to avoid panic.
            let _ = (path, image);
        }
        Msg::ThumbnailReady { path, texture } => {
            let mut att_cmds = Vec::new();
            if let Some(event) = attachments::update(
                &mut model.attachments,
                AttachmentsMsg::ThumbnailReady { path, texture },
                &mut att_cmds,
            ) {
                surface_event(model, event.message, event.is_error);
            }
            for c in att_cmds {
                match c {
                    AttachmentsCommand::PickFiles => cmds.push(Command::PickFiles),
                    AttachmentsCommand::HashFile { path } => {
                        cmds.push(Command::HashFile { path, retry: false })
                    }
                    AttachmentsCommand::LoadThumbnail { path } => {
                        cmds.push(Command::LoadThumbnail { path, retry: false })
                    }
                }
            }
        }
        Msg::Keywords(m) => {
            if let Some(event) = keywords::update(&mut model.keywords, m) {
                surface_event(model, event.message, event.is_error);
            }
        }
        Msg::DateTime(m) => datetime_picker::update(&mut model.datetime, m),
        Msg::SaveRequested(output_path) => match validate_for_save(model, output_path) {
            Ok(payload) => cmds.push(Command::SaveArchive(payload)),
            Err(err) => surface_event(model, err, true),
        },
        Msg::SaveCancelled => surface_event(model, "Save cancelled.".to_string(), false),
        Msg::SaveCompleted(result) => match result {
            Ok(path) => surface_event(model, format!("Archive saved: {}", path.display()), false),
            Err(err) => surface_event(model, format!("Failed to save archive:\n\n{err}"), true),
        },
    }
}

/// Execute a command synchronously (single-threaded for now) and return a resulting message.
pub fn run_command(cmd: Command) -> Msg {
    match cmd {
        Command::PickFiles => {
            let files = rfd::FileDialog::new()
                .set_title("Select attachments")
                .pick_files()
                .unwrap_or_default();
            Msg::Attachments(AttachmentsMsg::FilesPicked(files))
        }
        Command::HashFile { path, retry } => {
            let sha256 = match crate::utils::hash_file(&path) {
                Ok(h) => h,
                Err(_) => {
                    if retry {
                        "unavailable".to_string()
                    } else {
                        "unavailable".to_string()
                    }
                }
            };
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            let mime = attachments::guess_mime(&path);
            Msg::Attachments(AttachmentsMsg::HashComputed {
                path,
                sha256,
                size,
                mime,
            })
        }
        Command::LoadThumbnail { path, retry } => match attachments::load_image_thumbnail(&path) {
            Ok(image) => Msg::ThumbnailDecoded { path, image },
            Err(_) => {
                if retry {
                    Msg::Attachments(AttachmentsMsg::ThumbnailFailed { path })
                } else {
                    Msg::Attachments(AttachmentsMsg::ThumbnailFailed { path })
                }
            }
        },
        Command::SaveArchive(payload) => {
            let res = build_and_write_archive(
                &payload.output,
                &payload.title,
                &payload.body,
                &payload.attachments,
                payload.performed_at,
                payload.genre,
                &payload.keywords,
            )
            .map(|_| payload.output.clone());
            Msg::SaveCompleted(res.map_err(|e| e.to_string()))
        }
    }
}

fn surface_event(model: &mut AppModel, message: String, is_error: bool) {
    if is_error {
        model.error = Some(message.clone());
    }
    model.status = Some(message);
}

fn validate_for_save(model: &AppModel, output_path: PathBuf) -> Result<SavePayload, String> {
    let title = model.entry_title.trim().to_string();
    if title.is_empty() {
        return Err("Please enter a title.".into());
    }

    let body = model.markdown.text.trim().to_string();

    let keywords = Keywords::new(model.keywords.keywords().to_vec());
    let performed_at = datetime_picker::to_offset_datetime(&model.datetime)
        .map_err(|err| format!("Invalid date/time: {err}"))?;

    let attachment_meta: Vec<Attachment> = model
        .attachments
        .attachments()
        .iter()
        .map(|a| a.to_domain())
        .collect();

    crate::models::attachment::assert_unique_sanitized_names(&attachment_meta)
        .map_err(|e| e.to_string())?;

    Ok(SavePayload {
        output: output_path,
        title,
        body,
        attachments: attachment_meta,
        performed_at,
        genre: model.archive_genre,
        keywords: keywords.into_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn save_request_enqueues_and_completes() {
        let tmp = TempDir::new().unwrap();
        let output = tmp.path().join("test.eln");

        let mut model = AppModel::default();
        model.entry_title = "Title".into();
        model.markdown.text = "Body".into();

        let mut cmds = Vec::new();
        update(&mut model, Msg::SaveRequested(output.clone()), &mut cmds);

        assert_eq!(cmds.len(), 1, "save should enqueue command");

        let msg = run_command(cmds.pop().unwrap());
        let mut cmds2 = Vec::new();
        update(&mut model, msg, &mut cmds2);

        assert!(model.error.is_none());
        assert!(
            model
                .status
                .as_deref()
                .map(|s| s.contains("Archive saved"))
                .unwrap_or(false)
        );
        assert!(output.exists());
    }

    #[test]
    fn save_request_with_empty_title_sets_error() {
        let mut model = AppModel::default();
        model.entry_title = "   ".into();

        let mut cmds = Vec::new();
        update(
            &mut model,
            Msg::SaveRequested(PathBuf::from("/tmp/ignored.eln")),
            &mut cmds,
        );

        assert!(cmds.is_empty());
        assert!(model.error.is_some());
    }

    #[test]
    fn save_cancelled_sets_status() {
        let mut model = AppModel::default();
        let mut cmds = Vec::new();

        update(&mut model, Msg::SaveCancelled, &mut cmds);

        assert!(cmds.is_empty());
        assert_eq!(model.status.as_deref(), Some("Save cancelled."));
        assert!(model.error.is_none());
    }

    #[test]
    fn attachments_load_thumbnail_enqueues_command() {
        let mut model = AppModel::default();
        let mut cmds = Vec::new();
        let path = PathBuf::from("image.png");

        update(
            &mut model,
            Msg::Attachments(AttachmentsMsg::LoadThumbnail(path.clone())),
            &mut cmds,
        );

        assert_eq!(cmds.len(), 1);
        match cmds.pop().unwrap() {
            Command::LoadThumbnail { path: p, retry } => {
                assert_eq!(p, path);
                assert!(!retry);
            }
            _ => panic!("unexpected command"),
        }
    }

    #[test]
    fn pending_commands_can_track_attachment_flow() {
        let mut model = AppModel::default();
        let mut cmds = Vec::new();
        let path = PathBuf::from("image.png");

        update(
            &mut model,
            Msg::Attachments(AttachmentsMsg::LoadThumbnail(path.clone())),
            &mut cmds,
        );

        assert_eq!(cmds.len(), 1, "load thumbnail should enqueue command");

        // UI increments when dispatching commands to the worker.
        model.pending_commands += cmds.len();
        assert_eq!(model.pending_commands, 1);

        // Simulate worker response and UI decrement.
        let mut cmds2 = Vec::new();
        update(
            &mut model,
            Msg::Attachments(AttachmentsMsg::ThumbnailFailed { path }),
            &mut cmds2,
        );
        model.pending_commands = model.pending_commands.saturating_sub(1);

        assert_eq!(model.pending_commands, 0);
    }
}
