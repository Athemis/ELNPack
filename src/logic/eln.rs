// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Business logic for building ELN RO-Crate archives.
//!
//! Responsibilities:
//! - Sanitize user-provided names for filesystem safety.
//! - Package experiment content and attachments into a ZIP with RO-Crate metadata.
//! - Provide lightweight helpers for MIME guessing and markdown rendering.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use pulldown_cmark::{Options, Parser, html};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;
use zip::{CompressionMethod, write::FileOptions};

use crate::models::attachment::{Attachment, assert_unique_sanitized_names};
use crate::models::extra_fields::{ExtraField, ExtraFieldGroup};
use crate::utils::{hash_file, sanitize_component};

/// Export-ready packaging of extra fields.
struct ExtraFieldsExport {
    /// PropertyValue nodes for each field.
    property_values: Vec<serde_json::Value>,
    /// PropertyValue node carrying reconstructed eLabFTW metadata JSON.
    metadata_property: serde_json::Value,
    /// List of @id strings to be linked from the experiment variableMeasured.
    variable_measured_ids: Vec<String>,
}

/// Suggest a safe archive filename from a user-facing title.
///
/// Uses [`crate::utils::sanitize_component()`] for the base name and lowercases it, then
/// appends the `.eln` extension. Falls back to `eln_entry.eln` when the
/// sanitized title is empty.
pub fn suggested_archive_name(title: &str) -> String {
    let base = sanitize_component(title).to_ascii_lowercase();
    let final_base = if base.is_empty() { "eln_entry" } else { &base };
    format!("{}.eln", final_base)
}

/// Allowed archive genres for RO-Crate metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ArchiveGenre {
    #[default]
    Experiment,
    Resource,
}

/// How to store the main body in the RO-Crate metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BodyFormat {
    #[default]
    Html,
    Markdown,
}

impl ArchiveGenre {
    fn as_str(&self) -> &'static str {
        match self {
            ArchiveGenre::Experiment => "experiment",
            ArchiveGenre::Resource => "resource",
        }
    }
}

/// Force a specific extension onto a path when it is missing or different.
///
/// Keeps existing matching extension (case-insensitive); otherwise replaces it.
pub fn ensure_extension(mut path: PathBuf, extension: &str) -> PathBuf {
    let replace = !matches!(
        path.extension().and_then(|e| e.to_str()),
        Some(ext) if ext.eq_ignore_ascii_case(extension)
    );

    if replace {
        path.set_extension(extension);
    }
    path
}

/// Build a RO-Crate archive ZIP containing the experiment text, metadata, and attachments.
///
/// Creates directories inside the archive, copies attachments with sanitized names,
/// emits RO-Crate JSON-LD metadata, and writes the final ZIP to `output`.
/// Parent directories for `output` are created if missing.
#[allow(clippy::too_many_arguments)]
pub fn build_and_write_archive(
    output: &Path,
    title: &str,
    body: &str,
    attachments: &[Attachment],
    extra_fields: &[ExtraField],
    extra_groups: &[ExtraFieldGroup],
    performed_at: OffsetDateTime,
    genre: ArchiveGenre,
    keywords: &[String],
    body_format: BodyFormat,
) -> Result<()> {
    // Ensure parent exists so the archive can be written without IO errors.
    if let Some(parent) = output.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {:?}", parent))?;
    }

    // Guard against duplicate archive paths before writing anything.
    assert_unique_sanitized_names(attachments)?;

    let root_folder = sanitize_component(
        output
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("eln-entry"),
    );
    let root_prefix = format!("{}/", root_folder);
    let experiment_dir = format!("{}experiment/", root_prefix);

    let file = File::create(output)
        .with_context(|| format!("Failed to write archive file {:?}", output))?;
    let mut zip = zip::ZipWriter::new(file);
    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.add_directory(&root_prefix, options)
        .context("Failed to create root directory in archive")?;
    zip.add_directory(&experiment_dir, options)
        .context("Failed to create experiment directory in archive")?;

    let mut file_nodes = Vec::new();
    for meta in attachments.iter() {
        // Verify attachment integrity: rehash and compare with stored hash
        if meta.sha256 != "unavailable" {
            let current_hash = hash_file(&meta.path)
                .with_context(|| format!("Failed to rehash attachment {:?}", meta.path))?;

            if current_hash != meta.sha256 {
                anyhow::bail!(
                    "Attachment modified since it was added:\n  {:?}\n  expected sha256 {}\n  found sha256 {}",
                    meta.path,
                    meta.sha256,
                    current_hash,
                );
            }
        }

        let display_name = &meta.sanitized_name;
        let archive_path = format!("{}{}", experiment_dir, display_name);
        let id = format!("./experiment/{}", display_name);

        zip.start_file(&archive_path, options)
            .with_context(|| format!("Failed to add file {} to archive", archive_path))?;

        let mut reader = File::open(&meta.path)
            .with_context(|| format!("Failed to read attachment {:?}", meta.path))?;
        let mut _written = 0u64;
        let mut buffer = [0u8; 8192];
        loop {
            let read = reader
                .read(&mut buffer)
                .with_context(|| format!("Failed to read from {:?}", meta.path))?;
            if read == 0 {
                break;
            }
            zip.write_all(&buffer[..read])
                .with_context(|| format!("Failed to write {} into archive", archive_path))?;
            _written += read as u64;
        }

        let sha256 = meta.sha256.clone();
        let encoding = meta.mime.clone();

        file_nodes.push(serde_json::json!({
            "@id": id,
            "@type": "File",
            "name": display_name,
            "encodingFormat": encoding,
            "contentSize": meta.size.to_string(),
            "sha256": sha256,
        }));
    }

    let timestamp = performed_at
        .format(&Rfc3339)
        .map_err(|err| anyhow::anyhow!("Failed to format performed_at timestamp: {}", err))?;
    let (body_text, encoding_format) = match body_format {
        BodyFormat::Html => (markdown_to_html(body, false), "text/html"),
        BodyFormat::Markdown => (body.to_string(), "text/markdown"),
    };
    let org_id = "https://elnpack.app/#organization";

    let ExtraFieldsExport {
        property_values,
        metadata_property,
        variable_measured_ids,
    } = build_extra_fields_export(extra_fields, extra_groups)?;

    let experiment_node = serde_json::json!({
        "@id": "./experiment/",
        "@type": "Dataset",
        "name": title,
        "encodingFormat": encoding_format,
        "text": body_text,
        "dateCreated": timestamp,
        "dateModified": timestamp,
        "author": { "@id": org_id },
        "genre": genre.as_str(),
        "keywords": keywords,
        "variableMeasured": variable_measured_ids
            .iter()
            .map(|id| serde_json::json!({"@id": id}))
            .collect::<Vec<_>>(),
        "hasPart": file_nodes
            .iter()
            .map(|node| {
                serde_json::json!({"@id": node["@id"].as_str().unwrap_or("./experiment/") })
            })
            .collect::<Vec<_>>(),
    });

    let root_node = serde_json::json!({
        "@id": "./",
        "@type": "Dataset",
        "name": title,
        "hasPart": [ { "@id": "./experiment/" } ],
        "version": 103,
    });

    let metadata_node = serde_json::json!({
        "@id": "ro-crate-metadata.json",
        "@type": "CreativeWork",
        "about": { "@id": "./" },
        "conformsTo": { "@id": "https://w3id.org/ro/crate/1.2" },
        "dateCreated": timestamp,
        "sdPublisher": { "@id": org_id },
    });

    let organization_node = serde_json::json!({
        "@id": org_id,
        "@type": "Organization",
        "name": "elnPack",
        "url": "https://github.com/cbm343e/elnPack",
    });

    let mut graph = vec![metadata_node, root_node, experiment_node, organization_node];
    graph.extend(file_nodes);
    graph.push(metadata_property);
    graph.extend(property_values);

    let metadata = serde_json::json!({
        "@context": "https://w3id.org/ro/crate/1.2/context",
        "@graph": graph,
    });

    zip.start_file(format!("{}ro-crate-metadata.json", root_prefix), options)
        .context("Failed to create metadata file")?;
    let metadata_bytes = serde_json::to_vec_pretty(&metadata)?;
    zip.write_all(&metadata_bytes)
        .context("Failed to write metadata file")?;

    zip.finish().context("Failed to finalize archive")?;
    Ok(())
}

/// Build PropertyValue nodes and reconstructed eLabFTW JSON for extra fields.
fn build_extra_fields_export(
    extra_fields: &[ExtraField],
    extra_groups: &[ExtraFieldGroup],
) -> Result<ExtraFieldsExport> {
    let metadata_json = reconstruct_elabftw_metadata(extra_fields, extra_groups)?;

    let mut property_values = Vec::with_capacity(extra_fields.len() + 1);
    let mut variable_measured_ids = Vec::with_capacity(extra_fields.len() + 1);

    // Emit per-field PropertyValue nodes following eLabFTW style.
    for field in extra_fields {
        let id = format!("pv://{}", Uuid::new_v4());
        variable_measured_ids.push(id.clone());

        let mut node = serde_json::Map::new();
        node.insert("@id".into(), serde_json::Value::String(id.clone()));
        node.insert(
            "@type".into(),
            serde_json::Value::String("PropertyValue".into()),
        );
        node.insert(
            "propertyID".into(),
            serde_json::Value::String(field.label.clone()),
        );
        node.insert(
            "valueReference".into(),
            serde_json::Value::String(field.kind.as_str().to_string()),
        );
        node.insert("value".into(), value_to_json(field, ValueShape::Property));

        if let Some(unit) = &field.unit {
            node.insert("unitText".into(), serde_json::Value::String(unit.clone()));
        }
        if let Some(desc) = &field.description {
            node.insert(
                "description".into(),
                serde_json::Value::String(desc.clone()),
            );
        }
        // Keep node minimal to mirror eLabFTW exports.
        property_values.push(serde_json::Value::Object(node));
    }

    // Add the raw metadata blob for round-trip compatibility.
    let metadata_id = format!("pv://{}", Uuid::new_v4());
    variable_measured_ids.insert(0, metadata_id.clone());
    let metadata_property = serde_json::json!({
        "@id": metadata_id,
        "@type": "PropertyValue",
        "propertyID": "elabftw_metadata",
        "description": "eLabFTW metadata JSON as string",
        "value": metadata_json,
    });

    Ok(ExtraFieldsExport {
        property_values,
        metadata_property,
        variable_measured_ids,
    })
}

/// Produce the eLabFTW-compatible extra_fields JSON from current state.
fn reconstruct_elabftw_metadata(
    extra_fields: &[ExtraField],
    extra_groups: &[ExtraFieldGroup],
) -> Result<String> {
    let groups_json: Vec<serde_json::Value> = extra_groups
        .iter()
        .map(|g| {
            serde_json::json!({
                "id": g.id,
                "name": g.name,
            })
        })
        .collect();

    let mut fields = serde_json::Map::new();
    for field in extra_fields {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "type".into(),
            serde_json::Value::String(field.kind.as_str().to_string()),
        );

        if !field.options.is_empty() {
            obj.insert(
                "options".into(),
                serde_json::Value::Array(
                    field
                        .options
                        .iter()
                        .map(|v| serde_json::Value::String(v.clone()))
                        .collect(),
                ),
            );
        }
        if let Some(unit) = &field.unit {
            obj.insert("unit".into(), serde_json::Value::String(unit.clone()));
        }
        if !field.units.is_empty() {
            obj.insert(
                "units".into(),
                serde_json::Value::Array(
                    field
                        .units
                        .iter()
                        .map(|v| serde_json::Value::String(v.clone()))
                        .collect(),
                ),
            );
        }

        // Value shape matches eLabFTW expectations.
        if field.allow_multi_values && !field.value_multi.is_empty() {
            obj.insert(
                "value".into(),
                serde_json::Value::Array(
                    field
                        .value_multi
                        .iter()
                        .map(|v| serde_json::Value::String(v.clone()))
                        .collect(),
                ),
            );
        } else {
            obj.insert("value".into(), value_to_json(field, ValueShape::Metadata));
        }

        if let Some(position) = field.position {
            obj.insert(
                "position".into(),
                serde_json::Value::Number(position.into()),
            );
        }
        if field.required {
            obj.insert("required".into(), serde_json::Value::Bool(true));
        }
        if let Some(desc) = &field.description {
            obj.insert(
                "description".into(),
                serde_json::Value::String(desc.clone()),
            );
        }
        if field.allow_multi_values {
            obj.insert("allow_multi_values".into(), serde_json::Value::Bool(true));
        }
        if field.blank_value_on_duplicate {
            obj.insert(
                "blank_value_on_duplicate".into(),
                serde_json::Value::Bool(true),
            );
        }
        if let Some(group_id) = field.group_id {
            obj.insert(
                "group_id".into(),
                serde_json::Value::Number(group_id.into()),
            );
        }
        if field.readonly {
            obj.insert("readonly".into(), serde_json::Value::Bool(true));
        }

        fields.insert(field.label.clone(), serde_json::Value::Object(obj));
    }

    let root = serde_json::json!({
        "elabftw": {
            "display_main_text": true,
            "extra_fields_groups": groups_json,
        },
        "extra_fields": serde_json::Value::Object(fields),
    });

    let json = serde_json::to_string(&root)?;
    Ok(json)
}

/// Convert a field's value into the most appropriate JSON type.
enum ValueShape {
    /// Shape used in PropertyValue nodes.
    Property,
    /// Shape used in the embedded eLabFTW metadata JSON.
    Metadata,
}

fn value_to_json(field: &ExtraField, _shape: ValueShape) -> serde_json::Value {
    if field.allow_multi_values && !field.value_multi.is_empty() {
        return serde_json::Value::Array(
            field
                .value_multi
                .iter()
                .map(|v| serde_json::Value::String(v.clone()))
                .collect(),
        );
    }

    match field.kind {
        crate::models::extra_fields::ExtraFieldKind::Number => {
            // eLabFTW exports numbers as strings; keep that for compatibility.
            serde_json::Value::String(field.value.clone())
        }
        crate::models::extra_fields::ExtraFieldKind::Items
        | crate::models::extra_fields::ExtraFieldKind::Experiments
        | crate::models::extra_fields::ExtraFieldKind::Users => {
            // Use integer when possible; fall back to string.
            field
                .value
                .parse::<i64>()
                .map(serde_json::Value::from)
                .unwrap_or_else(|_| serde_json::Value::String(field.value.clone()))
        }
        _ => serde_json::Value::String(field.value.clone()),
    }
}

/// Render markdown to sanitized HTML for embedding in RO-Crate metadata.
///
/// When `parse_math` is true, this enables pulldown-cmark math extensions and
/// preserves KaTeX/MathJax-style span classes so that inline and display math
/// can still be styled by consumers while the HTML is sanitized by Ammonia.
fn markdown_to_html(body: &str, parse_math: bool) -> String {
    let mut builder = ammonia::Builder::default();
    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    if parse_math {
        options.insert(Options::ENABLE_MATH);
        // Allow math-related span classes so sanitized HTML retains enough hooks
        // for inline and display math styling (e.g. KaTeX/MathJax renderers).
        builder.add_allowed_classes("span", &["math", "math-inline", "math-display"]);
    }
    let parser = Parser::new_ext(body, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    builder.clean(&html_output).to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::{fs::File, io::Read};

    use super::ArchiveGenre;
    use super::BodyFormat;
    use super::build_and_write_archive;
    use super::ensure_extension;
    use super::markdown_to_html;
    use super::suggested_archive_name;
    use crate::models::attachment::Attachment;
    use crate::models::extra_fields::{ExtraField, ExtraFieldGroup, ExtraFieldKind};
    use crate::utils::sanitize_component;
    use serde_json::Value;
    use time::OffsetDateTime;
    use zip::ZipArchive;

    #[test]
    fn suggested_archive_name_reuses_sanitizer_and_lowercases() {
        let result = suggested_archive_name("Ångström Study v1");
        assert_eq!(result, "angstrom_study_v1.eln");
    }

    // Should leave an existing matching extension untouched, ignoring case.
    #[test]
    fn ensure_extension_preserves_matching_extension_case_insensitive() {
        let path = PathBuf::from("/tmp/report.ELN");
        let result = ensure_extension(path.clone(), "eln");

        assert_eq!(result, path);
    }

    // Should replace an unmatched extension with the requested one.
    #[test]
    fn ensure_extension_replaces_when_different() {
        let path = PathBuf::from("report.txt");
        let result = ensure_extension(path, "eln");

        assert_eq!(result.extension().and_then(|e| e.to_str()), Some("eln"));
    }

    // Markdown HTML rendering should sanitize scripts while retaining formatting like strikethrough.
    #[test]
    fn markdown_to_html_sanitizes_and_keeps_formatting() {
        let html = markdown_to_html("Hello <script>alert('x')</script> ~~gone~~", false);

        assert!(html.contains("<del>gone</del>"));
        assert!(!html.contains("script"));
    }

    #[test]
    fn markdown_to_html_keeps_math_styles_if_math_parsing_enabled() {
        let html = markdown_to_html("Hello $\\frac{1}{2}$ world $$\\frac{1}{2}$$", true);

        assert!(html.contains("<span class=\"math math-inline\">"));
        assert!(html.contains("<span class=\"math math-display\">"));
    }

    #[test]
    fn markdown_to_html_leaves_math_raw_when_parsing_disabled() {
        let html = markdown_to_html("E = mc$^2$ and $$F=ma$$", false);

        assert!(
            html.contains("E = mc$^2$"),
            "inline math should remain as raw text"
        );
        assert!(
            html.contains("$$F=ma$$"),
            "display math should remain as raw text"
        );
        assert!(
            !html.contains("math-inline") && !html.contains("math-display"),
            "math classes should not be injected when parsing is disabled"
        );
    }

    #[test]
    fn build_and_write_archive_writes_elabftw_style_extra_fields() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("extra.eln");

        let extra_fields = vec![ExtraField {
            label: "Detector".into(),
            kind: ExtraFieldKind::Select,
            value: "Pilatus".into(),
            value_multi: Vec::new(),
            options: vec!["Pilatus".into(), "Eiger".into()],
            unit: Some("model".into()),
            units: Vec::new(),
            position: Some(1),
            required: true,
            description: None,
            allow_multi_values: false,
            blank_value_on_duplicate: false,
            group_id: Some(1),
            readonly: false,
        }];
        let groups = vec![ExtraFieldGroup {
            id: 1,
            name: "General".into(),
            position: 0,
        }];

        build_and_write_archive(
            &out,
            "Title",
            "Body",
            &[],
            &extra_fields,
            &groups,
            OffsetDateTime::from_unix_timestamp(0).unwrap(),
            ArchiveGenre::Experiment,
            &[],
            BodyFormat::Html,
        )
        .unwrap();

        let file = File::open(&out).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();

        let root_folder = sanitize_component(out.file_stem().unwrap().to_str().unwrap());
        let meta_path = format!("{root_folder}/ro-crate-metadata.json");
        let mut meta_file = archive.by_name(&meta_path).unwrap();
        let mut buf = String::new();
        meta_file.read_to_string(&mut buf).unwrap();

        let meta: Value = serde_json::from_str(&buf).unwrap();
        let graph = meta["@graph"].as_array().unwrap();
        let experiment = graph
            .iter()
            .find(|n| n["@id"] == "./experiment/")
            .expect("experiment node present");

        let vars = experiment["variableMeasured"]
            .as_array()
            .expect("variableMeasured array");
        assert_eq!(vars.len(), 2, "field + metadata property present");

        let ids: Vec<_> = vars.iter().filter_map(|v| v["@id"].as_str()).collect();

        // Find the detector property node.
        let detector = graph
            .iter()
            .find(|n| n["propertyID"] == "Detector")
            .expect("detector property node");

        assert_eq!(detector["@type"], "PropertyValue");
        assert_eq!(detector["valueReference"], "select");
        assert_eq!(detector["value"], "Pilatus");
        assert_eq!(detector["unitText"], "model");
        assert!(ids.contains(&detector["@id"].as_str().unwrap()));

        // Metadata blob must be present and contain the field.
        let meta_blob = graph
            .iter()
            .find(|n| n["propertyID"] == "elabftw_metadata")
            .expect("metadata property");
        let raw = meta_blob["value"].as_str().expect("metadata string");
        let parsed: Value = serde_json::from_str(raw).expect("metadata parses");
        let fields = &parsed["extra_fields"];
        assert_eq!(fields["Detector"]["type"], "select");
        assert_eq!(fields["Detector"]["value"], "Pilatus");
    }

    #[test]
    fn build_and_write_archive_rejects_duplicate_sanitized_names() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("test.eln");
        let file1 = tmp.path().join("a.txt");
        let file2 = tmp.path().join("a..txt"); // same sanitized name
        fs::write(&file1, b"one").unwrap();
        fs::write(&file2, b"two").unwrap();

        let attachments = vec![
            Attachment::new(
                file1.clone(),
                "a.txt".into(),
                "text/plain".into(),
                "unavailable".into(),
                3,
            ),
            Attachment::new(
                file2.clone(),
                "a.txt".into(),
                "text/plain".into(),
                "unavailable".into(),
                3,
            ),
        ];

        let result = build_and_write_archive(
            &out,
            "Title",
            "Body",
            &attachments,
            &[],
            &[],
            OffsetDateTime::from_unix_timestamp(0).unwrap(),
            ArchiveGenre::Experiment,
            &[],
            BodyFormat::Html,
        );

        assert!(result.is_err(), "duplicate names should be rejected");
    }

    #[test]
    fn archive_genre_serializes_to_expected_str() {
        assert_eq!(ArchiveGenre::Resource.as_str(), "resource");
        assert_eq!(ArchiveGenre::Experiment.as_str(), "experiment");
    }
}
