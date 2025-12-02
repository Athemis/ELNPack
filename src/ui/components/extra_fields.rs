// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! UI component for importing and editing eLabFTW extra fields metadata.

use eframe::egui;

use crate::models::extra_fields::{ExtraField, ExtraFieldGroup, ExtraFieldKind};
use crate::ui::components::toggle_switch;

/// UI state for imported extra fields.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct ExtraFieldsModel {
    fields: Vec<ExtraField>,
    groups: Vec<ExtraFieldGroup>,
    editing_group: Option<usize>,
    editing_group_buffer: String,
}

impl ExtraFieldsModel {
    pub fn fields(&self) -> &[ExtraField] {
        &self.fields
    }

    pub fn groups(&self) -> &[ExtraFieldGroup] {
        &self.groups
    }
}

/// Messages produced by the extra fields view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtraFieldsMsg {
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
    ToggleAllowMulti {
        index: usize,
        enabled: bool,
    },
    #[allow(dead_code)]
    RemoveField(usize),
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
        ExtraFieldsMsg::ToggleAllowMulti { index, enabled } => {
            if let Some(field) = model.fields.get_mut(index) {
                field.allow_multi_values = enabled;
                if enabled {
                    if field.value_multi.is_empty() && !field.value.is_empty() {
                        field.value_multi = split_multi(&field.value);
                    }
                } else {
                    if !field.value_multi.is_empty() {
                        field.value = field.value_multi.first().cloned().unwrap_or_default();
                    }
                    field.value_multi.clear();
                }
            }
            None
        }
        ExtraFieldsMsg::RemoveField(index) => {
            if index < model.fields.len() {
                model.fields.remove(index);
            }
            None
        }
        ExtraFieldsMsg::StartEditGroup(idx) => {
            if let Some(g) = model.groups.get(idx) {
                model.editing_group = Some(idx);
                model.editing_group_buffer = g.name.clone();
            }
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
    }
}

/// Render the component and return triggered messages.
pub fn view(ui: &mut egui::Ui, model: &ExtraFieldsModel) -> Vec<ExtraFieldsMsg> {
    let mut msgs = Vec::new();

    egui::CollapsingHeader::new("Metadata (eLabFTW extra fields)")
        .default_open(false)
        .show(ui, |ui| {
            if ui
                .add(egui::Button::new(format!(
                    "{} Import metadata JSON",
                    egui_phosphor::regular::FILE_ARROW_DOWN
                )))
                .clicked()
            {
                msgs.push(ExtraFieldsMsg::ImportRequested);
            }

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

    msgs
}

fn render_fields(ui: &mut egui::Ui, model: &ExtraFieldsModel, msgs: &mut Vec<ExtraFieldsMsg>) {
    if model.fields.is_empty() {
        ui.label(
            egui::RichText::new("No metadata imported yet.")
                .italics()
                .color(egui::Color32::from_gray(110)),
        );
        return;
    }

    // Render grouped fields in group order, then any ungrouped.
    for group in model.groups.iter() {
        let group_fields: Vec<(usize, &ExtraField)> = model
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| f.group_id == Some(group.id))
            .collect();
        if group_fields.is_empty() {
            continue;
        }
        render_group_header(ui, group, msgs, model);
        ui.add_space(4.0);
        for (idx, field) in group_fields {
            render_field(ui, field, idx, msgs);
            ui.add_space(6.0);
        }
        ui.add_space(10.0);
    }

    let ungrouped: Vec<(usize, &ExtraField)> = model
        .fields
        .iter()
        .enumerate()
        .filter(|(_, f)| f.group_id.is_none())
        .collect();

    if !ungrouped.is_empty() {
        ui.heading("Other");
        ui.add_space(4.0);
        for (idx, field) in ungrouped {
            render_field(ui, field, idx, msgs);
            ui.add_space(6.0);
        }
    }
}

fn render_group_header(
    ui: &mut egui::Ui,
    group: &ExtraFieldGroup,
    msgs: &mut Vec<ExtraFieldsMsg>,
    model: &ExtraFieldsModel,
) {
    ui.horizontal(|ui| {
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

        if is_editing {
            let mut text = model.editing_group_buffer.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut text).hint_text("Group name"))
                .changed()
            {
                msgs.push(ExtraFieldsMsg::EditGroupName(text));
            }
            if ui.button(egui_phosphor::regular::CHECK).clicked() {
                msgs.push(ExtraFieldsMsg::CommitGroupName);
            }
            if ui.button(egui_phosphor::regular::X).clicked() {
                msgs.push(ExtraFieldsMsg::CancelGroupEdit);
            }
        } else {
            ui.heading(&group.name);
            if ui
                .button(egui_phosphor::regular::PENCIL_SIMPLE)
                .on_hover_text("Rename group")
                .clicked()
            {
                // find index of this group
                if let Some(idx) = model.groups.iter().position(|g| g.id == group.id) {
                    msgs.push(ExtraFieldsMsg::StartEditGroup(idx));
                }
            }
        }
    });
}

fn render_field(ui: &mut egui::Ui, field: &ExtraField, idx: usize, msgs: &mut Vec<ExtraFieldsMsg>) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            let mut label = field.label.clone();
            if field.required {
                label.push_str(" *");
            }
            ui.label(label);
            if let Some(desc) = &field.description {
                ui.label(
                    egui::RichText::new(desc)
                        .small()
                        .color(egui::Color32::from_gray(120)),
                );
            }
        });

        ui.add_space(4.0);
        ui.group(|ui| match field.kind {
            ExtraFieldKind::Checkbox => {
                let mut checked = field.value == "on";
                if ui.checkbox(&mut checked, "Checked").changed() {
                    msgs.push(ExtraFieldsMsg::ToggleCheckbox {
                        index: idx,
                        checked,
                    });
                }
            }
            ExtraFieldKind::Select | ExtraFieldKind::Radio => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Options")
                            .small()
                            .color(egui::Color32::from_gray(140)),
                    );
                    ui.add_space(6.0);
                    let mut allow_multi = field.allow_multi_values;
                    if toggle_switch(ui, &mut allow_multi).clicked() {
                        msgs.push(ExtraFieldsMsg::ToggleAllowMulti {
                            index: idx,
                            enabled: allow_multi,
                        });
                    }
                    ui.label(
                        egui::RichText::new("Allow multiple")
                            .small()
                            .color(egui::Color32::from_gray(140)),
                    );
                });
                ui.add_space(4.0);

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
            ExtraFieldKind::Number => {
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
            _ => {
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
        });
        ui.add_space(6.0);
    });
}

fn field_hint(kind: &ExtraFieldKind) -> &'static str {
    match kind {
        ExtraFieldKind::Date => "YYYY-MM-DD",
        ExtraFieldKind::DateTimeLocal => "YYYY-MM-DDTHH:MM",
        ExtraFieldKind::Time => "HH:MM",
        ExtraFieldKind::Url => "https://example.com",
        ExtraFieldKind::Email => "name@example.com",
        ExtraFieldKind::Number => "Number",
        _ => "",
    }
}

fn split_multi(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
