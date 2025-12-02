// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! UI component for importing and editing eLabFTW extra fields metadata.

use eframe::egui;

use crate::models::extra_fields::{ExtraField, ExtraFieldGroup, ExtraFieldKind, validate_field};

/// UI state for imported extra fields.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct ExtraFieldsModel {
    fields: Vec<ExtraField>,
    groups: Vec<ExtraFieldGroup>,
    editing_group: Option<usize>,
    editing_group_buffer: String,
    editing_field: Option<usize>,
    modal_open: bool,
    modal_draft: Option<FieldDraft>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FieldDraft {
    label: String,
    description: String,
    required: bool,
    allow_multi_values: bool,
    options: Vec<String>,
    units: Vec<String>,
    unit: String,
    kind: ExtraFieldKind,
    group_id: Option<i32>,
}

impl Default for FieldDraft {
    /// Creates a new FieldDraft initialized with empty and sensible default values.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = FieldDraft::default();
    /// assert!(d.label.is_empty());
    /// assert!(d.description.is_empty());
    /// assert_eq!(d.required, false);
    /// assert_eq!(d.allow_multi_values, false);
    /// assert!(d.options.is_empty());
    /// assert!(d.units.is_empty());
    /// assert!(d.unit.is_empty());
    /// assert_eq!(d.kind, crate::models::extra_fields::ExtraFieldKind::Text);
    /// assert_eq!(d.group_id, None);
    /// ```
    fn default() -> Self {
        Self {
            label: String::new(),
            description: String::new(),
            required: false,
            allow_multi_values: false,
            options: Vec::new(),
            units: Vec::new(),
            unit: String::new(),
            kind: ExtraFieldKind::Text,
            group_id: None,
        }
    }
}

impl ExtraFieldsModel {
    /// Access the model's stored metadata fields as a slice.
    ///
    /// Returns a borrowed slice of `ExtraField` values in the model, in the same order they are stored.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given an `ExtraFieldsModel` named `model`, borrow its fields:
    /// let fields: &[crate::models::extra_fields::ExtraField] = model.fields();
    /// println!("{} fields loaded", fields.len());
    /// ```
    pub fn fields(&self) -> &[ExtraField] {
        &self.fields
    }

    /// Access the configured extra field groups for this model.
    ///
    /// Returns a slice of `ExtraFieldGroup` values in their current order.
    ///
    /// # Examples
    ///
    /// ```
    /// let model = ExtraFieldsModel::default();
    /// let groups: &[ExtraFieldGroup] = model.groups();
    /// // inspect or iterate
    /// for g in groups {
    ///     println!("{}", g.name);
    /// }
    /// ```
    pub fn groups(&self) -> &[ExtraFieldGroup] {
        &self.groups
    }

    /// Returns whether any stored extra field is currently invalid.
    ///
    /// # Returns
    ///
    /// `true` if at least one field in the model fails validation, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// // assumes `ExtraFieldsModel` implements `Default` and `has_invalid_fields` is available
    /// let model = crate::ui::components::extra_fields::ExtraFieldsModel::default();
    /// assert_eq!(model.has_invalid_fields(), false);
    /// ```
    pub fn has_invalid_fields(&self) -> bool {
        self.fields.iter().any(field_invalid)
    }

    /// Ensure a group named "Default" exists in the model and return its id.
    ///
    /// If a group named "Default" is already present this returns its `id`. If not,
    /// a new group named "Default" is appended and its newly assigned `id` is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::ui::components::extra_fields::ExtraFieldsModel;
    /// # use crate::models::extra_fields::ExtraField; // placeholder imports to satisfy example context
    /// # use crate::models::extra_fields::ExtraFieldGroup;
    /// let mut model = ExtraFieldsModel {
    ///     fields: Vec::new(),
    ///     groups: Vec::new(),
    ///     editing_group: None,
    ///     editing_group_buffer: String::new(),
    ///     editing_field: None,
    ///     modal_open: false,
    ///     modal_draft: None,
    /// };
    /// let id = model.ensure_default_group();
    /// assert_eq!(model.groups.iter().find(|g| g.name == "Default").unwrap().id, id);
    /// ```
    pub fn ensure_default_group(&mut self) -> i32 {
        if let Some(g) = self.groups.iter().find(|g| g.name == "Default") {
            return g.id;
        }
        let next_id = self.groups.iter().map(|g| g.id).max().unwrap_or(0) + 1;
        self.groups.push(ExtraFieldGroup {
            id: next_id,
            name: "Default".into(),
            position: self.groups.len() as i32,
        });
        next_id
    }

    /// Return the id of the group with the lowest position (ties broken by the lowest id), creating a `Default` group if none exist.
    ///
    /// # Returns
    ///
    /// The id of the group with the smallest position; if multiple groups share the same position the one with the smallest `id` is chosen. If the model has no groups, a `Default` group is created and its id is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut model = ExtraFieldsModel::default();
    /// // when no groups exist, a Default group will be created and returned
    /// let default_id = model.lowest_position_group_id();
    /// assert!(model.groups.iter().any(|g| g.id == default_id));
    /// ```
    pub fn lowest_position_group_id(&mut self) -> i32 {
        if self.groups.is_empty() {
            return self.ensure_default_group();
        }
        self.groups
            .iter()
            .min_by_key(|g| (g.position, g.id))
            .map(|g| g.id)
            .unwrap_or_else(|| self.ensure_default_group())
    }

    /// Get the display name for a group, falling back to "Default" when the group is missing.
    ///
    /// If `group_id` is `None` or does not match any existing group, this returns `"Default"`.
    ///
    /// # Parameters
    ///
    /// - `group_id`: Optional id of the group to look up; use `None` to obtain the default name.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // returns "Default" when no group matches
    /// let name = model.display_group_name(None);
    /// assert_eq!(name, "Default");
    /// ```
    pub fn display_group_name(&self, group_id: Option<i32>) -> String {
        group_id
            .and_then(|gid| self.groups.iter().find(|g| g.id == gid))
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "Default".to_string())
    }
}

/// Messages produced by the extra fields view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtraFieldsMsg {
    DraftKindChanged(ExtraFieldKind),
    RemoveField(usize),
    ImportRequested,
    ImportCancelled,
    ImportLoaded {
        fields: Vec<ExtraField>,
        groups: Vec<ExtraFieldGroup>,
        source: std::path::PathBuf,
    },
    ImportFailed(String),
    EditValue {
        index: usize,
        value: String,
    },
    ToggleCheckbox {
        index: usize,
        checked: bool,
    },
    SelectUnit {
        index: usize,
        unit: String,
    },
    UpdateMulti {
        index: usize,
        values: Vec<String>,
    },
    StartEditGroup(usize),
    EditGroupName(String),
    CommitGroupName,
    CancelGroupEdit,
    RemoveGroup(usize),
    AddGroup,
    StartAddField {
        group_id: Option<i32>,
    },
    OpenFieldModal(usize),
    CloseFieldModal,
    DraftLabelChanged(String),
    DraftDescChanged(String),
    DraftRequiredToggled(bool),
    DraftAllowMultiToggled(bool),
    DraftOptionChanged {
        index: usize,
        value: String,
    },
    DraftAddOption,
    DraftRemoveOption(usize),
    DraftUnitChanged {
        index: usize,
        value: String,
    },
    DraftAddUnit,
    DraftRemoveUnit(usize),
    DraftDefaultUnitChanged(String),
    DraftGroupChanged(Option<i32>),
    CommitFieldModal,
}

/// Commands that require side effects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtraFieldsCommand {
    PickMetadataFile,
}

/// Feedback surfaced to the status bar/modal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraFieldsEvent {
    pub message: String,
    pub is_error: bool,
}

/// Apply a UI message to the extra-fields model, mutating state, optionally enqueueing side-effect commands, and returning an optional status event.
///
/// This function updates `model` according to `msg`. It may push one or more `ExtraFieldsCommand` entries onto `cmds` to request side effects (for example, file picking), and it returns `Some(ExtraFieldsEvent)` when the action should surface a status or error to the caller; otherwise it returns `None`.
///
/// # Examples
///
/// ```
/// use crate::ui::components::extra_fields::{ExtraFieldsModel, ExtraFieldsMsg, ExtraFieldsCommand, update};
///
/// let mut model = ExtraFieldsModel::default();
/// let mut cmds = Vec::new();
///
/// // Some messages produce a status event
/// let ev = update(&mut model, ExtraFieldsMsg::ImportCancelled, &mut cmds);
/// assert!(ev.is_some());
/// assert_eq!(ev.unwrap().is_error, false);
///
/// // Some messages request a command instead of returning an event
/// let ev2 = update(&mut model, ExtraFieldsMsg::ImportRequested, &mut cmds);
/// assert!(ev2.is_none());
/// assert!(cmds.iter().any(|c| matches!(c, ExtraFieldsCommand::PickMetadataFile)));
/// ```
pub fn update(
    model: &mut ExtraFieldsModel,
    msg: ExtraFieldsMsg,
    cmds: &mut Vec<ExtraFieldsCommand>,
) -> Option<ExtraFieldsEvent> {
    match msg {
        ExtraFieldsMsg::ImportRequested => {
            cmds.push(ExtraFieldsCommand::PickMetadataFile);
            None
        }
        ExtraFieldsMsg::ImportCancelled => Some(ExtraFieldsEvent {
            message: "Metadata import cancelled.".to_string(),
            is_error: false,
        }),
        ExtraFieldsMsg::ImportFailed(err) => Some(ExtraFieldsEvent {
            message: err,
            is_error: true,
        }),
        ExtraFieldsMsg::ImportLoaded {
            mut fields,
            groups,
            source,
        } => {
            fields.sort_by(|a, b| a.cmp_key().cmp(&b.cmp_key()));
            model.fields = fields;
            model.groups = groups;
            model.editing_group = None;
            model.editing_group_buffer.clear();
            Some(ExtraFieldsEvent {
                message: format!(
                    "Imported {} field(s) from {}",
                    model.fields.len(),
                    source.display()
                ),
                is_error: false,
            })
        }
        ExtraFieldsMsg::EditValue { index, value } => {
            if let Some(field) = model.fields.get_mut(index) {
                field.value = value;
                // keep multi list in sync for multi selects
                if field.allow_multi_values {
                    field.value_multi = split_multi(&field.value);
                }
            }
            None
        }
        ExtraFieldsMsg::ToggleCheckbox { index, checked } => {
            if let Some(field) = model.fields.get_mut(index) {
                field.value = if checked { "on".into() } else { String::new() };
            }
            None
        }
        ExtraFieldsMsg::SelectUnit { index, unit } => {
            if let Some(field) = model.fields.get_mut(index) {
                field.unit = Some(unit);
            }
            None
        }
        ExtraFieldsMsg::UpdateMulti { index, values } => {
            if let Some(field) = model.fields.get_mut(index) {
                field.value_multi = values.clone();
                field.value = values.join(", ");
            }
            None
        }
        ExtraFieldsMsg::OpenFieldModal(idx) => {
            if let Some(f) = model.fields.get(idx) {
                model.modal_open = true;
                model.editing_field = Some(idx);
                model.modal_draft = Some(FieldDraft {
                    label: f.label.clone(),
                    description: f.description.clone().unwrap_or_default(),
                    required: f.required,
                    allow_multi_values: f.allow_multi_values,
                    options: f.options.clone(),
                    units: f.units.clone(),
                    unit: f.unit.clone().unwrap_or_default(),
                    kind: f.kind.clone(),
                    group_id: f.group_id,
                });
            }
            None
        }
        ExtraFieldsMsg::CloseFieldModal => {
            model.modal_open = false;
            model.modal_draft = None;
            model.editing_field = None;
            None
        }
        ExtraFieldsMsg::DraftLabelChanged(text) => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.label = text;
            }
            None
        }
        ExtraFieldsMsg::DraftDescChanged(text) => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.description = text;
            }
            None
        }
        ExtraFieldsMsg::DraftRequiredToggled(val) => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.required = val;
            }
            None
        }
        ExtraFieldsMsg::DraftKindChanged(kind) => {
            if model.editing_field.is_none()
                && let Some(d) = model.modal_draft.as_mut()
            {
                d.kind = kind;
                d.options.clear();
                d.units.clear();
                d.unit.clear();
                d.allow_multi_values = false;
            }
            None
        }
        ExtraFieldsMsg::DraftAllowMultiToggled(val) => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.allow_multi_values = val;
            }
            None
        }
        ExtraFieldsMsg::DraftOptionChanged { index, value } => {
            if let Some(d) = model.modal_draft.as_mut()
                && index < d.options.len()
            {
                d.options[index] = value;
            }
            None
        }
        ExtraFieldsMsg::DraftAddOption => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.options.push(String::new());
            }
            None
        }
        ExtraFieldsMsg::DraftRemoveOption(i) => {
            if let Some(d) = model.modal_draft.as_mut()
                && i < d.options.len()
            {
                d.options.remove(i);
            }
            None
        }
        ExtraFieldsMsg::DraftUnitChanged { index, value } => {
            if let Some(d) = model.modal_draft.as_mut()
                && index < d.units.len()
            {
                d.units[index] = value;
            }
            None
        }
        ExtraFieldsMsg::DraftAddUnit => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.units.push(String::new());
            }
            None
        }
        ExtraFieldsMsg::DraftRemoveUnit(i) => {
            if let Some(d) = model.modal_draft.as_mut()
                && i < d.units.len()
            {
                d.units.remove(i);
            }
            None
        }
        ExtraFieldsMsg::DraftDefaultUnitChanged(unit) => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.unit = unit;
            }
            None
        }
        ExtraFieldsMsg::DraftGroupChanged(group) => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.group_id = group;
            }
            None
        }
        ExtraFieldsMsg::StartAddField { group_id } => {
            model.modal_open = true;
            model.editing_field = None;
            let mut draft = FieldDraft::default();
            let preferred = model.lowest_position_group_id();
            draft.group_id = group_id.or(Some(preferred));
            model.modal_draft = Some(draft);
            None
        }
        ExtraFieldsMsg::RemoveField(index) => {
            if index < model.fields.len() {
                model.fields.remove(index);
            }
            None
        }
        ExtraFieldsMsg::CommitFieldModal => {
            if let Some(draft) = model.modal_draft.take() {
                if name_conflict(model, &draft.label, model.editing_field) {
                    // keep modal open; restore draft
                    model.modal_draft = Some(draft);
                    return Some(ExtraFieldsEvent {
                        message: "Field name must be unique".into(),
                        is_error: true,
                    });
                }

                if let Some(idx) = model.editing_field {
                    if let Some(f) = model.fields.get_mut(idx) {
                        apply_draft_to_field(&draft, f);
                    }
                } else {
                    let label = draft.label.trim().to_string();
                    if !label.is_empty() {
                        let mut draft = draft;
                        let preferred = model.lowest_position_group_id();
                        draft.group_id = draft.group_id.or(Some(preferred));
                        let mut new_field = ExtraField {
                            label,
                            kind: draft.kind.clone(),
                            value: String::new(),
                            value_multi: Vec::new(),
                            options: Vec::new(),
                            unit: None,
                            units: Vec::new(),
                            position: Some(model.fields.len() as i32),
                            required: false,
                            description: None,
                            allow_multi_values: false,
                            blank_value_on_duplicate: false,
                            group_id: draft.group_id,
                            readonly: false,
                        };
                        apply_draft_to_field(&draft, &mut new_field);
                        model.fields.push(new_field);
                    }
                }
            }
            model.modal_open = false;
            model.editing_field = None;
            None
        }
        ExtraFieldsMsg::StartEditGroup(idx) => {
            if let Some(g) = model.groups.get(idx) {
                model.editing_group = Some(idx);
                model.editing_group_buffer = g.name.clone();
            }
            None
        }
        ExtraFieldsMsg::AddGroup => {
            let next_id = model.groups.iter().map(|g| g.id).max().unwrap_or(0) + 1;
            model.groups.push(ExtraFieldGroup {
                id: next_id,
                name: format!("Group {}", next_id),
                position: model.groups.len() as i32,
            });
            None
        }
        ExtraFieldsMsg::EditGroupName(name) => {
            model.editing_group_buffer = name;
            None
        }
        ExtraFieldsMsg::CommitGroupName => {
            if let Some(idx) = model.editing_group
                && let Some(group) = model.groups.get_mut(idx)
            {
                group.name = model.editing_group_buffer.trim().to_string();
            }
            model.editing_group = None;
            model.editing_group_buffer.clear();
            None
        }
        ExtraFieldsMsg::CancelGroupEdit => {
            model.editing_group = None;
            model.editing_group_buffer.clear();
            None
        }
        ExtraFieldsMsg::RemoveGroup(idx) => {
            if let Some(group) = model.groups.get(idx).cloned() {
                let removing_last = model.groups.len() == 1;
                if removing_last {
                    if let Some(g) = model.groups.get_mut(idx) {
                        g.name = "Default".into();
                        for f in model.fields.iter_mut() {
                            if f.group_id == Some(group.id) {
                                f.group_id = Some(g.id);
                            }
                        }
                    }
                } else {
                    model.groups.remove(idx);

                    // Ensure a Default group exists for reassignment if needed
                    let default_id = model.ensure_default_group();

                    for f in model.fields.iter_mut() {
                        if f.group_id == Some(group.id) {
                            f.group_id = Some(default_id);
                        }
                    }
                }
            }
            None
        }
    }
}

/// Render the "Metadata" UI for editing/importing extra fields and collect any user actions as messages.
///
/// The function builds the collapsible "Metadata" section, group/field controls, and the field edit modal (if open),
/// returning a vector of `ExtraFieldsMsg` values representing user-triggered actions during this render pass.
///
/// # Examples
///
/// ```no_run
/// # use egui;
/// # use crate::ui::components::extra_fields::{view, ExtraFieldsModel, ExtraFieldsMsg};
/// // In an actual egui application you would call `view` from within a UI frame:
/// // let mut model = ExtraFieldsModel::default();
/// // let mut ctx = egui::CtxRef::default();
/// // egui::CentralPanel::default().show(&ctx, |ui| {
/// //     let msgs: Vec<ExtraFieldsMsg> = view(ui, &model);
/// //     // handle msgs...
/// // });
/// ```
pub fn view(ui: &mut egui::Ui, model: &ExtraFieldsModel) -> Vec<ExtraFieldsMsg> {
    let mut msgs = Vec::new();

    egui::CollapsingHeader::new("Metadata")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(format!(
                        "{} Add group",
                        egui_phosphor::regular::FOLDER_PLUS
                    )))
                    .clicked()
                {
                    msgs.push(ExtraFieldsMsg::AddGroup);
                }
                if ui
                    .add(egui::Button::new(format!(
                        "{} Add field",
                        egui_phosphor::regular::PLUS
                    )))
                    .clicked()
                {
                    msgs.push(ExtraFieldsMsg::StartAddField { group_id: None });
                }
                if ui
                    .add(egui::Button::new(format!(
                        "{} Import JSON",
                        egui_phosphor::regular::FILE_ARROW_DOWN
                    )))
                    .clicked()
                {
                    msgs.push(ExtraFieldsMsg::ImportRequested);
                }
            });

            ui.add_space(6.0);

            ui.label(
                egui::RichText::new(
                    "Imports eLabFTW-style extra_fields JSON. Field options are imported; you can adjust values before saving."
                )
                .small()
                .color(egui::Color32::from_gray(110)),
            );

            ui.add_space(10.0);
            render_fields(ui, model, &mut msgs);
        });

    render_field_modal(ui.ctx(), model, &mut msgs);

    msgs
}

/// Renders all metadata groups and their fields into the given UI, producing UI-driven messages.
///
/// The function draws a placeholder when there are no groups or fields, then iterates over the
/// model's groups in order and renders a collapsible section for each group. Inside each group it
/// renders the group header (controls to rename/remove/add), the group's fields (each field row
/// and its value editor), and a button to add a new field scoped to that group. User interactions
/// are recorded as messages appended to `msgs`.
///
/// # Examples
///
/// ```no_run
/// # use egui;
/// # use crate::ui::components::{ExtraFieldsModel, ExtraFieldsMsg, render_fields};
/// // Within an egui painting/layout callback you would call:
/// // let mut msgs = Vec::new();
/// // render_fields(&mut ui, &model, &mut msgs);
/// ```
fn render_fields(ui: &mut egui::Ui, model: &ExtraFieldsModel, msgs: &mut Vec<ExtraFieldsMsg>) {
    if model.fields.is_empty() && model.groups.is_empty() {
        ui.label(
            egui::RichText::new("No metadata yet. Add a group or import JSON to begin.")
                .italics()
                .color(egui::Color32::from_gray(110)),
        );
    }

    // Render grouped fields in group order, collapsible.
    for group in model.groups.iter() {
        let group_fields: Vec<(usize, &ExtraField)> = model
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| f.group_id == Some(group.id))
            .collect();

        egui::CollapsingHeader::new(group.name.clone())
            .id_salt(format!("extra-group-{}", group.id))
            .default_open(true)
            .show(ui, |ui| {
                // Header controls inside the collapsible header area.
                render_group_header(ui, group, msgs, model);
                ui.add_space(4.0);
                if group_fields.is_empty() {
                    ui.label(
                        egui::RichText::new("No fields in this group yet.")
                            .italics()
                            .color(egui::Color32::from_gray(120)),
                    );
                } else {
                    for (idx, field) in group_fields {
                        render_field(ui, field, idx, msgs);
                        ui.add_space(6.0);
                    }
                }

                if ui
                    .add(egui::Button::new(format!(
                        "{} Add field to {}",
                        egui_phosphor::regular::PLUS,
                        group.name
                    )))
                    .clicked()
                {
                    msgs.push(ExtraFieldsMsg::StartAddField {
                        group_id: Some(group.id),
                    });
                }
            });

        ui.add_space(10.0);
    }
}

/// Render the header for a group row and emit user-driven `ExtraFieldsMsg` actions.
///
/// If the provided `group` is currently being edited the header shows Cancel/Save
/// controls and an inline text edit for the group name. Otherwise it shows Rename
/// and (when there is more than one group) Remove controls. User interactions are
/// appended to `msgs`.
///
/// # Parameters
///
/// - `ui`: the egui UI to render into.
/// - `group`: the group being rendered.
/// - `msgs`: a mutable vector receiving messages produced by user actions.
/// - `model`: the current `ExtraFieldsModel` used to detect editing state and buffers.
///
/// # Examples
///
/// ```no_run
/// use crate::models::extra_fields::{ExtraFieldGroup, ExtraFieldsModel, ExtraFieldsMsg};
///
/// // During an egui paint pass obtain `ui` from the framework and call:
/// let mut msgs = Vec::new();
/// let group = ExtraFieldGroup { id: 1, name: "Group".into(), position: 0 };
/// let model = ExtraFieldsModel::default();
/// // render_group_header(&mut ui, &group, &mut msgs, &model);
/// ```
fn render_group_header(
    ui: &mut egui::Ui,
    group: &ExtraFieldGroup,
    msgs: &mut Vec<ExtraFieldsMsg>,
    model: &ExtraFieldsModel,
) {
    let is_editing = model
        .editing_group
        .map(|idx| {
            model
                .groups
                .get(idx)
                .map(|g| g.id == group.id)
                .unwrap_or(false)
        })
        .unwrap_or(false);

    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        if is_editing {
            if ui.button(egui_phosphor::regular::X).clicked() {
                msgs.push(ExtraFieldsMsg::CancelGroupEdit);
            }
            if ui.button(egui_phosphor::regular::CHECK).clicked() {
                msgs.push(ExtraFieldsMsg::CommitGroupName);
            }
            let mut text = model.editing_group_buffer.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut text).hint_text("Group name"))
                .changed()
            {
                msgs.push(ExtraFieldsMsg::EditGroupName(text));
            }
        } else {
            if model.groups.len() > 1
                && ui
                    .button(egui_phosphor::regular::TRASH)
                    .on_hover_text("Remove group")
                    .clicked()
                && let Some(idx) = model.groups.iter().position(|g| g.id == group.id)
            {
                msgs.push(ExtraFieldsMsg::RemoveGroup(idx));
            }
            if ui
                .button(egui_phosphor::regular::PENCIL_SIMPLE)
                .on_hover_text("Rename group")
                .clicked()
                && let Some(idx) = model.groups.iter().position(|g| g.id == group.id)
            {
                msgs.push(ExtraFieldsMsg::StartEditGroup(idx));
            }
        }
    });
}

/// Render a single metadata field row including its label, edit/remove actions, optional description, and the kind-specific value editor.
///
/// The row is visually marked when the field is invalid. Clicking the trash or pencil icons pushes `RemoveField` or `OpenFieldModal` messages into the provided `msgs` vector.
///
/// # Examples
///
/// ```rust,no_run
/// // Called from an egui paint callback where `ui: &mut egui::Ui` is available.
/// // `field` is an `ExtraField` from your model and `msgs` is a mutable Vec<ExtraFieldsMsg>.
/// // `idx` is the index of the field in the model's fields list.
/// render_field(ui, &field, idx, &mut msgs);
/// ```
fn render_field(ui: &mut egui::Ui, field: &ExtraField, idx: usize, msgs: &mut Vec<ExtraFieldsMsg>) {
    let invalid = field_invalid(field);
    let mut frame = egui::Frame::group(ui.style()).stroke(if invalid {
        egui::Stroke::new(1.0, egui::Color32::from_rgb(200, 80, 80))
    } else {
        egui::Stroke::new(
            1.0,
            ui.style().visuals.widgets.noninteractive.bg_stroke.color,
        )
    });
    if invalid {
        // Use a translucent overlay that adapts to theme.
        let base = ui.style().visuals.extreme_bg_color; // typically background
        let tint = egui::Color32::from_rgb(200, 80, 80);
        let overlay = egui::Color32::from_rgba_unmultiplied(
            ((base.r() as u16 + tint.r() as u16) / 2) as u8,
            ((base.g() as u16 + tint.g() as u16) / 2) as u8,
            ((base.b() as u16 + tint.b() as u16) / 2) as u8,
            30,
        );
        frame = frame.fill(overlay);
    }

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let mut label = field.label.clone();
            if field.required {
                label.push_str(" *");
            }
            ui.label(label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui_phosphor::regular::TRASH)
                    .on_hover_text("Remove field")
                    .clicked()
                {
                    msgs.push(ExtraFieldsMsg::RemoveField(idx));
                }
                if ui
                    .button(egui_phosphor::regular::PENCIL_SIMPLE)
                    .on_hover_text("Edit field")
                    .clicked()
                {
                    msgs.push(ExtraFieldsMsg::OpenFieldModal(idx));
                }
            });
        });

        if let Some(desc) = &field.description {
            ui.label(
                egui::RichText::new(desc)
                    .small()
                    .color(egui::Color32::from_gray(120)),
            );
        }

        ui.add_space(4.0);
        render_field_value(ui, field, idx, msgs);
        ui.add_space(6.0);
    });
}

/// Render the field's value editor inside a grouped UI block.
///
/// Chooses the editor widget based on the field's kind (checkbox, select/radio,
/// number, or text) and emits UI-generated messages into `msgs`.
///
/// # Examples
///
/// ```rust,no_run
/// // Assume `ui: &mut egui::Ui`, `field: &ExtraField`, and `msgs: &mut Vec<ExtraFieldsMsg>`
/// // are available in the calling context.
/// render_field_value(ui, field, 0, msgs);
/// ```
fn render_field_value(
    ui: &mut egui::Ui,
    field: &ExtraField,
    idx: usize,
    msgs: &mut Vec<ExtraFieldsMsg>,
) {
    ui.group(|ui| match field.kind {
        ExtraFieldKind::Checkbox => render_checkbox(ui, field, idx, msgs),
        ExtraFieldKind::Select | ExtraFieldKind::Radio => render_options(ui, field, idx, msgs),
        ExtraFieldKind::Number => render_number(ui, field, idx, msgs),
        _ => render_text_input(ui, field, idx, msgs),
    });
}

/// Renders a single checkbox for a metadata field and emits a `ToggleCheckbox` message when the user changes it.
///
/// The checkbox is considered checked when the field's value equals `"on"`. If the user toggles the control,
/// a `ExtraFieldsMsg::ToggleCheckbox { index, checked }` is pushed to `msgs`.
///
/// # Examples
///
/// ```
/// // Pseudocode example showing usage:
/// // let mut ui = ...; // egui::Ui
/// // let field = ExtraField { value: "on".into(), ..Default::default() };
/// // let mut msgs = Vec::new();
/// // render_checkbox(&mut ui, &field, 0, &mut msgs);
/// // // If the user toggled the checkbox, msgs will contain a ToggleCheckbox message.
/// ```
fn render_checkbox(
    ui: &mut egui::Ui,
    field: &ExtraField,
    idx: usize,
    msgs: &mut Vec<ExtraFieldsMsg>,
) {
    let mut checked = field.value == "on";
    if ui.checkbox(&mut checked, "Checked").changed() {
        msgs.push(ExtraFieldsMsg::ToggleCheckbox {
            index: idx,
            checked,
        });
    }
}

/// Renders a field's selectable options as either checkboxes (for multi-select) or radio buttons (for single-select),
/// and appends the appropriate `ExtraFieldsMsg` when the user changes a selection.
///
/// - For fields with `allow_multi_values == true`, renders a checkbox per option, maintains the chosen set
///   (preferring `value_multi` when present, otherwise parsing `value`), and pushes `ExtraFieldsMsg::UpdateMulti { index, values }`
///   with the updated values on change.
/// - For single-selection fields, renders a radio button per option and pushes `ExtraFieldsMsg::EditValue { index, value }`
///   when an option is selected.
///
/// # Examples
///
/// ```
/// // Given a mutable `ui` (egui::Ui), an `ExtraField` named `field`, and a `msgs` vec:
/// // render_options(&mut ui, &field, 0, &mut msgs);
/// ```
fn render_options(
    ui: &mut egui::Ui,
    field: &ExtraField,
    idx: usize,
    msgs: &mut Vec<ExtraFieldsMsg>,
) {
    if field.allow_multi_values {
        let mut chosen = if field.value_multi.is_empty() {
            split_multi(&field.value)
        } else {
            field.value_multi.clone()
        };
        for opt in &field.options {
            let mut is_on = chosen.contains(opt);
            if ui.checkbox(&mut is_on, opt).changed() {
                if is_on {
                    chosen.push(opt.clone());
                } else {
                    chosen.retain(|v| v != opt);
                }
                msgs.push(ExtraFieldsMsg::UpdateMulti {
                    index: idx,
                    values: chosen.clone(),
                });
            }
        }
    } else {
        let mut current = field.value.clone();
        for opt in &field.options {
            if ui.radio_value(&mut current, opt.clone(), opt).clicked() {
                msgs.push(ExtraFieldsMsg::EditValue {
                    index: idx,
                    value: opt.clone(),
                });
            }
        }
    }
}

/// Renders a horizontal numeric input for an ExtraField and, if present, a unit selector.
///
/// Emits `ExtraFieldsMsg::EditValue` when the numeric text changes and
/// `ExtraFieldsMsg::SelectUnit` when a different unit is chosen.
///
/// # Examples
///
/// ```
/// // Within an egui painting function:
/// // let mut msgs = Vec::new();
/// // render_number(&mut ui, &field, index, &mut msgs);
/// ```
fn render_number(
    ui: &mut egui::Ui,
    field: &ExtraField,
    idx: usize,
    msgs: &mut Vec<ExtraFieldsMsg>,
) {
    ui.horizontal(|ui| {
        let mut val = field.value.clone();
        let disabled = field.readonly;
        let resp = ui.add_enabled(!disabled, egui::TextEdit::singleline(&mut val));
        if resp.changed() {
            msgs.push(ExtraFieldsMsg::EditValue {
                index: idx,
                value: val,
            });
        }
        if !field.units.is_empty() {
            let mut current_unit = field.unit.clone().unwrap_or_default();
            egui::ComboBox::from_id_salt(format!("extra-unit-{}", idx))
                .width(90.0)
                .selected_text(if current_unit.is_empty() {
                    "Unit"
                } else {
                    &current_unit
                })
                .show_ui(ui, |ui| {
                    for unit in &field.units {
                        if ui
                            .selectable_value(&mut current_unit, unit.clone(), unit)
                            .clicked()
                        {
                            msgs.push(ExtraFieldsMsg::SelectUnit {
                                index: idx,
                                unit: unit.clone(),
                            });
                        }
                    }
                });
        }
    });
}

/// Render a single-line text input for the given field and enqueue an `EditValue` message when the text changes.
///
/// The input uses `field_hint(kind)` as the hint text and is disabled when `field.readonly` is true.
/// When the user edits the value, an `ExtraFieldsMsg::EditValue { index, value }` is pushed into `msgs`.
///
/// # Examples
///
/// ```ignore
/// // Illustration (requires an egui context and real ExtraField)
/// # use crate::ui::components::extra_fields::{render_text_input, ExtraFieldsMsg};
/// # use crate::models::extra_fields::ExtraField;
/// # use egui::Ui;
/// // let mut ui: Ui = ...;
/// // let field = ExtraField::default();
/// // let mut msgs = Vec::new();
/// // render_text_input(&mut ui, &field, 0, &mut msgs);
/// ```
fn render_text_input(
    ui: &mut egui::Ui,
    field: &ExtraField,
    idx: usize,
    msgs: &mut Vec<ExtraFieldsMsg>,
) {
    let mut val = field.value.clone();
    let disabled = field.readonly;
    let resp = ui.add_enabled(
        !disabled,
        egui::TextEdit::singleline(&mut val).hint_text(field_hint(&field.kind)),
    );
    if resp.changed() {
        msgs.push(ExtraFieldsMsg::EditValue {
            index: idx,
            value: val,
        });
    }
}

/// Provide a user-facing hint string appropriate for the given field kind.
///
/// The returned static string suggests the expected input format or example content for the field kind.
/// Returns an empty string when no hint is applicable for the kind.
///
/// # Examples
///
/// ```
/// // Example assertions; adjust paths as needed when used from a different module.
/// assert_eq!(field_hint(&ExtraFieldKind::Date), "YYYY-MM-DD");
/// assert_eq!(field_hint(&ExtraFieldKind::Url), "https://example.com");
/// assert_eq!(field_hint(&ExtraFieldKind::Text), "");
/// ```
fn field_hint(kind: &ExtraFieldKind) -> &'static str {
    match kind {
        ExtraFieldKind::Date => "YYYY-MM-DD",
        ExtraFieldKind::DateTimeLocal => "YYYY-MM-DDTHH:MM",
        ExtraFieldKind::Time => "HH:MM",
        ExtraFieldKind::Url => "https://example.com",
        ExtraFieldKind::Email => "name@example.com",
        ExtraFieldKind::Number => "Number",
        ExtraFieldKind::Items | ExtraFieldKind::Experiments | ExtraFieldKind::Users => "Numeric ID",
        _ => "",
    }
}

/// Provides a human-readable label for an `ExtraFieldKind`.
///
/// The display label for the provided kind.
///
/// # Examples
///
/// ```
/// use crate::models::extra_fields::ExtraFieldKind;
/// assert_eq!(super::kind_label(&ExtraFieldKind::Number), "Number");
/// assert_eq!(super::kind_label(&ExtraFieldKind::DateTimeLocal), "Date/time");
/// ```
fn kind_label(kind: &ExtraFieldKind) -> &'static str {
    match kind {
        ExtraFieldKind::Text => "Text",
        ExtraFieldKind::Number => "Number",
        ExtraFieldKind::Select => "Select",
        ExtraFieldKind::Checkbox => "Checkbox",
        ExtraFieldKind::Date => "Date",
        ExtraFieldKind::DateTimeLocal => "Date/time",
        ExtraFieldKind::Time => "Time",
        ExtraFieldKind::Url => "URL",
        ExtraFieldKind::Email => "Email",
        ExtraFieldKind::Radio => "Radio",
        ExtraFieldKind::Items => "Items",
        ExtraFieldKind::Experiments => "Experiments",
        ExtraFieldKind::Users => "Users",
        ExtraFieldKind::Unknown(_) => "Unknown",
    }
}

/// Returns the set of supported field kinds in the fixed UI order.
///
/// The returned vector contains every `ExtraFieldKind` supported by the metadata editor,
/// ordered for display and selection.
///
/// # Examples
///
/// ```
/// use crate::models::extra_fields::ExtraFieldKind;
/// let kinds = all_kinds();
/// assert!(kinds.contains(&ExtraFieldKind::Text));
/// assert_eq!(kinds.first(), Some(&ExtraFieldKind::Text));
/// assert_eq!(kinds.len(), 13);
/// ```
fn all_kinds() -> Vec<ExtraFieldKind> {
    vec![
        ExtraFieldKind::Text,
        ExtraFieldKind::Number,
        ExtraFieldKind::Select,
        ExtraFieldKind::Checkbox,
        ExtraFieldKind::Date,
        ExtraFieldKind::DateTimeLocal,
        ExtraFieldKind::Time,
        ExtraFieldKind::Url,
        ExtraFieldKind::Email,
        ExtraFieldKind::Radio,
        ExtraFieldKind::Items,
        ExtraFieldKind::Experiments,
        ExtraFieldKind::Users,
    ]
}

/// Splits a comma-separated string into trimmed, non-empty values.
///
/// Leading and trailing whitespace around each item is removed and empty segments are discarded.
///
/// # Examples
///
/// ```
/// let parts = split_multi(" a, b ,,c , ");
/// assert_eq!(parts, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
/// ```
fn split_multi(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Checks whether `label` conflicts with any existing field label in `model`, using case-insensitive comparison.
///
/// Leading and trailing whitespace in `label` is ignored; an empty or whitespace-only `label` never conflicts.
/// The `editing` parameter, if provided, excludes the field at that index from the conflict check (useful when validating an in-place edit).
///
/// # Returns
///
/// `true` if another field's label equals `label` case-insensitively after trimming, `false` otherwise.
///
/// # Examples
///
/// ```ignore
/// // Example (conceptual): returns true when a different field already has the same label (case-insensitive)
/// let conflict = name_conflict(&model, "Username", Some(editing_index));
/// ```
fn name_conflict(model: &ExtraFieldsModel, label: &str, editing: Option<usize>) -> bool {
    let key = label.trim().to_lowercase();
    if key.is_empty() {
        return false;
    }
    model.fields.iter().enumerate().any(|(idx, f)| {
        idx != editing.unwrap_or(usize::MAX) && f.label.trim().eq_ignore_ascii_case(&key)
    })
}

/// Determines whether an `ExtraField` fails validation.
///
/// # Returns
///
/// `true` if the field is invalid, `false` otherwise.
///
/// # Examples
///
/// ```
/// use crate::models::extra_fields::ExtraField;
/// // Construct a default field and check validation status
/// let field = ExtraField::default();
/// let _is_invalid = field_invalid(&field);
/// ```
fn field_invalid(field: &ExtraField) -> bool {
    validate_field(field).is_some()
}

/// Trim the given input and return `None` when the trimmed result is empty.
///
/// # Examples
///
/// ```
/// assert_eq!(trimmed_or_none("  hello "), Some("hello".to_string()));
/// assert_eq!(trimmed_or_none("\n\t  "), None);
/// assert_eq!(trimmed_or_none("world"), Some("world".to_string()));
/// ```
fn trimmed_or_none(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Apply values from a `FieldDraft` to an existing `ExtraField`.
///
/// Only non-empty draft label replaces the field's label; description, required flag,
/// multi-value allowance, and group assignment are always copied. For fields of kind
/// `Select` or `Radio`, the draft's `options` replace the field's `options`. For
/// `Number` fields, the draft's `units` and `unit` are copied (unit is trimmed and
/// treated as `None` when empty).
///
/// # Examples
///
/// ```
/// use crate::models::extra_fields::{ExtraField, ExtraFieldKind};
///
/// let draft = crate::ui::components::extra_fields::FieldDraft {
///     label: " New Label ".into(),
///     description: "desc".into(),
///     required: true,
///     allow_multi_values: false,
///     options: vec!["a".into(), "b".into()],
///     units: vec!["kg".into()],
///     unit: "kg".into(),
///     kind: ExtraFieldKind::Select,
///     group_id: Some(1),
/// };
///
/// let mut field = ExtraField {
///     id: 0,
///     label: "old".into(),
///     description: None,
///     required: false,
///     allow_multi_values: true,
///     options: vec![],
///     units: vec![],
///     unit: None,
///     kind: ExtraFieldKind::Select,
///     group_id: None,
///     value: String::new(),
///     value_multi: Vec::new(),
///     position: 0,
/// };
///
/// crate::ui::components::extra_fields::apply_draft_to_field(&draft, &mut field);
///
/// assert_eq!(field.label, "New Label");
/// assert_eq!(field.description.as_deref(), Some("desc"));
/// assert!(field.required);
/// assert_eq!(field.options, vec!["a".to_string(), "b".to_string()]);
/// assert_eq!(field.group_id, Some(1));
/// ```
fn KEEP_EXISTING
fn apply_draft_to_field(draft: &FieldDraft, field: &mut ExtraField) {
    let label = draft.label.trim();
    if !label.is_empty() {
        field.label = label.to_string();
    }
    field.description = trimmed_or_none(&draft.description);
    field.required = draft.required;
    field.allow_multi_values = draft.allow_multi_values;
    field.group_id = draft.group_id;

    if matches!(field.kind, ExtraFieldKind::Select | ExtraFieldKind::Radio) {
        field.options = draft.options.clone();
    }

    if matches!(field.kind, ExtraFieldKind::Number) {
        field.units = draft.units.clone();
        field.unit = trimmed_or_none(&draft.unit);
    }
}

/// Render the "Edit field" modal when the model indicates a field is being edited or created.
///
/// Shows a centered, resizable window containing the draft's title, type (when creating),
/// description, required toggle, kind-specific controls (options for Select/Radio, units for Number),
/// group assignment, and Save/Cancel actions. User interactions are translated into `ExtraFieldsMsg`
/// entries pushed into the provided `msgs` vector.
///
/// # Examples
///
/// ```
/// // Pseudo-usage: obtain an `egui::Context`, an `ExtraFieldsModel` with `modal_open = true`
/// // and a populated `modal_draft`, then collect messages produced by the modal UI:
/// # use crate::ui::components::extra_fields::{ExtraFieldsModel, ExtraFieldsMsg};
/// # use egui::Context;
/// let ctx: &Context = /* egui context from your app */ unimplemented!();
/// let model: &ExtraFieldsModel = /* model with modal_open and modal_draft set */ unimplemented!();
/// let mut msgs = Vec::new();
/// render_field_modal(ctx, model, &mut msgs);
/// // `msgs` will contain messages corresponding to user interactions with the modal.
/// ```
fn render_field_modal(
    ctx: &egui::Context,
    model: &ExtraFieldsModel,
    msgs: &mut Vec<ExtraFieldsMsg>,
) {
    if !model.modal_open {
        return;
    }
    let Some(draft) = model.modal_draft.clone() else {
        return;
    };

    let conflict = model
        .modal_draft
        .as_ref()
        .map(|d| name_conflict(model, &d.label, model.editing_field))
        .unwrap_or(false);

    egui::Window::new("Edit field")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            let can_save = !draft.label.trim().is_empty() && !conflict;

            ui.set_width(ui.available_width().max(420.0));

            ui.label("Title");
            let mut title = draft.label.clone();
            if ui.text_edit_singleline(&mut title).changed() {
                msgs.push(ExtraFieldsMsg::DraftLabelChanged(title));
            }
            // Reserve space even when no conflict to avoid layout jump.
            ui.add_space(2.0);
            let color = if conflict {
                egui::Color32::from_rgb(200, 80, 80)
            } else {
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 0)
            };
            let text = egui::RichText::new("Field name must be unique").color(color);
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(text);
            });

            if model.editing_field.is_none() {
                ui.add_space(8.0);
                ui.label("Field type");
                let mut kind = draft.kind.clone();
                egui::ComboBox::from_id_salt("extra-field-kind")
                    .selected_text(kind_label(&kind))
                    .show_ui(ui, |ui| {
                        for k in all_kinds() {
                            if ui
                                .selectable_value(&mut kind, k.clone(), kind_label(&k))
                                .clicked()
                            {
                                msgs.push(ExtraFieldsMsg::DraftKindChanged(k));
                            }
                        }
                    });
            }

            ui.add_space(8.0);
            ui.label("Description");
            let mut desc = draft.description.clone();
            if ui
                .add(egui::TextEdit::multiline(&mut desc).desired_rows(3))
                .changed()
            {
                msgs.push(ExtraFieldsMsg::DraftDescChanged(desc));
            }

            ui.add_space(8.0);
            let mut required = draft.required;
            if ui.checkbox(&mut required, "Required").changed() {
                msgs.push(ExtraFieldsMsg::DraftRequiredToggled(required));
            }

            ui.add_space(8.0);
            match draft.kind {
                ExtraFieldKind::Select | ExtraFieldKind::Radio => {
                    let mut allow_multi = draft.allow_multi_values;
                    if ui.checkbox(&mut allow_multi, "Allow multiple").changed() {
                        msgs.push(ExtraFieldsMsg::DraftAllowMultiToggled(allow_multi));
                    }
                    ui.add_space(6.0);
                    ui.label("Options");
                    for (i, opt) in draft.options.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let mut v = opt.clone();
                            if ui.text_edit_singleline(&mut v).changed() {
                                msgs.push(ExtraFieldsMsg::DraftOptionChanged {
                                    index: i,
                                    value: v,
                                });
                            }
                            if ui.button(egui_phosphor::regular::TRASH).clicked() {
                                msgs.push(ExtraFieldsMsg::DraftRemoveOption(i));
                            }
                        });
                    }
                    if ui.button(egui_phosphor::regular::PLUS).clicked() {
                        msgs.push(ExtraFieldsMsg::DraftAddOption);
                    }
                }
                ExtraFieldKind::Number => {
                    ui.label("Units");
                    for (i, unit) in draft.units.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let mut v = unit.clone();
                            if ui.text_edit_singleline(&mut v).changed() {
                                msgs.push(ExtraFieldsMsg::DraftUnitChanged { index: i, value: v });
                            }
                            if ui.button(egui_phosphor::regular::TRASH).clicked() {
                                msgs.push(ExtraFieldsMsg::DraftRemoveUnit(i));
                            }
                        });
                    }
                    if ui.button(egui_phosphor::regular::PLUS).clicked() {
                        msgs.push(ExtraFieldsMsg::DraftAddUnit);
                    }
                    ui.add_space(6.0);
                    ui.label("Default unit");
                    let mut unit = draft.unit.clone();
                    if ui.text_edit_singleline(&mut unit).changed() {
                        msgs.push(ExtraFieldsMsg::DraftDefaultUnitChanged(unit));
                    }
                }
                _ => {}
            }

            ui.add_space(8.0);
            ui.label("Group assignment");
            let mut current = draft.group_id.unwrap_or(-1);
            let display_name = model.display_group_name(draft.group_id);

            egui::ComboBox::from_label("Group")
                .selected_text(display_name)
                .show_ui(ui, |ui| {
                    for g in &model.groups {
                        if ui.selectable_value(&mut current, g.id, &g.name).clicked() {
                            msgs.push(ExtraFieldsMsg::DraftGroupChanged(Some(g.id)));
                        }
                    }
                });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let save_btn = ui.add_enabled(can_save, egui::Button::new("Save"));
                if save_btn.clicked() && can_save {
                    msgs.push(ExtraFieldsMsg::CommitFieldModal);
                }
                if ui.button("Cancel").clicked() {
                    msgs.push(ExtraFieldsMsg::CloseFieldModal);
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Creates a new `ExtraField` with the given label and kind, initialized with empty/default values.
    ///
    /// The provided `label` is copied into the field and `kind` is assigned; all other properties are
    /// set to empty or default states (empty strings/vectors, `None` for optional fields, and `false` for booleans).
    ///
    /// # Examples
    ///
    /// ```
    /// let f = make_field("Temperature", ExtraFieldKind::Number);
    /// assert_eq!(f.label, "Temperature");
    /// assert_eq!(f.kind, ExtraFieldKind::Number);
    /// assert!(f.value.is_empty());
    /// assert!(f.options.is_empty());
    /// ```
    fn make_field(label: &str, kind: ExtraFieldKind) -> ExtraField {
        ExtraField {
            label: label.into(),
            kind,
            value: String::new(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        }
    }

    /// Create an ExtraFieldGroup with the given `id` and `name`, initializing `position` to 0.
    ///
    /// # Examples
    ///
    /// ```
    /// let g = make_group(42, "Group A");
    /// assert_eq!(g.id, 42);
    /// assert_eq!(g.name, "Group A");
    /// assert_eq!(g.position, 0);
    /// ```
    fn make_group(id: i32, name: &str) -> ExtraFieldGroup {
        ExtraFieldGroup {
            id,
            name: name.into(),
            position: 0,
        }
    }

    #[test]
    fn import_loaded_populates_model() {
        let mut model = ExtraFieldsModel::default();
        let mut cmds = Vec::new();
        let msg = ExtraFieldsMsg::ImportLoaded {
            fields: vec![ExtraField {
                label: "Example".into(),
                kind: ExtraFieldKind::Text,
                value: "value".into(),
                value_multi: Vec::new(),
                options: vec![],
                unit: None,
                units: vec![],
                position: Some(2),
                required: true,
                description: None,
                allow_multi_values: false,
                blank_value_on_duplicate: false,
                group_id: None,
                readonly: false,
            }],
            groups: vec![],
            source: PathBuf::from("sample.json"),
        };

        let event = update(&mut model, msg, &mut cmds).unwrap();
        assert!(cmds.is_empty());
        assert_eq!(model.fields.len(), 1);
        assert!(event.message.contains("Imported 1 field"));
        assert!(!event.is_error);
    }

    #[test]
    fn required_empty_marks_invalid() {
        let mut model = ExtraFieldsModel::default();
        let mut f = make_field("Req", ExtraFieldKind::Text);
        f.required = true;
        model.fields.push(f);

        assert!(model.has_invalid_fields());
    }

    #[test]
    fn invalid_number_marks_invalid() {
        let mut model = ExtraFieldsModel::default();
        let mut f = make_field("Num", ExtraFieldKind::Number);
        f.value = "abc".into();
        model.fields.push(f);

        assert!(model.has_invalid_fields());
    }

    #[test]
    fn valid_integer_id_is_accepted() {
        let mut model = ExtraFieldsModel::default();
        let mut f = make_field("ID", ExtraFieldKind::Users);
        f.value = "123".into();
        model.fields.push(f);

        assert!(!model.has_invalid_fields());
    }

    #[test]
    fn group_display_name_falls_back_to_id() {
        let mut model = ExtraFieldsModel::default();
        model.groups.push(make_group(1, "One"));

        assert_eq!(model.display_group_name(Some(1)), "One");
        assert_eq!(model.display_group_name(Some(99)), "Default");
    }

    #[test]
    fn add_field_uses_existing_group_when_present() {
        let mut model = ExtraFieldsModel::default();
        model.groups.push(make_group(10, "Group 1"));

        let mut cmds = Vec::new();
        let _ = update(
            &mut model,
            ExtraFieldsMsg::StartAddField { group_id: None },
            &mut cmds,
        );
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftLabelChanged("Example".into()),
            &mut cmds,
        );
        let _ = update(&mut model, ExtraFieldsMsg::CommitFieldModal, &mut cmds);

        assert_eq!(model.groups.len(), 1, "should not create new default group");
        assert_eq!(model.fields.len(), 1);
        assert_eq!(model.fields[0].group_id, Some(10));
    }

    #[test]
    fn remove_field_drops_entry() {
        let mut model = ExtraFieldsModel::default();
        model.fields.push(ExtraField {
            label: "One".into(),
            kind: ExtraFieldKind::Text,
            value: "a".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        });

        let _ = update(&mut model, ExtraFieldsMsg::RemoveField(0), &mut Vec::new());
        assert!(model.fields.is_empty());
    }

    #[test]
    fn modal_save_updates_field() {
        let mut model = ExtraFieldsModel::default();
        model.fields.push(ExtraField {
            label: "Old".into(),
            kind: ExtraFieldKind::Select,
            value: "A".into(),
            value_multi: vec!["A".into()],
            options: vec!["A".into(), "B".into()],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: Some("desc".into()),
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        });
        let mut cmds = Vec::new();
        let _ = update(&mut model, ExtraFieldsMsg::OpenFieldModal(0), &mut cmds);
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftLabelChanged("New".into()),
            &mut cmds,
        );
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftDescChanged("ndesc".into()),
            &mut cmds,
        );
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftAllowMultiToggled(true),
            &mut cmds,
        );
        let _ = update(&mut model, ExtraFieldsMsg::DraftAddOption, &mut cmds);
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftOptionChanged {
                index: 2,
                value: "C".into(),
            },
            &mut cmds,
        );
        let _ = update(&mut model, ExtraFieldsMsg::CommitFieldModal, &mut cmds);

        let f = &model.fields[0];
        assert_eq!(f.label, "New");
        assert_eq!(f.description.as_deref(), Some("ndesc"));
        assert!(f.allow_multi_values);
        assert!(f.options.contains(&"C".into()));
    }

    #[test]
    fn draft_kind_change_only_affects_creation() {
        // Creation path
        let mut model = ExtraFieldsModel::default();
        let mut cmds = Vec::new();
        let _ = update(
            &mut model,
            ExtraFieldsMsg::StartAddField { group_id: None },
            &mut cmds,
        );
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftKindChanged(ExtraFieldKind::Select),
            &mut cmds,
        );
        let _ = update(&mut model, ExtraFieldsMsg::DraftAddOption, &mut cmds);
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftOptionChanged {
                index: 0,
                value: "One".into(),
            },
            &mut cmds,
        );
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftLabelChanged("New field".into()),
            &mut cmds,
        );
        let _ = update(&mut model, ExtraFieldsMsg::CommitFieldModal, &mut cmds);
        assert_eq!(model.fields.len(), 1);
        assert_eq!(model.fields[0].kind, ExtraFieldKind::Select);
        assert_eq!(model.fields[0].options, vec!["One".to_string()]);

        // Edit path should ignore kind change
        let _ = update(&mut model, ExtraFieldsMsg::OpenFieldModal(0), &mut cmds);
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftKindChanged(ExtraFieldKind::Text),
            &mut cmds,
        );
        let _ = update(&mut model, ExtraFieldsMsg::CommitFieldModal, &mut cmds);
        assert_eq!(model.fields[0].kind, ExtraFieldKind::Select);
    }

    #[test]
    fn modal_cancel_keeps_field() {
        let mut model = ExtraFieldsModel::default();
        model.fields.push(ExtraField {
            label: "Old".into(),
            kind: ExtraFieldKind::Number,
            value: "1".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: Some("m".into()),
            units: vec!["m".into()],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        });
        let mut cmds = Vec::new();
        let _ = update(&mut model, ExtraFieldsMsg::OpenFieldModal(0), &mut cmds);
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftLabelChanged("New".into()),
            &mut cmds,
        );
        let _ = update(&mut model, ExtraFieldsMsg::CloseFieldModal, &mut cmds);

        assert_eq!(model.fields[0].label, "Old");
    }

    #[test]
    fn duplicate_name_blocks_new_field() {
        let mut model = ExtraFieldsModel::default();
        model.fields.push(ExtraField {
            label: "Name".into(),
            kind: ExtraFieldKind::Text,
            value: "".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        });
        let mut cmds = Vec::new();
        let _ = update(
            &mut model,
            ExtraFieldsMsg::StartAddField { group_id: None },
            &mut cmds,
        );
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftLabelChanged("Name".into()),
            &mut cmds,
        );
        let event = update(&mut model, ExtraFieldsMsg::CommitFieldModal, &mut cmds).unwrap();

        assert!(event.is_error);
        assert_eq!(model.fields.len(), 1);
        assert!(model.modal_open); // still open for correction
    }

    #[test]
    fn duplicate_name_blocks_edit() {
        let mut model = ExtraFieldsModel::default();
        model.fields.push(ExtraField {
            label: "First".into(),
            kind: ExtraFieldKind::Text,
            value: "".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        });
        model.fields.push(ExtraField {
            label: "Second".into(),
            kind: ExtraFieldKind::Text,
            value: "".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        });

        let mut cmds = Vec::new();
        let _ = update(&mut model, ExtraFieldsMsg::OpenFieldModal(1), &mut cmds);
        let _ = update(
            &mut model,
            ExtraFieldsMsg::DraftLabelChanged("First".into()),
            &mut cmds,
        );
        let event = update(&mut model, ExtraFieldsMsg::CommitFieldModal, &mut cmds).unwrap();

        assert!(event.is_error);
        assert_eq!(model.fields[1].label, "Second"); // unchanged
        assert!(model.modal_open);
    }

    #[test]
    fn remove_group_clears_field_group_ids() {
        let mut model = ExtraFieldsModel::default();
        model.groups.push(ExtraFieldGroup {
            id: 1,
            name: "G1".into(),
            position: 0,
        });
        model.fields.push(ExtraField {
            label: "F".into(),
            kind: ExtraFieldKind::Text,
            value: "v".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: Some(1),
            readonly: false,
        });

        let mut cmds = Vec::new();
        let _ = update(&mut model, ExtraFieldsMsg::RemoveGroup(0), &mut cmds);

        assert_eq!(model.groups.len(), 1);
        assert_eq!(model.groups[0].name, "Default");
        assert_eq!(model.fields[0].group_id, Some(model.groups[0].id));
    }

    /// Ensures that removing the only existing group renames it to "Default" and reassigns its fields to that group.
    ///
    /// This test constructs a model with a single group and a field assigned to that group, invokes the remove-group
    /// action, and asserts that one group remains named "Default" and that the field's `group_id` points to that group.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut model = ExtraFieldsModel::default();
    /// model.groups.push(ExtraFieldGroup { id: 5, name: "Only".into(), position: 0 });
    /// model.fields.push(ExtraField {
    ///     label: "F".into(),
    ///     kind: ExtraFieldKind::Text,
    ///     value: "v".into(),
    ///     value_multi: Vec::new(),
    ///     options: vec![],
    ///     unit: None,
    ///     units: vec![],
    ///     position: None,
    ///     required: false,
    ///     description: None,
    ///     allow_multi_values: false,
    ///     blank_value_on_duplicate: false,
    ///     group_id: Some(5),
    ///     readonly: false,
    /// });
    ///
    /// let mut cmds = Vec::new();
    /// let _ = update(&mut model, ExtraFieldsMsg::RemoveGroup(0), &mut cmds);
    ///
    /// assert_eq!(model.groups.len(), 1);
    /// assert_eq!(model.groups[0].name, "Default");
    /// assert_eq!(model.fields[0].group_id, Some(model.groups[0].id));
    /// ```
    #[test]
    fn removing_last_group_recreates_default_and_reassigns() {
        let mut model = ExtraFieldsModel::default();
        model.groups.push(ExtraFieldGroup {
            id: 5,
            name: "Only".into(),
            position: 0,
        });
        model.fields.push(ExtraField {
            label: "F".into(),
            kind: ExtraFieldKind::Text,
            value: "v".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: Some(5),
            readonly: false,
        });

        let mut cmds = Vec::new();
        let _ = update(&mut model, ExtraFieldsMsg::RemoveGroup(0), &mut cmds);

        assert_eq!(model.groups.len(), 1);
        assert_eq!(model.groups[0].name, "Default");
        assert_eq!(model.fields[0].group_id, Some(model.groups[0].id));
    }

    #[test]
    fn removing_only_group_renames_to_default() {
        let mut model = ExtraFieldsModel::default();
        model.groups.push(ExtraFieldGroup {
            id: 7,
            name: "Solo".into(),
            position: 0,
        });
        model.fields.push(ExtraField {
            label: "F".into(),
            kind: ExtraFieldKind::Text,
            value: "v".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: Some(7),
            readonly: false,
        });

        let mut cmds = Vec::new();
        let _ = update(&mut model, ExtraFieldsMsg::RemoveGroup(0), &mut cmds);

        assert_eq!(model.groups.len(), 1);
        assert_eq!(model.groups[0].name, "Default");
        assert_eq!(model.fields[0].group_id, Some(7));
    }

    #[test]
    fn removing_group_moves_fields_to_default() {
        let mut model = ExtraFieldsModel::default();
        model.groups.push(ExtraFieldGroup {
            id: 1,
            name: "G1".into(),
            position: 0,
        });
        model.groups.push(ExtraFieldGroup {
            id: 2,
            name: "G2".into(),
            position: 1,
        });
        model.fields.push(ExtraField {
            label: "F".into(),
            kind: ExtraFieldKind::Text,
            value: "v".into(),
            value_multi: Vec::new(),
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: Some(2),
            readonly: false,
        });

        let mut cmds = Vec::new();
        let _ = update(&mut model, ExtraFieldsMsg::RemoveGroup(1), &mut cmds);

        assert!(model.groups.iter().any(|g| g.name == "Default"));
        let default_id = model
            .groups
            .iter()
            .find(|g| g.name == "Default")
            .unwrap()
            .id;
        assert_eq!(model.fields[0].group_id, Some(default_id));
    }

    #[test]
    fn remove_field_updates_model() {
        let mut model = ExtraFieldsModel::default();
        let mut cmds = Vec::new();
        update(
            &mut model,
            ExtraFieldsMsg::ImportLoaded {
                fields: vec![ExtraField {
                    label: "One".into(),
                    kind: ExtraFieldKind::Text,
                    value: "a".into(),
                    value_multi: Vec::new(),
                    options: vec![],
                    unit: None,
                    units: vec![],
                    position: None,
                    required: false,
                    description: None,
                    allow_multi_values: false,
                    blank_value_on_duplicate: false,
                    group_id: None,
                    readonly: false,
                }],
                groups: vec![],
                source: PathBuf::from("sample.json"),
            },
            &mut cmds,
        );

        assert_eq!(model.fields.len(), 1);
        let _ = update(&mut model, ExtraFieldsMsg::RemoveField(0), &mut Vec::new());
        assert!(model.fields.is_empty());
    }
}