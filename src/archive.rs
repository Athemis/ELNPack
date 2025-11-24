use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use mime_guess::mime::Mime;
use pulldown_cmark::{Options, Parser, html};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use zip::{CompressionMethod, write::FileOptions};

pub fn suggested_archive_name(title: &str) -> String {
    let mut sanitized = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch.to_ascii_lowercase());
        } else if ch.is_whitespace() || matches!(ch, '-' | '_') {
            sanitized.push('_');
        }
    }
    let trimmed = sanitized.trim_matches('_');
    let base = if trimmed.is_empty() {
        "eln-entry"
    } else {
        trimmed
    };
    format!("{}.eln", base)
}

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

pub fn build_and_write_archive(
    output: &Path,
    title: &str,
    body: &str,
    attachments: &[PathBuf],
    performed_at: OffsetDateTime,
) -> Result<()> {
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
    for (idx, source_path) in attachments.iter().enumerate() {
        let display_name = source_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("attachment-{}.bin", idx + 1));
        let archive_path = format!("{}{}", experiment_dir, display_name);
        let id = format!("./experiment/{}", display_name);

        zip.start_file(&archive_path, options)
            .with_context(|| format!("Failed to add file {} to archive", archive_path))?;

        let mut reader = File::open(source_path)
            .with_context(|| format!("Failed to read attachment {:?}", source_path))?;
        let mut hasher = Sha256::new();
        let mut written = 0u64;
        let mut buffer = [0u8; 8192];
        loop {
            let read = reader
                .read(&mut buffer)
                .with_context(|| format!("Failed to read from {:?}", source_path))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            zip.write_all(&buffer[..read])
                .with_context(|| format!("Failed to write {} into archive", archive_path))?;
            written += read as u64;
        }

        let sha256 = format!("{:x}", hasher.finalize());
        let encoding = guess_mime(source_path).essence_str().to_string();

        file_nodes.push(serde_json::json!({
            "@id": id,
            "@type": "File",
            "name": display_name,
            "encodingFormat": encoding,
            "contentSize": written.to_string(),
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
        "conformsTo": { "@id": "https://w3id.org/ro/crate/1.1" },
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
        "@context": "https://w3id.org/ro/crate/1.1/context",
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

fn sanitize_component(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            sanitized.push(ch);
        } else if ch.is_whitespace() {
            sanitized.push('_');
        }
    }
    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "eln_entry".into()
    } else {
        trimmed.to_string()
    }
}

fn guess_mime(path: &Path) -> Mime {
    mime_guess::from_path(path).first_or_octet_stream()
}

fn markdown_to_html(body: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(body, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
