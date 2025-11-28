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
use zip::{CompressionMethod, write::FileOptions};

use crate::models::attachment::{Attachment, assert_unique_sanitized_names};
use crate::utils::{hash_file, sanitize_component};

/// Suggest a safe archive filename from a user-facing title.
///
/// Uses [`sanitize_component`] for the base name and lowercases it, then
/// appends the `.eln` extension. Falls back to `eln_entry.eln` when the
/// sanitized title is empty.
pub fn suggested_archive_name(title: &str) -> String {
    let base = sanitize_component(title).to_ascii_lowercase();
    let final_base = if base.is_empty() { "eln_entry" } else { &base };
    format!("{}.eln", final_base)
}

/// Allowed archive genres for RO-Crate metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveGenre {
    Experiment,
    Resource,
}

/// How to store the main body in the RO-Crate metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyFormat {
    Html,
    Markdown,
}

impl Default for BodyFormat {
    fn default() -> Self {
        BodyFormat::Html
    }
}

impl Default for ArchiveGenre {
    fn default() -> Self {
        ArchiveGenre::Experiment
    }
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
pub fn build_and_write_archive(
    output: &Path,
    title: &str,
    body: &str,
    attachments: &[Attachment],
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

    use super::ArchiveGenre;
    use super::BodyFormat;
    use super::build_and_write_archive;
    use super::ensure_extension;
    use super::markdown_to_html;
    use super::suggested_archive_name;
    use crate::models::attachment::Attachment;
    use time::OffsetDateTime;

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
