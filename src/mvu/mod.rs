// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Root Model-View-Update kernel wiring component state, messages, and commands.

use std::path::PathBuf;

use crate::logic::eln::{ArchiveGenre, build_and_write_archive};
use crate::models::attachment::Attachment;
use crate::models::extra_fields::{ExtraField, ExtraFieldGroup};
use crate::models::keywords::Keywords;
use crate::ui::components::attachments::{
    self, AttachmentsCommand, AttachmentsModel, AttachmentsMsg,
};
use crate::ui::components::datetime_picker::{self, DateTimeModel, DateTimeMsg};
use crate::ui::components::extra_fields::{
    self, ExtraFieldsCommand, ExtraFieldsModel, ExtraFieldsMsg,
};
use crate::ui::components::keywords::{self, KeywordsModel, KeywordsMsg};
use crate::ui::components::markdown::{MarkdownModel, MarkdownMsg};

/// Top-level application state.
#[derive(Default)]
pub struct AppModel {
    /// User-facing entry title.
    pub entry_title: String,
    /// Selected archive genre for metadata.
    pub archive_genre: ArchiveGenre,
    /// How the body should be stored (raw markdown or rendered HTML).
    pub body_format: crate::logic::eln::BodyFormat,
    /// Markdown editor state.
    pub markdown: MarkdownModel,
    /// Attachment picker state.
    pub attachments: AttachmentsModel,
    /// Keywords editor state.
    pub keywords: KeywordsModel,
    /// Imported eLabFTW extra fields metadata.
    pub extra_fields: ExtraFieldsModel,
    /// Date/time picker state.
    pub datetime: DateTimeModel,
    /// Latest status message to display.
    pub status: Option<String>,
    /// Latest error message to display in modal.
    pub error: Option<String>,
    /// Count of queued background commands.
    pub pending_commands: usize,
}

/// Application messages routed through the update function.
pub enum Msg {
    EntryTitleChanged(String),
    SetGenre(ArchiveGenre),
    SetBodyFormat(crate::logic::eln::BodyFormat),
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
    ExtraFields(ExtraFieldsMsg),
    DateTime(DateTimeMsg),
}

/// Commands represent side-effects executed between frames.
pub enum Command {
    PickFiles,
    HashFile { path: PathBuf, _retry: bool },
    LoadThumbnail { path: PathBuf, _retry: bool },
    PickExtraFieldsFile,
    SaveArchive(SavePayload),
}

/// Captured, validated data for saving.
pub struct SavePayload {
    /// Final archive path on disk (with `.eln` extension enforced).
    pub output: PathBuf,
    /// Entry title.
    pub title: String,
    /// Markdown text from the editor.
    pub body: String,
    /// Attachment metadata (already sanitized and hashed).
    pub attachments: Vec<Attachment>,
    /// Timestamp for when the entry was performed.
    pub performed_at: time::OffsetDateTime,
    /// Selected archive genre.
    pub genre: ArchiveGenre,
    /// Normalized keywords.
    pub keywords: Vec<String>,
    /// Imported extra fields metadata.
    pub extra_fields: Vec<ExtraField>,
    pub extra_groups: Vec<ExtraFieldGroup>,
    /// Stored body format (HTML or Markdown).
    pub body_format: crate::logic::eln::BodyFormat,
}

/// Update the application model and enqueue commands.
pub fn update(model: &mut AppModel, msg: Msg, cmds: &mut Vec<Command>) {
    match msg {
        Msg::EntryTitleChanged(text) => model.entry_title = text,
        Msg::SetGenre(genre) => model.archive_genre = genre,
        Msg::SetBodyFormat(format) => model.body_format = format,
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
                    AttachmentsCommand::HashFile { path } => cmds.push(Command::HashFile {
                        path,
                        _retry: false,
                    }),
                    AttachmentsCommand::LoadThumbnail { path } => {
                        cmds.push(Command::LoadThumbnail {
                            path,
                            _retry: false,
                        })
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
                    AttachmentsCommand::HashFile { path } => cmds.push(Command::HashFile {
                        path,
                        _retry: false,
                    }),
                    AttachmentsCommand::LoadThumbnail { path } => {
                        cmds.push(Command::LoadThumbnail {
                            path,
                            _retry: false,
                        })
                    }
                }
            }
        }
        Msg::Keywords(m) => {
            if let Some(event) = keywords::update(&mut model.keywords, m) {
                surface_event(model, event.message, event.is_error);
            }
        }
        Msg::ExtraFields(m) => {
            let mut extra_cmds = Vec::new();
            if let Some(event) = extra_fields::update(&mut model.extra_fields, m, &mut extra_cmds) {
                surface_event(model, event.message, event.is_error);
            }
            for c in extra_cmds {
                match c {
                    ExtraFieldsCommand::PickMetadataFile => cmds.push(Command::PickExtraFieldsFile),
                }
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
        Command::PickExtraFieldsFile => {
            let file = rfd::FileDialog::new()
                .set_title("Select eLabFTW metadata JSON")
                .add_filter("JSON", &["json"])
                .pick_file();

            match file {
                Some(path) => match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        match crate::models::extra_fields::parse_elabftw_extra_fields(&content) {
                            Ok(import) => Msg::ExtraFields(ExtraFieldsMsg::ImportLoaded {
                                fields: import.fields,
                                groups: import.groups,
                                source: path,
                            }),
                            Err(err) => {
                                Msg::ExtraFields(ExtraFieldsMsg::ImportFailed(err.to_string()))
                            }
                        }
                    }
                    Err(err) => Msg::ExtraFields(ExtraFieldsMsg::ImportFailed(format!(
                        "Failed to read metadata file: {err}"
                    ))),
                },
                None => Msg::ExtraFields(ExtraFieldsMsg::ImportCancelled),
            }
        }
        Command::HashFile { path, _retry: _ } => {
            let sha256 = crate::utils::hash_file(&path).unwrap_or_else(|_| "unavailable".into());
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            let mime = attachments::guess_mime(&path);
            Msg::Attachments(AttachmentsMsg::HashComputed {
                path,
                sha256,
                size,
                mime,
            })
        }
        Command::LoadThumbnail { path, _retry: _ } => {
            match attachments::load_image_thumbnail(&path) {
                Ok(image) => Msg::ThumbnailDecoded { path, image },
                Err(_) => Msg::Attachments(AttachmentsMsg::ThumbnailFailed { path }),
            }
        }
        Command::SaveArchive(payload) => {
            let res = build_and_write_archive(
                &payload.output,
                &payload.title,
                &payload.body,
                &payload.attachments,
                &payload.extra_fields,
                &payload.extra_groups,
                payload.performed_at,
                payload.genre,
                &payload.keywords,
                payload.body_format,
            )
            .map(|_| payload.output.clone());
            Msg::SaveCompleted(res.map_err(|e| e.to_string()))
        }
    }
}

/// Update status/error fields consistently for user feedback.
fn surface_event(model: &mut AppModel, message: String, is_error: bool) {
    if is_error {
        model.error = Some(message.clone());
    }
    model.status = Some(message);
}

/// Validate model state and build the payload required to save an archive.
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

    for field in model.extra_fields.fields() {
        if let Some(err) = crate::ui::components::extra_fields::validate_field(field) {
            let msg = match err {
                "required" => format!("Field '{}' is required.", field.label),
                "invalid_url" => format!("Field '{}' must be a valid http/https URL.", field.label),
                "invalid_number" => format!("Field '{}' must be a valid number.", field.label),
                "invalid_integer" => format!("Field '{}' must be a valid integer ID.", field.label),
                _ => format!("Field '{}' is invalid.", field.label),
            };
            return Err(msg);
        }
    }

    Ok(SavePayload {
        output: output_path,
        title,
        body,
        attachments: attachment_meta,
        performed_at,
        genre: model.archive_genre,
        keywords: keywords.into_vec(),
        extra_fields: model.extra_fields.fields().to_vec(),
        extra_groups: model.extra_fields.groups().to_vec(),
        body_format: model.body_format,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::*;
    use crate::models::extra_fields::ExtraFieldKind;
    use crate::ui::components::extra_fields::ExtraFieldsMsg;
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
            Command::LoadThumbnail { path: p, _retry } => {
                assert_eq!(p, path);
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

    #[test]
    fn validate_rejects_invalid_url_field() {
        let mut model = AppModel::default();
        model.entry_title = "Has URL".into();
        model.markdown.text = "Body".into();

        add_url_field(&mut model, "htp://example");

        match validate_for_save(&model, PathBuf::from("/tmp/out.eln")) {
            Err(err) => assert!(err.contains("valid http/https URL")),
            Ok(_) => panic!("validation should fail for invalid URL"),
        }
    }

    #[test]
    fn validate_accepts_valid_url_field() {
        let mut model = AppModel::default();
        model.entry_title = "Has URL".into();
        model.markdown.text = "Body".into();

        add_url_field(&mut model, "https://example.com/path");

        let res = validate_for_save(&model, PathBuf::from("/tmp/out.eln"));

        assert!(res.is_ok());
    }

    #[test]
    fn validate_rejects_invalid_number_field() {
        let mut model = AppModel::default();
        model.entry_title = "Has number".into();
        model.markdown.text = "Body".into();

        add_typed_field(&mut model, ExtraFieldKind::Number, "abc");

        match validate_for_save(&model, PathBuf::from("/tmp/out.eln")) {
            Err(err) => assert!(err.contains("valid number")),
            Ok(_) => panic!("validation should fail for invalid number"),
        }
    }

    #[test]
    fn validate_accepts_valid_number_field() {
        let mut model = AppModel::default();
        model.entry_title = "Has number".into();
        model.markdown.text = "Body".into();

        add_typed_field(&mut model, ExtraFieldKind::Number, "42.5");

        let res = validate_for_save(&model, PathBuf::from("/tmp/out.eln"));

        assert!(res.is_ok());
    }

    #[test]
    fn validate_rejects_invalid_integer_field() {
        let mut model = AppModel::default();
        model.entry_title = "Has int".into();
        model.markdown.text = "Body".into();

        add_typed_field(&mut model, ExtraFieldKind::Items, "12.3");

        match validate_for_save(&model, PathBuf::from("/tmp/out.eln")) {
            Err(err) => assert!(err.contains("valid integer ID")),
            Ok(_) => panic!("validation should fail for invalid integer"),
        }
    }

    #[test]
    fn validate_accepts_valid_integer_field() {
        let mut model = AppModel::default();
        model.entry_title = "Has int".into();
        model.markdown.text = "Body".into();

        add_typed_field(&mut model, ExtraFieldKind::Users, "12345");

        let res = validate_for_save(&model, PathBuf::from("/tmp/out.eln"));

        assert!(res.is_ok());
    }

    fn add_url_field(model: &mut AppModel, value: &str) {
        let mut cmds = Vec::new();

        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::StartAddField { group_id: None }),
            &mut cmds,
        );
        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::DraftLabelChanged("URL".into())),
            &mut cmds,
        );
        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::DraftKindChanged(ExtraFieldKind::Url)),
            &mut cmds,
        );
        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::CommitFieldModal),
            &mut cmds,
        );
        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::EditValue {
                index: 0,
                value: value.into(),
            }),
            &mut cmds,
        );

        assert!(
            cmds.is_empty(),
            "URL field setup should not enqueue commands"
        );
    }

    fn add_typed_field(model: &mut AppModel, kind: ExtraFieldKind, value: &str) {
        let mut cmds = Vec::new();

        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::StartAddField { group_id: None }),
            &mut cmds,
        );
        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::DraftLabelChanged("Field".into())),
            &mut cmds,
        );
        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::DraftKindChanged(kind)),
            &mut cmds,
        );
        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::CommitFieldModal),
            &mut cmds,
        );
        update(
            model,
            Msg::ExtraFields(ExtraFieldsMsg::EditValue {
                index: 0,
                value: value.into(),
            }),
            &mut cmds,
        );

        assert!(
            cmds.is_empty(),
            "typed field setup should not enqueue commands"
        );
    }
}
