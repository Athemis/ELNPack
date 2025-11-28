//! Keyword collection domain helper.

/// Simple wrapper to keep keyword normalization in one place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keywords {
    items: Vec<String>,
}

impl Keywords {
    pub fn new(items: Vec<String>) -> Self {
        let mut kw = Self { items };
        kw.normalize();
        kw
    }

    #[allow(dead_code)]
    pub fn items(&self) -> &[String] {
        &self.items
    }

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
