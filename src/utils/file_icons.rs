// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges
//! Phosphor file-icon mapping based on MIME type and filename.
//!
//! This helper is intentionally UI-agnostic so both UI components and
//! non-UI logic can choose a representative icon for a file. It favors
//! MIME matches and falls back to extension/name checks for archive
//! composites (e.g., `*.tar.bz2`).

use std::path::Path;

/// Return a Phosphor file icon matching the MIME type or filename.
pub fn icon_for(mime: &str, path: &Path) -> &'static str {
    let mime = mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let fname = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if mime.starts_with("image/") {
        return match ext.as_str() {
            "png" => egui_phosphor::regular::FILE_PNG,
            "jpg" | "jpeg" => egui_phosphor::regular::FILE_JPG,
            "svg" => egui_phosphor::regular::FILE_SVG,
            _ => egui_phosphor::regular::FILE_IMAGE,
        };
    }
    if mime.starts_with("video/") {
        return egui_phosphor::regular::FILE_VIDEO;
    }
    if mime.starts_with("audio/") {
        return egui_phosphor::regular::FILE_AUDIO;
    }
    if mime == "application/pdf" {
        return egui_phosphor::regular::FILE_PDF;
    }
    if mime == "text/csv" || ext == "csv" {
        return egui_phosphor::regular::FILE_CSV;
    }
    if is_archive_mime(&mime, &ext, &fname) {
        return egui_phosphor::regular::FILE_ARCHIVE;
    }
    if mime == "application/msword"
        || mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        || ext == "doc"
        || ext == "docx"
    {
        return egui_phosphor::regular::FILE_DOC;
    }
    if mime == "application/vnd.ms-excel"
        || mime == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        || ext == "xls"
        || ext == "xlsx"
    {
        return egui_phosphor::regular::FILE_XLS;
    }
    if mime == "application/vnd.ms-powerpoint"
        || mime == "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        || ext == "ppt"
        || ext == "pptx"
    {
        return egui_phosphor::regular::FILE_PPT;
    }
    if mime == "application/vnd.oasis.opendocument.text" || ext == "odt" {
        return egui_phosphor::regular::FILE_TEXT;
    }
    if mime == "application/vnd.oasis.opendocument.spreadsheet" || ext == "ods" {
        return egui_phosphor::regular::FILE_TEXT;
    }
    if mime == "application/vnd.oasis.opendocument.presentation" || ext == "odp" {
        return egui_phosphor::regular::FILE_TEXT;
    }
    if mime == "application/json" || ext == "json" {
        return egui_phosphor::regular::FILE_CODE;
    }
    if mime == "application/xml" || mime == "text/xml" || ext == "xml" {
        return egui_phosphor::regular::FILE_CODE;
    }
    if ext == "ini" {
        return egui_phosphor::regular::FILE_INI;
    }
    if mime == "text/html" || ext == "html" || ext == "htm" {
        return egui_phosphor::regular::FILE_HTML;
    }
    if mime == "text/markdown" || ext == "md" {
        return egui_phosphor::regular::FILE_MD;
    }
    if mime == "text/css" || ext == "css" {
        return egui_phosphor::regular::FILE_CSS;
    }
    if mime == "application/javascript" || mime == "text/javascript" || ext == "js" {
        return egui_phosphor::regular::FILE_JS;
    }
    if ext == "jsx" {
        return egui_phosphor::regular::FILE_JSX;
    }
    if mime == "application/typescript" || ext == "ts" {
        return egui_phosphor::regular::FILE_TS;
    }
    if ext == "tsx" {
        return egui_phosphor::regular::FILE_TSX;
    }
    if ext == "rs" {
        return egui_phosphor::regular::FILE_RS;
    }
    if ext == "py" {
        return egui_phosphor::regular::FILE_PY;
    }
    if ext == "c" {
        return egui_phosphor::regular::FILE_C;
    }
    if ext == "cpp" || ext == "cc" || ext == "cxx" {
        return egui_phosphor::regular::FILE_CPP;
    }
    if ext == "cs" {
        return egui_phosphor::regular::FILE_C_SHARP;
    }
    if ext == "sql" {
        return egui_phosphor::regular::FILE_SQL;
    }
    if ext == "vue" {
        return egui_phosphor::regular::FILE_VUE;
    }
    if ext == "txt" || mime.starts_with("text/") {
        return egui_phosphor::regular::FILE_TXT;
    }

    egui_phosphor::regular::FILE
}

fn is_archive_mime(mime: &str, ext: &str, fname: &str) -> bool {
    mime == "application/zip"
        || mime == "application/gzip"
        || mime == "application/x-7z-compressed"
        || mime == "application/x-rar-compressed"
        || mime == "application/x-gtar"
        || mime == "application/x-tar"
        || mime == "application/x-bzip2"
        || mime == "application/x-xz"
        || mime == "application/zstd"
        || ext == "rar"
        || ext == "7z"
        || ext == "xz"
        || ext == "zst"
        || ext == "bz2"
        || fname.ends_with(".tar.gz")
        || fname.ends_with(".tgz")
        || fname.ends_with(".tar.bz2")
        || fname.ends_with(".tbz2")
        || fname.ends_with(".tar.xz")
        || fname.ends_with(".tar.zst")
}
