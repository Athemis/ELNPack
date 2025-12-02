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
    /// Maps an eLabFTW field type identifier string to the corresponding `ExtraFieldKind`.
    ///
    /// Unknown identifiers are captured in the `Unknown` variant containing the original string.
    ///
    /// # Examples
    ///
    /// ```
    /// let k = ExtraFieldKind::from_str("number");
    /// assert!(matches!(k, ExtraFieldKind::Number));
    ///
    /// let u = ExtraFieldKind::from_str("custom-type");
    /// match u {
    ///     ExtraFieldKind::Unknown(s) => assert_eq!(s, "custom-type"),
    ///     _ => panic!("expected Unknown variant"),
    /// }
    /// ```
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
    /// Build a sort key that orders by position (missing positions sort last) and then by label.
    ///
    /// The returned tuple is (position_or_max, label).
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::models::extra_fields::ExtraField;
    ///
    /// let a = ExtraField {
    ///     label: "A".into(),
    ///     kind: crate::models::extra_fields::ExtraFieldKind::Text,
    ///     value: "".into(),
    ///     value_multi: vec![],
    ///     options: vec![],
    ///     unit: None,
    ///     units: vec![],
    ///     position: Some(1),
    ///     required: false,
    ///     description: None,
    ///     allow_multi_values: false,
    ///     blank_value_on_duplicate: false,
    ///     group_id: None,
    ///     readonly: false,
    /// };
    /// let b = ExtraField { position: None, ..a.clone() };
    /// assert_eq!(a.cmp_key(), (1, "A"));
    /// assert_eq!(b.cmp_key().0, std::i32::MAX);
    /// ```
    pub fn cmp_key(&self) -> (i32, &str) {
        (self.position.unwrap_or(i32::MAX), &self.label)
    }
}

/// Validate a single extra field and return a short reason code when it is invalid.
///
/// This performs minimal, pure validation based on the field's kind and required flag:
/// - If the field is required and empty, returns `Some("required")`.
/// - For `Url`: empty values are allowed; otherwise the value must parse as an `http` or `https` URL with a host, otherwise returns `Some("invalid_url")`.
/// - For `Number`: empty values are allowed; otherwise the value must parse as a floating-point number, otherwise returns `Some("invalid_number")`.
/// - For `Items`, `Experiments`, `Users`: empty values are allowed; otherwise the value must parse as a 64-bit integer, otherwise returns `Some("invalid_integer")`.
/// - For all other kinds, no validation error is produced.
///
/// # Returns
///
/// `Some(reason)` when validation fails, where `reason` is one of:
/// - `"required"`
/// - `"invalid_url"`
/// - `"invalid_number"`
/// - `"invalid_integer"`
///
/// Returns `None` when the field is valid.
///
/// # Examples
///
/// ```
/// # use crate::models::extra_fields::{ExtraField, ExtraFieldKind, validate_field};
/// let f = ExtraField {
///     label: "Website".into(),
///     kind: ExtraFieldKind::Url,
///     value: "https://example.com".into(),
///     value_multi: vec![],
///     options: vec![],
///     unit: None,
///     units: vec![],
///     position: None,
///     required: false,
///     description: None,
///     allow_multi_values: false,
///     blank_value_on_duplicate: false,
///     group_id: None,
///     readonly: false,
/// };
/// assert_eq!(validate_field(&f), None);
/// ```
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

/// Parses eLabFTW metadata JSON and extracts extra field definitions and groups.
///
/// Deserializes the provided JSON string and maps the `extra_fields` object and optional
/// `elabftw.extra_fields_groups` into an `ExtraFieldsImport` containing normalized
/// `ExtraField` entries and `ExtraFieldGroup` metadata. Returns a parsing error if the JSON
/// is invalid or cannot be deserialized into the expected structure.
///
/// # Examples
///
/// ```
/// let json = r#"{ "extra_fields": {} }"#;
/// let import = parse_elabftw_extra_fields(json).unwrap();
/// assert!(import.fields.is_empty());
/// assert!(import.groups.is_empty());
/// ```
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

/// Convert a JSON value to a string following the module's serialization rules.
///
/// Maps `Option<&serde_json::Value>` to `Option<String>`:
/// - JSON string -> the inner string
/// - JSON number -> its decimal string representation
/// - JSON boolean -> `"on"` for `true`, `""` for `false`
/// - any other JSON value -> the value's JSON string representation
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// assert_eq!(value_to_string(Some(&json!("text"))), Some("text".to_string()));
/// assert_eq!(value_to_string(Some(&json!(42))), Some("42".to_string()));
/// assert_eq!(value_to_string(Some(&json!(true))), Some("on".to_string()));
/// assert_eq!(value_to_string(None), None);
/// ```
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
}