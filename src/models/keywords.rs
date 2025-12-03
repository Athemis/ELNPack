// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Keyword collection domain helper.

/// Simple wrapper to keep keyword normalization in one place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keywords {
    items: Vec<String>,
}

impl Keywords {
    /// Create a keyword collection, normalizing duplicates case-insensitively.
    ///
    /// Leading/trailing whitespace is preserved; only duplicate tokens
    /// (case-insensitive) are removed, keeping the first occurrence's casing.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use elnpack::models::keywords::Keywords;
    ///
    /// let kw = Keywords::new(vec!["DNA".into(), "dna".into(), "RNA".into()]);
    /// assert_eq!(kw.items(), &["DNA", "RNA"]);
    /// ```
    pub fn new(items: Vec<String>) -> Self {
        let mut kw = Self { items };
        kw.normalize();
        kw
    }

    #[allow(dead_code)]
    /// Borrow the normalized keyword slice.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let kw = elnpack::models::keywords::Keywords::new(vec!["A".into()]);
    /// assert_eq!(kw.items(), &["A"]);
    /// ```
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Consume the wrapper and return the owned vector.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let kw = elnpack::models::keywords::Keywords::new(vec!["X".into()]);
    /// let owned = kw.into_vec();
    /// assert_eq!(owned, vec!["X".to_string()]);
    /// ```
    pub fn into_vec(self) -> Vec<String> {
        self.items
    }

    fn normalize(&mut self) {
        // Dedup case-insensitively while preserving original casing of first occurrence.
        let mut seen = Vec::<String>::new();
        self.items.retain(|kw| {
            let lower = kw.to_ascii_lowercase();
            if seen.contains(&lower) {
                false
            } else {
                seen.push(lower);
                true
            }
        });
    }
}
