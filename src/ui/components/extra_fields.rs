// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! UI component for importing and editing eLabFTW extra fields metadata.

use eframe::egui;

use crate::models::extra_fields::{ExtraField, ExtraFieldKind};

/// UI state for imported extra fields.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct ExtraFieldsModel {
    fields: Vec<ExtraField>,
}

impl ExtraFieldsModel {
    pub fn fields(&self) -> &[ExtraField] {
        &self.fields
    }
}

/// Messages produced by the extra fields view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtraFieldsMsg {
    ImportRequested,
    ImportCancelled,
    ImportLoaded {
        fields: Vec<ExtraField>,
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
        ExtraFieldsMsg::ImportLoaded { mut fields, source } => {
            fields.sort_by(|a, b| a.cmp_key().cmp(&b.cmp_key()));
            model.fields = fields;
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

    for (idx, field) in model.fields.iter().enumerate() {
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
            match field.kind {
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
                    let mut current = field.value.clone();
                    egui::ComboBox::from_id_salt(format!("extra-select-{}", idx))
                        .selected_text(if current.is_empty() {
                            "Select"
                        } else {
                            &current
                        })
                        .show_ui(ui, |ui| {
                            for opt in &field.options {
                                if ui
                                    .selectable_value(&mut current, opt.clone(), opt)
                                    .clicked()
                                {
                                    msgs.push(ExtraFieldsMsg::EditValue {
                                        index: idx,
                                        value: opt.clone(),
                                    });
                                }
                            }
                        });
                }
                ExtraFieldKind::Number => {
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
                        let mut current_unit = field.unit.clone().unwrap_or_else(|| "".to_string());
                        egui::ComboBox::from_id_salt(format!("extra-unit-{}", idx))
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
            }
        });
        ui.add_space(6.0);
    }
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
            source: PathBuf::from("sample.json"),
        };

        let event = update(&mut model, msg, &mut cmds).unwrap();
        assert!(cmds.is_empty());
        assert_eq!(model.fields.len(), 1);
        assert!(event.message.contains("Imported 1 field"));
        assert!(!event.is_error);
    }
}
