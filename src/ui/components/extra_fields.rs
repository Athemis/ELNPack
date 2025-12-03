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
    readonly: bool,
    allow_multi_values: bool,
    options: Vec<String>,
    units: Vec<String>,
    unit: String,
    kind: ExtraFieldKind,
    group_id: Option<i32>,
}

impl Default for FieldDraft {
    /// Creates a new `FieldDraft` populated with empty/default values.
    ///
    /// The returned draft has an empty `label` and `description`, `required` and
    /// `allow_multi_values` set to `false`, no `options` or `units`, the default
    /// kind `ExtraFieldKind::Text`, and no group assignment.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = FieldDraft::default();
    /// assert!(d.label.is_empty());
    /// assert!(d.description.is_empty());
    /// assert_eq!(d.required, false);
    /// assert_eq!(d.readonly, false);
    /// assert_eq!(d.allow_multi_values, false);
    /// assert!(d.options.is_empty());
    /// assert!(d.units.is_empty());
    /// assert_eq!(d.kind, ExtraFieldKind::Text);
    /// assert!(d.group_id.is_none());
    /// ```
    fn default() -> Self {
        Self {
            label: String::new(),
            description: String::new(),
            required: false,
            readonly: false,
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
    /// Access the model's extra fields as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given an existing `model: ExtraFieldsModel`:
    /// // let slice = model.fields();
    /// // assert!(slice.is_empty() || slice[0].label.len() > 0);
    /// ```
    pub fn fields(&self) -> &[ExtraField] {
        &self.fields
    }

    /// Returns a slice of all field groups in the model.
    ///
    /// The returned slice borrows the model's internal group storage and can be used for read-only iteration.
    ///
    /// # Examples
    ///
    /// ```
    /// let model = ExtraFieldsModel::default();
    /// let groups: &[ExtraFieldGroup] = model.groups();
    /// // iterate without taking ownership
    /// for g in groups {
    ///     println!("{}", g.name);
    /// }
    /// ```
    pub fn groups(&self) -> &[ExtraFieldGroup] {
        &self.groups
    }

    /// Returns whether any extra field in the model is invalid.
    ///
    /// # Returns
    ///
    /// `true` if at least one field is invalid, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let model = ExtraFieldsModel {
    ///     fields: Vec::new(),
    ///     groups: Vec::new(),
    ///     editing_group: None,
    ///     editing_group_buffer: String::new(),
    ///     editing_field: None,
    ///     modal_open: false,
    ///     modal_draft: None,
    /// };
    /// assert!(!model.has_invalid_fields());
    /// ```
    pub fn has_invalid_fields(&self) -> bool {
        self.fields.iter().any(field_invalid)
    }

    /// Ensure a group named "Default" exists in the model and return its id.
    ///
    /// If a "Default" group already exists, returns its id. Otherwise a new group named
    /// "Default" is created with a new id and appended to `self.groups`, and that id is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut model = ExtraFieldsModel::default();
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

    /// Return the id of the group with the lowest position, creating a default group if none exist.
    ///
    /// If multiple groups share the same lowest position, the group with the smallest id is chosen.
    /// If the model has no groups, a Default group is created and its id is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `ExtraFieldsModel` has a public `new()` constructor and public `groups` field.
    /// let mut model = ExtraFieldsModel::new();
    /// // No groups yet: this will create the Default group and return its id.
    /// let default_id = model.lowest_position_group_id();
    /// assert!(model.groups.iter().any(|g| g.id == default_id));
    ///
    /// // When multiple groups exist, returns the one with the lowest position (tie-broken by id).
    /// model.groups.push(ExtraFieldGroup { id: 10, position: 1, name: "A".into() });
    /// model.groups.push(ExtraFieldGroup { id: 5, position: 1, name: "B".into() });
    /// let chosen = model.lowest_position_group_id();
    /// assert_eq!(chosen, 5);
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

    /// Return the name of the group with the given id, or "Default" when `group_id` is `None` or no matching group exists.
    ///
    /// # Examples
    ///
    /// ```
    /// let model = ExtraFieldsModel {
    ///     fields: vec![],
    ///     groups: vec![ExtraFieldGroup { id: 1, name: "Specs".into(), position: 0 }],
    ///     editing_group: None,
    ///     editing_group_buffer: String::new(),
    ///     editing_field: None,
    ///     modal_open: false,
    ///     modal_draft: None,
    /// };
    ///
    /// assert_eq!(model.display_group_name(Some(1)), "Specs");
    /// assert_eq!(model.display_group_name(None), "Default");
    /// assert_eq!(model.display_group_name(Some(42)), "Default");
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
    DraftReadonlyToggled(bool),
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

/// Update the model based on a message.
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
            model.modal_open = false;
            model.modal_draft = None;
            model.editing_field = None;
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
                    readonly: f.readonly,
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
        ExtraFieldsMsg::DraftReadonlyToggled(val) => {
            if let Some(d) = model.modal_draft.as_mut() {
                d.readonly = val;
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

/// Render the extra-fields editor UI and collect user actions as messages.
///
/// This draws the collapsible "Metadata" section, action buttons, the grouped field list,
/// and any open field-edit modal, returning a list of messages for actions the user took
/// during this render pass.
///
/// # Returns
///
/// A `Vec<ExtraFieldsMsg>` containing messages that represent user actions triggered while rendering.
///
/// # Examples
///
/// ```no_run
/// use egui::Context;
/// // Create a UI context and model (in a real app these come from your app state)
/// let ctx = Context::default();
/// let mut model = crate::ui::components::extra_fields::ExtraFieldsModel::default();
/// // Render once to collect messages (in a real app this happens inside your frame)
/// let mut ui = ctx.begin_frame(Default::default());
/// let msgs = crate::ui::components::extra_fields::view(&mut ui, &model);
/// // msgs now contains any actions the user performed during the render
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

/// Render the list of extra fields grouped into collapsible group panels and collect any emitted UI messages.
///
/// Renders each group in `model.groups` as a collapsible header containing its fields; when there are
/// no groups and no fields a muted placeholder label is shown. User actions in the rendered controls
/// (for example "Add field to <group>") are pushed into `msgs`.
///
/// # Examples
///
/// ```no_run
/// use egui;
/// let ctx = egui::Context::default();
/// let mut model = ExtraFieldsModel::default();
/// let mut msgs = Vec::new();
///
/// egui::CentralPanel::default().show(&ctx, |ui| {
///     render_fields(ui, &model, &mut msgs);
/// });
///
/// // No interaction in this example, so no messages were produced.
/// assert!(msgs.is_empty());
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

/// Render the header controls for a single extra-field group (rename, remove, or edit).
///
/// Displays either an inline editing row (cancel and commit buttons plus a single-line
/// text edit bound to the model's editing buffer) when that group is currently being
/// edited, or action buttons to rename or remove the group when not editing. User
/// interactions append corresponding `ExtraFieldsMsg` entries to the provided `msgs`
/// vector:
/// - CancelGroupEdit, CommitGroupName, EditGroupName when editing;
/// - RemoveGroup(idx) or StartEditGroup(idx) when not editing (removal only shown if
///   more than one group exists).
///
/// The function reads `model.editing_group` and `model.editing_group_buffer` and uses
/// the group's `id` to find the group's index when emitting messages that require it.
///
/// # Examples
///
/// ```no_run
/// # use egui;
/// # use my_crate::ui::components::extra_fields::{ExtraFieldGroup, ExtraFieldsModel, ExtraFieldsMsg, render_group_header};
/// # // The following is a non-executable sketch showing typical usage:
/// # let mut ui: egui::Ui = unimplemented!();
/// # let group = ExtraFieldGroup { id: 1, name: "Default".into(), position: 0 };
/// # let model = ExtraFieldsModel::default();
/// let mut msgs = Vec::new();
/// // render_group_header(&mut ui, &group, &mut msgs, &model);
/// // After calling, `msgs` may contain messages produced by user interaction.
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

/// Render a single extra-field card including its label, description, controls (edit/remove)
/// and the appropriate value editor for the field's kind.
///
/// The card is visually highlighted when the field is invalid. Clicking the trash or pencil
/// buttons pushes `ExtraFieldsMsg::RemoveField` or `ExtraFieldsMsg::OpenFieldModal` (with
/// the provided `idx`) onto the supplied `msgs` vector; other interactions push their
/// corresponding messages as handled by the value renderer.
///
/// # Examples
///
/// ```
/// use egui::{CtxRef, CentralPanel};
/// // In an actual egui app you would call this from within a UI callback:
/// // let ctx: &egui::CtxRef = ...;
/// // CentralPanel::default().show(ctx, |ui| {
/// //     let field = ExtraField::default(); // construct a test field
/// //     let mut msgs = Vec::new();
/// //     render_field(ui, &field, 0, &mut msgs);
/// // });
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

/// Renders the appropriate input widget for a field based on its kind inside a framed group.
///
/// The widget emitted depends on the field's `kind`:
/// - `Checkbox` renders a checkbox control.
/// - `Select` and `Radio` render option controls.
/// - `Number` renders a numeric input (and unit selector when applicable).
/// - All other kinds render a text input.
///
/// The function emits user interactions as `ExtraFieldsMsg` entries pushed into `msgs`.
///
/// # Examples
///
/// ```
/// // Note: this example is illustrative; constructing a real `egui::Ui` requires an egui context.
/// // let mut ui: egui::Ui = ...;
/// // let field = ExtraField::default();
/// // let mut msgs = Vec::new();
/// // render_field_value(&mut ui, &field, 0, &mut msgs);
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

/// Renders a checkbox for a boolean extra field and queues a ToggleCheckbox message when its state changes.
///
/// The checkbox reflects the field's `value` being `"on"` and, when toggled by the user, appends
/// `ExtraFieldsMsg::ToggleCheckbox { index, checked }` to `msgs`.
///
/// # Examples
///
/// ```no_run
/// use crate::ui::components::extra_fields::{ExtraField, ExtraFieldKind, ExtraFieldsMsg};
/// use egui::Ui;
///
/// // Inside an egui UI callback:
/// let field = ExtraField {
///     label: "Enabled".into(),
///     value: "on".into(),
///     kind: ExtraFieldKind::Checkbox,
///     ..Default::default()
/// };
/// let mut msgs = Vec::new();
/// // render_checkbox(&mut ui, &field, 0, &mut msgs);
/// // If the user toggles the checkbox, msgs will receive a ToggleCheckbox message.
/// ```
fn render_checkbox(
    ui: &mut egui::Ui,
    field: &ExtraField,
    idx: usize,
    msgs: &mut Vec<ExtraFieldsMsg>,
) {
    let mut checked = field.value == "on";
    ui.add_enabled_ui(!field.readonly, |ui| {
        if ui.checkbox(&mut checked, "Checked").changed() {
            msgs.push(ExtraFieldsMsg::ToggleCheckbox {
                index: idx,
                checked,
            });
        }
    });
}

/// Renders selectable options for a select/radio field and emits messages when the selection changes.
///
/// When `field.allow_multi_values` is true this renders a list of checkboxes and emits
/// `ExtraFieldsMsg::UpdateMulti` with the selected values; otherwise it renders radio buttons
/// and emits `ExtraFieldsMsg::EditValue` for the chosen option.
///
/// # Examples
///
/// ```
/// // inside an egui UI callback:
/// // let mut msgs = Vec::new();
/// // render_options(ui, &field, 0, &mut msgs);
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
        ui.add_enabled_ui(!field.readonly, |ui| {
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
        });
    } else {
        let mut current = field.value.clone();
        ui.add_enabled_ui(!field.readonly, |ui| {
            for opt in &field.options {
                if ui.radio_value(&mut current, opt.clone(), opt).clicked() {
                    msgs.push(ExtraFieldsMsg::EditValue {
                        index: idx,
                        value: opt.clone(),
                    });
                }
            }
        });
    }
}

/// Renders a numeric text input for an extra field and, if present, a unit selector.
///
/// The input is disabled when the field is read-only. User edits emit `ExtraFieldsMsg::EditValue`,
/// and selecting a unit emits `ExtraFieldsMsg::SelectUnit`.
///
/// # Examples
///
/// ```no_run
/// // inside an egui UI callback:
/// // render_number(ui, &field, idx, &mut msgs);
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
            ui.add_enabled_ui(!disabled, |ui| {
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
            });
        }
    });
}

/// Renders a single-line text input for an ExtraField and emits an `EditValue` message when the user edits the value.
///
/// The input is rendered disabled when the field is readonly.
///
/// # Examples
///
/// ```no_run
/// // Within an egui UI callback:
/// // render_text_input(&mut ui, &field, idx, &mut msgs);
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

/// Provides a short placeholder hint string for the given field kind.
///
/// The hint is intended for use as an input placeholder or example value (e.g., date format, URL, email, numeric ID).
///
/// # Examples
///
/// ```
/// let hint = field_hint(&ExtraFieldKind::Date);
/// assert_eq!(hint, "YYYY-MM-DD");
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
/// The returned string is the display name used in the UI for the given kind.
///
/// # Examples
///
/// ```
/// use crate::ExtraFieldKind;
/// assert_eq!(super::kind_label(&ExtraFieldKind::Text), "Text");
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

/// List all supported extra-field kinds in a stable, display-friendly order.
///
/// The returned vector enumerates every `ExtraFieldKind` the UI supports; the order is stable
/// and used for consistent presentation in selection widgets.
///
/// # Examples
///
/// ```
/// let kinds = all_kinds();
/// assert!(!kinds.is_empty());
/// assert_eq!(kinds[0], ExtraFieldKind::Text);
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

/// Split a comma-separated string into trimmed, non-empty tokens.
///
/// Empty items and surrounding whitespace are discarded; each token is returned as an owned `String`.
///
/// # Examples
///
/// ```
/// let v = split_multi(" a, b, ,c ");
/// assert_eq!(v, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
/// ```
fn split_multi(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Determine if a label conflicts with any existing field label in the model, ignoring case and surrounding whitespace, optionally excluding a field at the given index (useful while editing).
///
/// The check is case-insensitive and treats labels that are empty after trimming as non-conflicting.
///
/// # Examples
///
/// ```
/// let mut model = ExtraFieldsModel::default();
/// // assume ExtraField implements Default and has a `label` field
/// model.fields.push(ExtraField { label: "Email".into(), ..Default::default() });
///
/// // New label "email" conflicts with existing "Email"
/// assert!(name_conflict(&model, "email", None));
///
/// // When editing the existing field, exclude its index to avoid a self-conflict
/// assert!(!name_conflict(&model, "email", Some(0)));
/// ```
fn name_conflict(model: &ExtraFieldsModel, label: &str, editing: Option<usize>) -> bool {
    let key = label.trim();
    if key.is_empty() {
        return false;
    }
    model.fields.iter().enumerate().any(|(idx, f)| {
        idx != editing.unwrap_or(usize::MAX) && f.label.trim().eq_ignore_ascii_case(key)
    })
}

/// Determines whether a metadata field fails validation.
///
/// # Returns
///
/// `true` if the field fails validation, `false` otherwise.
///
/// # Examples
///
/// ```
/// let f = ExtraField::default();
/// assert_eq!(field_invalid(&f), validate_field(&f).is_some());
/// ```
fn field_invalid(field: &ExtraField) -> bool {
    validate_field(field).is_some()
}

/// Returns the trimmed input as `Some(String)` or `None` when the trimmed string is empty.
///
/// # Examples
///
/// ```
/// assert_eq!(trimmed_or_none("  foo  "), Some(String::from("foo")));
/// assert_eq!(trimmed_or_none("   "), None);
/// assert_eq!(trimmed_or_none("bar"), Some(String::from("bar")));
/// ```
fn trimmed_or_none(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Applies a FieldDraft to an existing ExtraField, updating the field's editable properties.
///
/// The draft's trimmed non-empty `label` replaces the field label; `description` is trimmed and
/// stored as `Some` or `None`; `required`, `allow_multi_values`, and `group_id` are applied
/// directly. When the field kind is `Select` or `Radio`, `options` are copied from the draft.
/// When the field kind is `Number`, `units` and the trimmed default `unit` are copied from the draft.
///
/// # Examples
///
/// ```
/// let mut field = ExtraField {
///     label: "Old".into(),
///     description: Some("old".into()),
///     required: false,
///     allow_multi_values: false,
///     options: vec![],
///     units: vec![],
///     unit: None,
///     kind: ExtraFieldKind::Select,
///     group_id: Some(1),
///     value: String::new(),
///     value_multi: vec![],
/// };
///
/// let draft = FieldDraft {
///     label: " New Label ".into(),
///     description: "  ".into(),
///     required: true,
///     allow_multi_values: true,
///     options: vec!["a".into(), "b".into()],
///     units: vec!["u".into()],
///     unit: "u".into(),
///     kind: ExtraFieldKind::Select,
///     group_id: Some(2),
/// };
///
/// apply_draft_to_field(&draft, &mut field);
///
/// assert_eq!(field.label, "New Label");
/// assert_eq!(field.description, None);
/// assert!(field.required);
/// assert!(field.allow_multi_values);
/// assert_eq!(field.options, vec!["a", "b"]);
/// assert_eq!(field.group_id, Some(2));
/// ```
fn apply_draft_to_field(draft: &FieldDraft, field: &mut ExtraField) {
    let label = draft.label.trim();
    if !label.is_empty() {
        field.label = label.to_string();
    }
    field.description = trimmed_or_none(&draft.description);
    field.required = draft.required;
    field.readonly = draft.readonly;
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

/// Renders and manages the "Edit field" modal for creating or editing an extra field.
///
/// The modal is shown only when the model's `modal_open` is true and a `modal_draft` is present.
/// While open, the modal updates the draft via `ExtraFieldsMsg` entries pushed to `msgs` (e.g.
/// `DraftLabelChanged`, `DraftDescChanged`, `DraftOptionChanged`, `DraftGroupChanged`, etc.)
/// and emits `CommitFieldModal` or `CloseFieldModal` when the user saves or cancels.
/// The save button is enabled only when the draft has a non-empty title and no name conflict.
///
/// # Examples
///
/// ```rust,no_run
/// // Typical usage inside a UI render loop:
/// // let ctx: egui::Context = /* obtained from egui framework */;
/// // let mut model = ExtraFieldsModel::default();
/// // let mut msgs = Vec::new();
/// render_field_modal(&ctx, &model, &mut msgs);
/// // Process `msgs` through the component's update function afterwards.
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
            ui.add_space(4.0);
            let mut readonly = draft.readonly;
            if ui.checkbox(&mut readonly, "Read-only").changed() {
                msgs.push(ExtraFieldsMsg::DraftReadonlyToggled(readonly));
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
    /// The returned field has no value, options, units, or group assigned and is not required or readonly.
    ///
    /// # Examples
    ///
    /// ```
    /// let f = make_field("Temperature", ExtraFieldKind::Number);
    /// assert_eq!(f.label, "Temperature");
    /// assert_eq!(f.kind, ExtraFieldKind::Number);
    /// assert!(f.value.is_empty());
    /// assert!(f.options.is_empty());
    /// assert!(!f.required);
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

    /// Create an ExtraFieldGroup with the given id and name, initializing its position to 0.
    ///
    /// # Examples
    ///
    /// ```
    /// let g = make_group(42, "Measurements");
    /// assert_eq!(g.id, 42);
    /// assert_eq!(g.name, "Measurements");
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

    /// Ensures renaming a field to a label that already exists is rejected and the modal remains open.
    ///
    /// Verifies that committing an edit which would produce a duplicate label emits an error event,
    /// leaves the original field label unchanged, and keeps the edit modal open.
    ///
    /// # Examples
    ///
    /// ```
    /// // Setup: two fields "First" and "Second"
    /// // Open modal for the second field, change its label to "First", and attempt to commit.
    /// // Expected: an error event is returned, the second field's label stays "Second", and modal stays open.
    /// ```
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
