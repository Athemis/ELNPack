// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Shared helper utilities reused by UI and business logic.

pub mod hash;
pub mod sanitize_component;

/// Compute the SHA-256 hash of a file.
pub use hash::hash_file;
/// Sanitize user-provided strings into filesystem-safe path components.
pub use sanitize_component::sanitize_component;
