// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Attachment domain model and validation helpers (UI-agnostic).

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

/// Sanitized attachment metadata used for archive creation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    /// Original absolute path on disk.
    pub path: PathBuf,
    /// Filename already sanitized for archive storage.
    pub sanitized_name: String,
    /// Detected MIME type.
    pub mime: String,
    /// SHA-256 hash of the file contents or `"unavailable"` if hashing failed.
    pub sha256: String,
    /// File size in bytes.
    pub size: u64,
}

impl Attachment {
    /// Construct a new attachment with pre-sanitized metadata.
    ///
    /// The caller must provide a name that is already filesystem-safe and
    /// unique within the archive.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::path::PathBuf;
    /// use elnpack::models::attachment::Attachment;
    ///
    /// let att = Attachment::new(
    ///     PathBuf::from("/tmp/note.txt"),
    ///     "note.txt".into(),
    ///     "text/plain".into(),
    ///     "unavailable".into(),
    ///     42,
    /// );
    /// assert_eq!(att.sanitized_name, "note.txt");
    /// ```
    pub fn new(
        path: PathBuf,
        sanitized_name: String,
        mime: String,
        sha256: String,
        size: u64,
    ) -> Self {
        Self {
            path,
            sanitized_name,
            mime,
            sha256,
            size,
        }
    }
}

/// Ensure there are no duplicate archive paths produced by sanitized names.
///
/// # Errors
///
/// Returns an error when two attachments share the same `sanitized_name`.
///
/// # Examples
///
/// ```rust,ignore
/// use std::path::PathBuf;
/// use elnpack::models::attachment::{Attachment, assert_unique_sanitized_names};
///
/// let attachments = vec![
///     Attachment::new(PathBuf::from("a.txt"), "a.txt".into(), "text/plain".into(), "unavailable".into(), 1),
///     Attachment::new(PathBuf::from("b.txt"), "a.txt".into(), "text/plain".into(), "unavailable".into(), 1),
/// ];
/// assert!(assert_unique_sanitized_names(&attachments).is_err());
/// ```
pub fn assert_unique_sanitized_names(attachments: &[Attachment]) -> Result<()> {
    let mut seen = HashSet::new();
    for att in attachments {
        if !seen.insert(att.sanitized_name.clone()) {
            return Err(anyhow!(
                "Duplicate attachment filename in archive: {}",
                att.sanitized_name
            ));
        }
    }
    Ok(())
}
