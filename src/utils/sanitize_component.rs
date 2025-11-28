// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Alexander Minges

//! Produce filesystem-safe path components shared across the app.

/// Produce a filesystem-safe path component.
///
/// # Steps
/// - Transliterate Unicode to ASCII with `deunicode` (e.g., "Å" → "A").
/// - Allow ASCII alphanumerics plus `-`, `_`, and `.`; treat other characters as `_`.
/// - Collapse runs of `_` and `.`; trim trailing dots/spaces.
/// - Guard against reserved/empty names.
///
/// This keeps multi-part extensions intact (for example `data.v1.2.tar.gz`
/// stays `data.v1.2.tar.gz`) while remaining extractor-friendly on
/// Windows and Unix.
pub fn sanitize_component(value: &str) -> String {
    // Step 1: transliterate to ASCII to avoid multi-byte surprises.
    let transliterated = deunicode::deunicode(value);
    let mut out = String::with_capacity(transliterated.len());
    let mut last: Option<char> = None;

    // Step 2: map characters into the allowed set and collapse runs of `_` and `.`.
    for ch in transliterated.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            ch
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

    // Trim trailing dots/spaces which can be problematic on Windows.
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }

    // Fallback for empty or special dot-only names.
    if out.is_empty() || out == "." || out == ".." {
        return "eln_entry".to_string();
    }

    // Protect against Windows reserved device names for the basename.
    let (basename, ext) = match out.rsplit_once('.') {
        Some((base, ext)) if !base.is_empty() => (base.to_string(), Some(ext.to_string())),
        _ => (out.clone(), None),
    };

    let upper = basename.to_ascii_uppercase();
    let is_reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
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

#[cfg(test)]
mod tests {
    use super::sanitize_component;

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
}
