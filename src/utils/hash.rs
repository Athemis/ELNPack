// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! File hashing helper utilities.

use std::fs::File;
use std::io;
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Compute the SHA-256 hash of a file and return its lowercase hex digest.
///
/// # Errors
///
/// Returns an error when the file cannot be opened or fully read.
///
/// # Examples
///
/// ```rust,ignore
/// use std::path::Path;
/// let digest = elnpack::utils::hash_file(Path::new("notes.txt"))?;
/// assert_eq!(digest.len(), 64);
/// ```
pub fn hash_file(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("Failed to open file for hashing: {:?}", path))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)
        .with_context(|| format!("Failed to read file for hashing: {:?}", path))?;
    Ok(format!("{:x}", hasher.finalize()))
}
