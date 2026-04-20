// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! File hashing helper utilities.

use std::fs::File;
use std::io::Read;
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
    let mut buffer = [0_u8; 8 * 1024];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("Failed to read file for hashing: {:?}", path))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::hash_file;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn hashes_file_contents_as_lowercase_sha256_hex() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        fs::write(&path, b"abc").unwrap();

        let digest = hash_file(&path).unwrap();

        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
