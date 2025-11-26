//! Business logic for building ELN RO-Crate archives.
//!
//! Responsibilities:
//! - Sanitize user-provided names for filesystem safety.
//! - Package experiment content and attachments into a ZIP with RO-Crate metadata.
//! - Provide lightweight helpers for MIME guessing and markdown rendering.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use pulldown_cmark::{Options, Parser, html};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use zip::{CompressionMethod, write::FileOptions};

/// Attachment metadata supplied by the UI to avoid recomputing hashes and MIME.
#[derive(Clone, Debug)]
pub struct AttachmentMeta {
    pub path: PathBuf,
    pub mime: String,
    pub sha256: String,
    pub size: u64,
}

/// Suggest a safe archive filename from a user-facing title.
///
/// Non-alphanumeric characters become `_`, sequences are collapsed, and a
/// default of `eln-entry.eln` is returned when the title is empty.
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
    attachments: &[AttachmentMeta],
    performed_at: OffsetDateTime,
    genre: ArchiveGenre,
    keywords: &[String],
) -> Result<()> {
    // Ensure parent exists so the archive can be written without IO errors.
    if let Some(parent) = output.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {:?}", parent))?;
    }

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
    for (idx, meta) in attachments.iter().enumerate() {
        let raw_name = meta
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("attachment-{}.bin", idx + 1));

        let display_name = sanitize_component(&raw_name);
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
    let body_html = markdown_to_html(body);
    let org_id = "https://elnpack.app/#organization";

    let experiment_node = serde_json::json!({
        "@id": "./experiment/",
        "@type": "Dataset",
        "name": title,
        "text": body_html,
        "dateCreated": timestamp,
        "dateModified": timestamp,
        "author": { "@id": org_id },
        "genre": genre.as_str(),
        "keywords": keywords,
        "hasPart": file_nodes.iter().map(|node| {
            serde_json::json!({"@id": node["@id"].as_str().unwrap_or("./experiment/") })
        }).collect::<Vec<_>>(),
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

/// Produce a filesystem-safe path component.
///
/// # Steps
/// - Transliterate Unicode to ASCII with `deunicode` (e.g., "Å" → "A").
/// - Allow ASCII alphanumerics plus `-`, `_`, and `.`; treat other characters as `_`.
/// - Collapse runs of `_` and `.`; trim trailing dots/spaces.
/// - Guard against reserved/empty names and clamp to a safe length.
///
/// This keeps multi-part extensions intact (for example `data.v1.2.tar.gz`
/// stays `data.v1.2.tar.gz`) while remaining extractor-friendly on
/// Windows and Unix.
///
/// # Examples
/// ```ignore
/// let name = sanitize_component("Café (draft).md");
/// assert_eq!(name, "Cafe_draft.md");
/// ```
fn sanitize_component(value: &str) -> String {
    // Step 1: transliterate to ASCII to avoid multi-byte surprises when clamping.
    let transliterated = deunicode::deunicode(value);
    let mut out = String::with_capacity(transliterated.len());
    let mut last: Option<char> = None;

    // Step 2: map characters into the allowed set and collapse runs of `_` and `.`.
    for ch in transliterated.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            ch
        } else if ch.is_whitespace() || ch == '/' || ch == '\\' {
            '_'
        } else {
            '_'
        };

        match mapped {
            '_' => {
                if last != Some('_') {
                    out.push('_');
                    last = Some('_');
                }
            }
            '.' => {
                if last != Some('.') {
                    out.push('.');
                    last = Some('.');
                }
            }
            c => {
                out.push(c);
                last = Some(c);
            }
        }
    }

    // Additional cleanup: avoid a stray underscore immediately before a dot.
    while let Some(pos) = out.find("_.") {
        out.remove(pos);
    }

    // Step 3: trim trailing dots/spaces which can be problematic on Windows.
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }

    // Step 4: fallback for empty or special dot-only names.
    if out.is_empty() || out == "." || out == ".." {
        return "eln_entry".to_string();
    }

    // Step 5: protect against Windows reserved device names for the basename.
    let (basename, ext) = match out.rsplit_once('.') {
        Some((base, ext)) if !base.is_empty() => (base.to_string(), Some(ext.to_string())),
        _ => (out.clone(), None),
    };

    let upper = basename.to_ascii_uppercase();
    let is_reserved = matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL"
            | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
            | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
    );

    if is_reserved {
        let mut new_base = basename;
        new_base.push('_');
        out = if let Some(ext) = ext {
            format!("{new_base}.{ext}")
        } else {
            new_base
        };
    }

    out
}

/// Render markdown to sanitized HTML for embedding in RO-Crate metadata.
fn markdown_to_html(body: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(body, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    ammonia::Builder::default().clean(&html_output).to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    // SystemTime kept for potential temp-file helpers; silence unused warning until reintroduced.
    #[allow(unused_imports)]
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::ArchiveGenre;
    use super::ensure_extension;
    use super::markdown_to_html;
    use super::sanitize_component;
    use super::suggested_archive_name;

    // Sanitization should transliterate accents and preserve dots/extension.
    #[test]
    fn sanitize_component_transliterates_and_preserves_extension_with_dots() {
        let result = sanitize_component("Café (draft).md");
        assert_eq!(result, "Cafe_draft.md");
    }

    // Whitespace and separators must collapse to single underscores.
    #[test]
    fn sanitize_component_collapses_whitespace_and_separators() {
        let result = sanitize_component("Ångström data 2025/11/25");
        assert_eq!(result, "Angstrom_data_2025_11_25");
    }

    // Dots are deduplicated while multi-part extensions remain intact.
    #[test]
    fn sanitize_component_deduplicates_dots_and_keeps_multi_part_extensions() {
        let result = sanitize_component("data..v1...2.tar..gz");
        assert_eq!(result, "data.v1.2.tar.gz");
    }

    // Trailing dots are trimmed for better Windows compatibility.
    #[test]
    fn sanitize_component_trims_trailing_dots() {
        let result = sanitize_component("name.");
        assert_eq!(result, "name");
    }

    // Reserved Windows device names in the basename get a suffix.
    #[test]
    fn sanitize_component_appends_suffix_for_windows_reserved_basenames() {
        assert_eq!(sanitize_component("CON"), "CON_");
        assert_eq!(sanitize_component("NUL.txt"), "NUL_.txt");
    }

    // Pure dots fall back to the default name.
    #[test]
    fn sanitize_component_falls_back_for_dot_only_names() {
        assert_eq!(sanitize_component("..."), "eln_entry");
    }

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
        let html = markdown_to_html("Hello <script>alert('x')</script> ~~gone~~");

        assert!(html.contains("<del>gone</del>"));
        assert!(!html.contains("script"));
    }

    #[test]
    fn archive_genre_serializes_to_expected_str() {
        assert_eq!(ArchiveGenre::Resource.as_str(), "resource");
        assert_eq!(ArchiveGenre::Experiment.as_str(), "experiment");
    }
}
