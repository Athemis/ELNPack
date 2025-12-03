// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Extra field definitions imported from eLabFTW metadata JSON.
//! Parsing is kept pure so it can be reused by UI and archive logic.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use email_address::EmailAddress;
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
    /// Creates an `ExtraFieldKind` from an eLabFTW type token.
    ///
    /// Maps known type strings (e.g. `"text"`, `"number"`, `"select"`, `"url"`, `"email"`, `"items"`, `"experiments"`, `"users"`, etc.) to their corresponding `ExtraFieldKind` variants. Unknown tokens are returned as `ExtraFieldKind::Unknown` containing the original string.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::models::extra_fields::ExtraFieldKind;
    ///
    /// assert_eq!(ExtraFieldKind::from_str("text"), ExtraFieldKind::Text);
    /// assert_eq!(ExtraFieldKind::from_str("number"), ExtraFieldKind::Number);
    /// assert_eq!(ExtraFieldKind::from_str("custom-type"), ExtraFieldKind::Unknown("custom-type".to_string()));
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

    /// String form used when emitting metadata.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Select => "select",
            Self::Checkbox => "checkbox",
            Self::Date => "date",
            Self::DateTimeLocal => "datetime-local",
            Self::Time => "time",
            Self::Url => "url",
            Self::Email => "email",
            Self::Radio => "radio",
            Self::Items => "items",
            Self::Experiments => "experiments",
            Self::Users => "users",
            Self::Unknown(raw) => raw.as_str(),
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

/// Validate a single `ExtraField` and return a reason code when the field is invalid.
///
/// Performs required-field checks and kind-specific validations:
/// - `required`: returns `Some("required")` when the trimmed value is empty.
/// - `Url`: returns `Some("invalid_url")` if the non-empty value is not an `http`/`https` URL with a host.
/// - `Number`: returns `Some("invalid_number")` if the non-empty value is not a valid floating-point number.
/// - `Items`, `Experiments`, `Users`: return `Some("invalid_integer")` if the non-empty value is not a valid integer.
/// - `Email`: returns `Some("invalid_email")` if the non-empty value is not a valid email address.
/// For other kinds or when the value is empty (and not required), validation returns `None`.
///
/// # Returns
///
/// `Some(reason_code)` if the field is invalid, `None` if the field is valid or no validation applies.
///
/// # Examples
///
/// ```
/// use crate::models::extra_fields::{ExtraField, ExtraFieldKind, validate_field};
///
/// let valid_number = ExtraField {
///     label: "num".into(),
///     kind: ExtraFieldKind::Number,
///     value: "3.14".into(),
///     value_multi: Vec::new(),
///     options: Vec::new(),
///     unit: None,
///     units: Vec::new(),
///     position: None,
///     required: false,
///     description: None,
///     allow_multi_values: false,
///     blank_value_on_duplicate: false,
///     group_id: None,
///     readonly: false,
/// };
/// assert_eq!(validate_field(&valid_number), None);
///
/// let missing_required = ExtraField { required: true, value: "".into(), ..valid_number.clone() };
/// assert_eq!(validate_field(&missing_required), Some("required"));
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
        ExtraFieldKind::Email => {
            if value.is_empty() {
                return None;
            }
            if EmailAddress::parse_with_options(value, Default::default()).is_ok() {
                None
            } else {
                Some("invalid_email")
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

/// Parse extra field definitions and groups from an eLabFTW metadata JSON string.
///
/// This function deserializes the eLabFTW metadata payload and converts the `extra_fields` map
/// and `elabftw.extra_fields_groups` into a structured `ExtraFieldsImport` containing normalized
/// `ExtraField` entries (with single and multi values, options, units, group association, etc.)
/// and ordered `ExtraFieldGroup` entries. It treats absent or empty shapes as defaults and converts
/// JSON numbers/strings to the appropriate Rust types where possible.
///
/// # Returns
///
/// `ExtraFieldsImport` containing parsed `fields` and `groups` on success; returns an error if the
/// input JSON cannot be parsed as expected.
///
/// # Examples
///
/// ```
/// let json = r#"
/// {
///   "extra_fields": {
///     "Notes": { "type": "text", "value": "sample" }
///   },
///   "elabftw": { "extra_fields_groups": [] }
/// }
/// "#;
/// let parsed = parse_elabftw_extra_fields(json).unwrap();
/// assert_eq!(parsed.fields.len(), 1);
/// assert_eq!(parsed.fields[0].label, "Notes");
/// assert_eq!(parsed.fields[0].value, "sample");
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

/// Convert a JSON `Value` reference into an optional `String` representation.
///
/// - Returns `None` when `val` is `None`.
/// - For `Value::String`, returns a cloned string.
/// - For `Value::Number`, returns the number's string form.
/// - For `Value::Bool`, returns `"on"` for `true` and an empty string for `false`.
/// - For all other JSON value types, returns the value's `to_string()` representation.
///
/// # Examples
///
/// ```
/// use serde_json::Value;
///
/// assert_eq!(super::value_to_string(None), None);
/// assert_eq!(super::value_to_string(Some(&Value::String("hi".into()))), Some("hi".into()));
/// assert_eq!(super::value_to_string(Some(&Value::Number(42.into()))), Some("42".into()));
/// assert_eq!(super::value_to_string(Some(&Value::Bool(true))), Some("on".into()));
/// assert_eq!(super::value_to_string(Some(&Value::Bool(false))), Some("".into()));
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