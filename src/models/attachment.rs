// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Attachment domain model and validation helpers (UI-agnostic).

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

/// Sanitized attachment metadata used for archive creation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub path: PathBuf,
    pub sanitized_name: String,
    pub mime: String,
    pub sha256: String,
    pub size: u64,
}

impl Attachment {
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
