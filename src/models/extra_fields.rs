// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Extra field definitions imported from eLabFTW metadata JSON.
//! Parsing is kept pure so it can be reused by UI and archive logic.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

/// Supported eLabFTW field kinds we know how to render.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtraFieldKind {
    Text,
    Number,
    Select,
    Checkbox,
    Date,
    DateTimeLocal,
    Time,
    Url,
    Email,
    Radio,
    Items,
    Experiments,
    Users,
    Unknown(String),
}

impl ExtraFieldKind {
    fn from_str(raw: &str) -> Self {
        match raw {
            "text" => Self::Text,
            "number" => Self::Number,
            "select" => Self::Select,
            "checkbox" => Self::Checkbox,
            "date" => Self::Date,
            "datetime-local" => Self::DateTimeLocal,
            "time" => Self::Time,
            "url" => Self::Url,
            "email" => Self::Email,
            "radio" => Self::Radio,
            "items" => Self::Items,
            "experiments" => Self::Experiments,
            "users" => Self::Users,
            other => Self::Unknown(other.to_string()),
        }
    }
}

/// Single extra field definition + value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraField {
    pub label: String,
    pub kind: ExtraFieldKind,
    pub value: String,
    pub options: Vec<String>,
    pub unit: Option<String>,
    pub units: Vec<String>,
    pub position: Option<i32>,
    pub required: bool,
    pub description: Option<String>,
    pub allow_multi_values: bool,
    pub blank_value_on_duplicate: bool,
    pub group_id: Option<i32>,
    pub readonly: bool,
}

impl ExtraField {
    /// Sort helper: position first, then label.
    pub fn cmp_key(&self) -> (i32, &str) {
        (self.position.unwrap_or(i32::MAX), &self.label)
    }
}

#[derive(Debug, Deserialize)]
struct ExtraFieldsEnvelope {
    extra_fields: BTreeMap<String, ExtraFieldRaw>,
}

#[derive(Debug, Deserialize)]
struct ExtraFieldRaw {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    options: Vec<Value>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    units: Vec<Value>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    position: Option<i32>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    allow_multi_values: bool,
    #[serde(default)]
    blank_value_on_duplicate: bool,
    #[serde(default)]
    readonly: bool,
    #[serde(default)]
    group_id: Option<Value>,
}

/// Parse the `extra_fields` object from an eLabFTW metadata JSON string.
pub fn parse_elabftw_extra_fields(json: &str) -> Result<Vec<ExtraField>> {
    let env: ExtraFieldsEnvelope =
        serde_json::from_str(json).context("Failed to parse eLabFTW metadata JSON")?;

    let mut fields = Vec::with_capacity(env.extra_fields.len());

    for (label, raw) in env.extra_fields {
        let kind = ExtraFieldKind::from_str(raw.kind.trim());
        let options = raw
            .options
            .iter()
            .filter_map(|v| value_to_string(Some(v)))
            .collect::<Vec<_>>();
        let units = raw
            .units
            .iter()
            .filter_map(|v| value_to_string(Some(v)))
            .collect::<Vec<_>>();

        let value = raw
            .value
            .as_ref()
            .and_then(|v| value_to_string(Some(v)))
            .unwrap_or_else(String::new);

        let group_id = match raw.group_id.as_ref() {
            Some(Value::Number(n)) => n.as_i64().map(|v| v as i32),
            Some(Value::String(s)) => s.parse::<i32>().ok(),
            _ => None,
        };

        fields.push(ExtraField {
            label,
            kind,
            value,
            options,
            unit: raw.unit.filter(|u| !u.trim().is_empty()),
            units,
            position: raw.position,
            required: raw.required,
            description: raw.description.filter(|d| !d.trim().is_empty()),
            allow_multi_values: raw.allow_multi_values,
            blank_value_on_duplicate: raw.blank_value_on_duplicate,
            group_id,
            readonly: raw.readonly,
        });
    }

    fields.sort_by(|a, b| a.cmp_key().cmp(&b.cmp_key()));
    Ok(fields)
}

fn value_to_string(val: Option<&Value>) -> Option<String> {
    match val? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(if *b { "on" } else { "" }.to_string()),
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_elabftw_extra_fields;

    #[test]
    fn parses_sample_extra_fields() {
        let json = r#"{"extra_fields":{"Model":{"type":"text","value":"Empyrian","position":1,"required":true},"X-Ray Wavelength":{"type":"number","unit":"\u212b","units":["\u212b","nm"],"value":1.540562,"position":8}}}"#;

        let fields = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].label, "Model");
        assert_eq!(fields[0].value, "Empyrian");
        assert!(fields[0].required);
        assert_eq!(fields[1].label, "X-Ray Wavelength");
        assert_eq!(fields[1].unit.as_deref(), Some("\u{212b}"));
        assert_eq!(
            fields[1].units,
            vec!["\u{212b}".to_string(), "nm".to_string()]
        );
        assert_eq!(fields[1].value, "1.540562");
    }
}
