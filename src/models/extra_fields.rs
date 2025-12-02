// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Extra field definitions imported from eLabFTW metadata JSON.
//! Parsing is kept pure so it can be reused by UI and archive logic.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

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
    pub value_multi: Vec<String>,
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

/// Pure validation of a single extra field. Returns `Some(reason_code)` when invalid.
pub fn validate_field(field: &ExtraField) -> Option<&'static str> {
    let value = field.value.trim();

    if field.required && value.is_empty() {
        return Some("required");
    }

    match field.kind {
        ExtraFieldKind::Url => {
            if value.is_empty() {
                return None;
            }
            Url::parse(value)
                .ok()
                .filter(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
                .map(|_| None)
                .unwrap_or(Some("invalid_url"))
        }
        ExtraFieldKind::Number => {
            if value.is_empty() {
                return None;
            }
            if value.parse::<f64>().is_ok() {
                None
            } else {
                Some("invalid_number")
            }
        }
        ExtraFieldKind::Items | ExtraFieldKind::Experiments | ExtraFieldKind::Users => {
            if value.is_empty() {
                return None;
            }
            if value.parse::<i64>().is_ok() {
                None
            } else {
                Some("invalid_integer")
            }
        }
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct ExtraFieldsEnvelope {
    extra_fields: BTreeMap<String, ExtraFieldRaw>,
    #[serde(default)]
    elabftw: Option<ElabFtWBlock>,
}

#[derive(Debug, Deserialize, Default)]
struct ElabFtWBlock {
    #[serde(default)]
    extra_fields_groups: Vec<ExtraFieldGroupRaw>,
}

#[derive(Debug, Deserialize)]
struct ExtraFieldGroupRaw {
    id: Value,
    name: String,
}

/// Group information for display ordering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraFieldGroup {
    pub id: i32,
    pub name: String,
    pub position: i32,
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

/// Parsed payload: fields plus optional groups metadata.
pub struct ExtraFieldsImport {
    pub fields: Vec<ExtraField>,
    pub groups: Vec<ExtraFieldGroup>,
}

/// Parse the `extra_fields` object from an eLabFTW metadata JSON string.
pub fn parse_elabftw_extra_fields(json: &str) -> Result<ExtraFieldsImport> {
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

        let (value, value_multi) = match raw.value.as_ref() {
            Some(Value::Array(arr)) => {
                let vals = arr
                    .iter()
                    .filter_map(|v| value_to_string(Some(v)))
                    .collect::<Vec<_>>();
                let joined = vals.join(", ");
                (joined, vals)
            }
            other => (
                other
                    .and_then(|v| value_to_string(Some(v)))
                    .unwrap_or_else(String::new),
                Vec::new(),
            ),
        };

        let group_id = match raw.group_id.as_ref() {
            Some(Value::Number(n)) => n.as_i64().map(|v| v as i32),
            Some(Value::String(s)) => s.parse::<i32>().ok(),
            _ => None,
        };

        fields.push(ExtraField {
            label,
            kind,
            value,
            value_multi,
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
    let groups = env
        .elabftw
        .unwrap_or_default()
        .extra_fields_groups
        .into_iter()
        .enumerate()
        .filter_map(|(idx, g)| match g.id {
            Value::Number(n) => n.as_i64().map(|v| (v as i32, idx as i32, g.name)),
            Value::String(s) => s.parse::<i32>().ok().map(|v| (v, idx as i32, g.name)),
            _ => None,
        })
        .map(|(id, pos, name)| ExtraFieldGroup {
            id,
            name,
            position: pos,
        })
        .collect();

    Ok(ExtraFieldsImport { fields, groups })
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
    use super::*;

    #[test]
    fn parses_sample_extra_fields() {
        let json = r#"{"elabftw":{"extra_fields_groups":[{"id":1,"name":"General"}]},"extra_fields":{"Model":{"type":"text","value":"Empyrian","position":1,"required":true,"group_id":1},"X-Ray Wavelength":{"type":"number","unit":"\u212b","units":["\u212b","nm"],"value":1.540562,"position":8}}}"#;

        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.groups.len(), 1);
        assert_eq!(import.groups[0].name, "General");
        assert_eq!(import.fields.len(), 2);
        assert_eq!(import.fields[0].label, "Model");
        assert_eq!(import.fields[0].value, "Empyrian");
        assert!(import.fields[0].required);
        assert_eq!(import.fields[0].group_id, Some(1));
        assert_eq!(import.fields[1].label, "X-Ray Wavelength");
        assert_eq!(import.fields[1].unit.as_deref(), Some("\u{212b}"));
        assert_eq!(
            import.fields[1].units,
            vec!["\u{212b}".to_string(), "nm".to_string()]
        );
        assert_eq!(import.fields[1].value, "1.540562");
    }

    #[test]
    fn parse_handles_empty_extra_fields() {
        let json = r#"{"extra_fields":{}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields.len(), 0);
        assert_eq!(import.groups.len(), 0);
    }

    #[test]
    fn parse_handles_missing_elabftw_block() {
        let json = r#"{"extra_fields":{"Field1":{"type":"text","value":"test"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields.len(), 1);
        assert_eq!(import.groups.len(), 0);
    }

    #[test]
    fn parse_handles_array_values() {
        let json = r#"{"extra_fields":{"Tags":{"type":"select","value":["tag1","tag2","tag3"],"allow_multi_values":true}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields.len(), 1);
        assert_eq!(import.fields[0].value, "tag1, tag2, tag3");
        assert_eq!(import.fields[0].value_multi, vec!["tag1", "tag2", "tag3"]);
    }

    #[test]
    fn parse_handles_numeric_string_group_id() {
        let json = r#"{"extra_fields":{"Field":{"type":"text","value":"test","group_id":"42"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].group_id, Some(42));
    }

    #[test]
    fn parse_handles_all_field_kinds() {
        let json = r#"{"extra_fields":{"F1":{"type":"text"},"F2":{"type":"number"},"F3":{"type":"select"},"F4":{"type":"checkbox"},"F5":{"type":"date"},"F6":{"type":"datetime-local"},"F7":{"type":"time"},"F8":{"type":"url"},"F9":{"type":"email"},"F10":{"type":"radio"},"F11":{"type":"items"},"F12":{"type":"experiments"},"F13":{"type":"users"},"F14":{"type":"unknown-type"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields.len(), 14);
        assert_eq!(import.fields[0].kind, ExtraFieldKind::Text);
        assert_eq!(import.fields[1].kind, ExtraFieldKind::Number);
        assert_eq!(import.fields[2].kind, ExtraFieldKind::Select);
        assert_eq!(import.fields[3].kind, ExtraFieldKind::Checkbox);
        assert_eq!(import.fields[4].kind, ExtraFieldKind::Date);
        assert_eq!(import.fields[5].kind, ExtraFieldKind::DateTimeLocal);
        assert_eq!(import.fields[6].kind, ExtraFieldKind::Time);
        assert_eq!(import.fields[7].kind, ExtraFieldKind::Url);
        assert_eq!(import.fields[8].kind, ExtraFieldKind::Email);
        assert_eq!(import.fields[9].kind, ExtraFieldKind::Radio);
        assert_eq!(import.fields[10].kind, ExtraFieldKind::Items);
        assert_eq!(import.fields[11].kind, ExtraFieldKind::Experiments);
        assert_eq!(import.fields[12].kind, ExtraFieldKind::Users);
        assert!(matches!(
            import.fields[13].kind,
            ExtraFieldKind::Unknown(ref s) if s == "unknown-type"
        ));
    }

    #[test]
    fn parse_sorts_fields_by_position_then_label() {
        let json = r#"{"extra_fields":{"Z":{"type":"text","position":1},"A":{"type":"text","position":3},"M":{"type":"text","position":1},"B":{"type":"text"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].label, "M");
        assert_eq!(import.fields[1].label, "Z");
        assert_eq!(import.fields[2].label, "A");
        assert_eq!(import.fields[3].label, "B");
    }

    #[test]
    fn parse_filters_empty_unit_and_description() {
        let json = r#"{"extra_fields":{"F1":{"type":"number","unit":"  ","description":""},"F2":{"type":"number","unit":"kg","description":"Weight"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].unit, None);
        assert_eq!(import.fields[0].description, None);
        assert_eq!(import.fields[1].unit, Some("kg".to_string()));
        assert_eq!(import.fields[1].description, Some("Weight".to_string()));
    }

    #[test]
    fn parse_handles_boolean_values() {
        let json = r#"{"extra_fields":{"Enabled":{"type":"checkbox","value":true},"Disabled":{"type":"checkbox","value":false}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].value, "on");
        assert_eq!(import.fields[1].value, "");
    }

    #[test]
    fn parse_rejects_invalid_json() {
        let json = r#"{"extra_fields": invalid}"#;
        assert!(parse_elabftw_extra_fields(json).is_err());
    }

    #[test]
    fn parse_handles_numeric_values_in_options_and_units() {
        let json = r#"{"extra_fields":{"Field":{"type":"select","options":[1,2,3],"units":[10,20]}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].options, vec!["1", "2", "3"]);
        assert_eq!(import.fields[0].units, vec!["10", "20"]);
    }

    #[test]
    fn validate_field_accepts_empty_non_required() {
        let field = ExtraField {
            label: "Optional".into(),
            kind: ExtraFieldKind::Text,
            value: "".into(),
            value_multi: vec![],
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
        };
        assert_eq!(validate_field(&field), None);
    }

    #[test]
    fn validate_field_rejects_empty_required() {
        let field = ExtraField {
            label: "Required".into(),
            kind: ExtraFieldKind::Text,
            value: "   ".into(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: true,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };
        assert_eq!(validate_field(&field), Some("required"));
    }

    #[test]
    fn validate_field_rejects_invalid_urls() {
        let test_cases = vec![
            ("not-a-url", Some("invalid_url")),
            ("htp://example.com", Some("invalid_url")),
            ("ftp://example.com", Some("invalid_url")),
            ("http://", Some("invalid_url")),
            ("https://example.com", None),
            ("http://localhost:8080/path", None),
            ("", None),
        ];

        for (value, expected) in test_cases {
            let field = ExtraField {
                label: "URL".into(),
                kind: ExtraFieldKind::Url,
                value: value.into(),
                value_multi: vec![],
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
            };
            assert_eq!(
                validate_field(&field),
                expected,
                "Failed for URL: {}",
                value
            );
        }
    }

    #[test]
    fn validate_field_rejects_invalid_numbers() {
        let test_cases = vec![
            ("abc", Some("invalid_number")),
            ("12.34.56", Some("invalid_number")),
            ("1e1000", Some("invalid_number")),
            ("42", None),
            ("3.14159", None),
            ("-273.15", None),
            ("1e-10", None),
            ("", None),
        ];

        for (value, expected) in test_cases {
            let field = ExtraField {
                label: "Number".into(),
                kind: ExtraFieldKind::Number,
                value: value.into(),
                value_multi: vec![],
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
            };
            assert_eq!(
                validate_field(&field),
                expected,
                "Failed for number: {}",
                value
            );
        }
    }

    #[test]
    fn validate_field_rejects_invalid_integer_ids() {
        let kinds = vec![
            ExtraFieldKind::Items,
            ExtraFieldKind::Experiments,
            ExtraFieldKind::Users,
        ];

        for kind in kinds {
            for value in ["123", "0", "999999", ""] {
                let field = ExtraField {
                    label: "ID".into(),
                    kind: kind.clone(),
                    value: value.into(),
                    value_multi: vec![],
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
                };
                assert_eq!(validate_field(&field), None, "Should accept: {}", value);
            }

            for value in ["12.5", "abc", "1.0", "-5"] {
                let field = ExtraField {
                    label: "ID".into(),
                    kind: kind.clone(),
                    value: value.into(),
                    value_multi: vec![],
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
                };
                assert_eq!(
                    validate_field(&field),
                    Some("invalid_integer"),
                    "Should reject: {}",
                    value
                );
            }
        }
    }

    #[test]
    fn validate_field_ignores_validation_for_other_types() {
        let kinds = vec![
            ExtraFieldKind::Text,
            ExtraFieldKind::Select,
            ExtraFieldKind::Checkbox,
            ExtraFieldKind::Date,
            ExtraFieldKind::DateTimeLocal,
            ExtraFieldKind::Time,
            ExtraFieldKind::Email,
            ExtraFieldKind::Radio,
        ];

        for kind in kinds {
            let field = ExtraField {
                label: "Field".into(),
                kind,
                value: "any value here".into(),
                value_multi: vec![],
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
            };
            assert_eq!(validate_field(&field), None);
        }
    }

    #[test]
    fn extra_field_cmp_key_orders_by_position_then_label() {
        let field1 = ExtraField {
            label: "Z".into(),
            position: Some(1),
            kind: ExtraFieldKind::Text,
            value: String::new(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };

        let field2 = ExtraField {
            label: "A".into(),
            position: Some(1),
            kind: ExtraFieldKind::Text,
            value: String::new(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };

        let field3 = ExtraField {
            label: "M".into(),
            position: Some(2),
            kind: ExtraFieldKind::Text,
            value: String::new(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };

        let field4 = ExtraField {
            label: "B".into(),
            position: None,
            kind: ExtraFieldKind::Text,
            value: String::new(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };

        assert!(field2.cmp_key() < field1.cmp_key(), "A before Z at same position");
        assert!(field1.cmp_key() < field3.cmp_key(), "position 1 before position 2");
        assert!(field3.cmp_key() < field4.cmp_key(), "positioned before unpositioned");
    }

    #[test]
    fn parse_handles_null_and_missing_values() {
        let json = r#"{"extra_fields":{"F1":{"type":"text","value":null},"F2":{"type":"text"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].value, "");
        assert_eq!(import.fields[1].value, "");
    }

    #[test]
    fn parse_handles_whitespace_in_type() {
        let json = r#"{"extra_fields":{"Field":{"type":"  text  "}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].kind, ExtraFieldKind::Text);
    }

    #[test]
    fn extra_field_kind_from_str_is_case_sensitive() {
        assert_eq!(ExtraFieldKind::from_str("text"), ExtraFieldKind::Text);
        assert!(matches!(
            ExtraFieldKind::from_str("TEXT"),
            ExtraFieldKind::Unknown(_)
        ));
    }

    #[test]
    fn parse_handles_complex_nested_groups() {
        let json = r#"{"elabftw":{"extra_fields_groups":[{"id":"10","name":"Alpha"},{"id":20,"name":"Beta"}]},"extra_fields":{"F1":{"type":"text","group_id":"10"},"F2":{"type":"text","group_id":20}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.groups.len(), 2);
        assert_eq!(import.groups[0].id, 10);
        assert_eq!(import.groups[0].name, "Alpha");
        assert_eq!(import.groups[1].id, 20);
        assert_eq!(import.groups[1].name, "Beta");
        assert_eq!(import.fields[0].group_id, Some(10));
        assert_eq!(import.fields[1].group_id, Some(20));
    }

    #[test]
    fn value_to_string_handles_edge_cases() {
        let json = r#"{"extra_fields":{"Obj":{"type":"text","value":{"nested":"object"}},"Arr":{"type":"text","value":[1,2,3]}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert!(import.fields[0].value.contains("nested"));
        assert_eq!(import.fields[1].value, "1, 2, 3");
    }
}
    #[test]
    fn parse_handles_empty_extra_fields() {
        let json = r#"{"extra_fields":{}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields.len(), 0);
        assert_eq!(import.groups.len(), 0);
    }

    #[test]
    fn parse_handles_missing_elabftw_block() {
        let json = r#"{"extra_fields":{"Field1":{"type":"text","value":"test"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields.len(), 1);
        assert_eq!(import.groups.len(), 0);
    }

    #[test]
    fn parse_handles_array_values() {
        let json = r#"{"extra_fields":{"Tags":{"type":"select","value":["tag1","tag2","tag3"],"allow_multi_values":true}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields.len(), 1);
        assert_eq!(import.fields[0].value, "tag1, tag2, tag3");
        assert_eq!(import.fields[0].value_multi, vec!["tag1", "tag2", "tag3"]);
    }

    #[test]
    fn parse_handles_numeric_string_group_id() {
        let json = r#"{"extra_fields":{"Field":{"type":"text","value":"test","group_id":"42"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].group_id, Some(42));
    }

    #[test]
    fn parse_handles_all_field_kinds() {
        let json = r#"{"extra_fields":{"F1":{"type":"text"},"F2":{"type":"number"},"F3":{"type":"select"},"F4":{"type":"checkbox"},"F5":{"type":"date"},"F6":{"type":"datetime-local"},"F7":{"type":"time"},"F8":{"type":"url"},"F9":{"type":"email"},"F10":{"type":"radio"},"F11":{"type":"items"},"F12":{"type":"experiments"},"F13":{"type":"users"},"F14":{"type":"unknown-type"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields.len(), 14);
        assert_eq!(import.fields[0].kind, ExtraFieldKind::Text);
        assert_eq!(import.fields[1].kind, ExtraFieldKind::Number);
        assert_eq!(import.fields[2].kind, ExtraFieldKind::Select);
        assert_eq!(import.fields[3].kind, ExtraFieldKind::Checkbox);
        assert_eq!(import.fields[4].kind, ExtraFieldKind::Date);
        assert_eq!(import.fields[5].kind, ExtraFieldKind::DateTimeLocal);
        assert_eq!(import.fields[6].kind, ExtraFieldKind::Time);
        assert_eq!(import.fields[7].kind, ExtraFieldKind::Url);
        assert_eq!(import.fields[8].kind, ExtraFieldKind::Email);
        assert_eq!(import.fields[9].kind, ExtraFieldKind::Radio);
        assert_eq!(import.fields[10].kind, ExtraFieldKind::Items);
        assert_eq!(import.fields[11].kind, ExtraFieldKind::Experiments);
        assert_eq!(import.fields[12].kind, ExtraFieldKind::Users);
        assert!(matches!(
            import.fields[13].kind,
            ExtraFieldKind::Unknown(ref s) if s == "unknown-type"
        ));
    }

    #[test]
    fn parse_sorts_fields_by_position_then_label() {
        let json = r#"{"extra_fields":{"Z":{"type":"text","position":1},"A":{"type":"text","position":3},"M":{"type":"text","position":1},"B":{"type":"text"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        // Position 1: M, Z (alphabetical)
        // Position 3: A
        // No position (i32::MAX): B
        assert_eq!(import.fields[0].label, "M");
        assert_eq!(import.fields[1].label, "Z");
        assert_eq!(import.fields[2].label, "A");
        assert_eq!(import.fields[3].label, "B");
    }

    #[test]
    fn parse_filters_empty_unit_and_description() {
        let json = r#"{"extra_fields":{"F1":{"type":"number","unit":"  ","description":""},"F2":{"type":"number","unit":"kg","description":"Weight"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].unit, None);
        assert_eq!(import.fields[0].description, None);
        assert_eq!(import.fields[1].unit, Some("kg".to_string()));
        assert_eq!(import.fields[1].description, Some("Weight".to_string()));
    }

    #[test]
    fn parse_handles_boolean_values() {
        let json = r#"{"extra_fields":{"Enabled":{"type":"checkbox","value":true},"Disabled":{"type":"checkbox","value":false}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].value, "on");
        assert_eq!(import.fields[1].value, "");
    }

    #[test]
    fn parse_rejects_invalid_json() {
        let json = r#"{"extra_fields": invalid}"#;
        assert!(parse_elabftw_extra_fields(json).is_err());
    }

    #[test]
    fn parse_handles_numeric_values_in_options_and_units() {
        let json = r#"{"extra_fields":{"Field":{"type":"select","options":[1,2,3],"units":[10,20]}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].options, vec!["1", "2", "3"]);
        assert_eq!(import.fields[0].units, vec!["10", "20"]);
    }

    #[test]
    fn validate_field_accepts_empty_non_required() {
        let field = ExtraField {
            label: "Optional".into(),
            kind: ExtraFieldKind::Text,
            value: "".into(),
            value_multi: vec![],
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
        };
        assert_eq!(validate_field(&field), None);
    }

    #[test]
    fn validate_field_rejects_empty_required() {
        let field = ExtraField {
            label: "Required".into(),
            kind: ExtraFieldKind::Text,
            value: "   ".into(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            position: None,
            required: true,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };
        assert_eq!(validate_field(&field), Some("required"));
    }

    #[test]
    fn validate_field_rejects_invalid_urls() {
        let test_cases = vec![
            ("not-a-url", Some("invalid_url")),
            ("htp://example.com", Some("invalid_url")),
            ("ftp://example.com", Some("invalid_url")),
            ("http://", Some("invalid_url")),
            ("https://example.com", None),
            ("http://localhost:8080/path", None),
            ("", None), // empty is allowed for non-required
        ];

        for (value, expected) in test_cases {
            let field = ExtraField {
                label: "URL".into(),
                kind: ExtraFieldKind::Url,
                value: value.into(),
                value_multi: vec![],
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
            };
            assert_eq!(
                validate_field(&field),
                expected,
                "Failed for URL: {}",
                value
            );
        }
    }

    #[test]
    fn validate_field_rejects_invalid_numbers() {
        let test_cases = vec![
            ("abc", Some("invalid_number")),
            ("12.34.56", Some("invalid_number")),
            ("1e1000", Some("invalid_number")),
            ("42", None),
            ("3.14159", None),
            ("-273.15", None),
            ("1e-10", None),
            ("", None), // empty is allowed for non-required
        ];

        for (value, expected) in test_cases {
            let field = ExtraField {
                label: "Number".into(),
                kind: ExtraFieldKind::Number,
                value: value.into(),
                value_multi: vec![],
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
            };
            assert_eq!(
                validate_field(&field),
                expected,
                "Failed for number: {}",
                value
            );
        }
    }

    #[test]
    fn validate_field_rejects_invalid_integer_ids() {
        let kinds = vec![
            ExtraFieldKind::Items,
            ExtraFieldKind::Experiments,
            ExtraFieldKind::Users,
        ];

        for kind in kinds {
            // Valid cases
            for value in ["123", "0", "999999", ""] {
                let field = ExtraField {
                    label: "ID".into(),
                    kind: kind.clone(),
                    value: value.into(),
                    value_multi: vec![],
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
                };
                assert_eq!(validate_field(&field), None, "Should accept: {}", value);
            }

            // Invalid cases
            for value in ["12.5", "abc", "1.0", "-5"] {
                let field = ExtraField {
                    label: "ID".into(),
                    kind: kind.clone(),
                    value: value.into(),
                    value_multi: vec![],
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
                };
                assert_eq!(
                    validate_field(&field),
                    Some("invalid_integer"),
                    "Should reject: {}",
                    value
                );
            }
        }
    }

    #[test]
    fn validate_field_ignores_validation_for_other_types() {
        let kinds = vec![
            ExtraFieldKind::Text,
            ExtraFieldKind::Select,
            ExtraFieldKind::Checkbox,
            ExtraFieldKind::Date,
            ExtraFieldKind::DateTimeLocal,
            ExtraFieldKind::Time,
            ExtraFieldKind::Email,
            ExtraFieldKind::Radio,
        ];

        for kind in kinds {
            let field = ExtraField {
                label: "Field".into(),
                kind,
                value: "any value here".into(),
                value_multi: vec![],
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
            };
            assert_eq!(validate_field(&field), None);
        }
    }

    #[test]
    fn extra_field_cmp_key_orders_by_position_then_label() {
        let field1 = ExtraField {
            label: "Z".into(),
            position: Some(1),
            kind: ExtraFieldKind::Text,
            value: String::new(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };

        let field2 = ExtraField {
            label: "A".into(),
            position: Some(1),
            kind: ExtraFieldKind::Text,
            value: String::new(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };

        let field3 = ExtraField {
            label: "M".into(),
            position: Some(2),
            kind: ExtraFieldKind::Text,
            value: String::new(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };

        let field4 = ExtraField {
            label: "B".into(),
            position: None,
            kind: ExtraFieldKind::Text,
            value: String::new(),
            value_multi: vec![],
            options: vec![],
            unit: None,
            units: vec![],
            required: false,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: None,
            readonly: false,
        };

        assert!(field2.cmp_key() < field1.cmp_key(), "A before Z at same position");
        assert!(field1.cmp_key() < field3.cmp_key(), "position 1 before position 2");
        assert!(field3.cmp_key() < field4.cmp_key(), "positioned before unpositioned");
    }

    #[test]
    fn parse_handles_null_and_missing_values() {
        let json = r#"{"extra_fields":{"F1":{"type":"text","value":null},"F2":{"type":"text"}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].value, "");
        assert_eq!(import.fields[1].value, "");
    }

    #[test]
    fn parse_handles_whitespace_in_type() {
        let json = r#"{"extra_fields":{"Field":{"type":"  text  "}}}"#;
        let import = parse_elabftw_extra_fields(json).unwrap();
        assert_eq!(import.fields[0].kind, ExtraFieldKind::Text);
    }

    #[test]
    fn extra_field_kind_from_str_is_case_sensitive() {
        assert_eq!(ExtraFieldKind::from_str("text"), ExtraFieldKind::Text);
        assert!(matches!(
            ExtraFieldKind::from_str("TEXT"),
            ExtraFieldKind::Unknown(_)
        ));
    }
}
