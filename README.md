# ELNPack

A lightweight electronic lab notebook (ELN) entry packager built with Rust, eframe/egui, and RO-Crate metadata. Users can write Markdown notes, attach files, preview thumbnails, and export a `.eln` archive containing the experiment text plus attachments and RO-Crate metadata.

## Features
- Markdown editor with quick-insert toolbar (headings, inline/block code, lists, links, quotes, images, strike/underline, rules) and caret-aware insertion.
- Attachment handling with image thumbnails and duplicate detection via SHA-256 hashing.
- Filename sanitization for cross-platform compatibility (preserves extensions, handles special characters).
- RO-Crate metadata generation and ZIP-based `.eln` archive output.
- Date/time selection for experiment timestamp.

## Filename Sanitization & Editing
When you attach files, ELNPack automatically sanitizes filenames to ensure cross-platform compatibility while preserving file extensions (including multi-part extensions like `.tar.gz`). The sanitization process:

1. Transliterates Unicode characters (e.g., `Café` → `Cafe`)
2. Replaces separators and special characters with underscores
3. Preserves dots in extensions while deduplicating consecutive dots
4. Trims trailing dots and spaces for Windows compatibility
5. Guards against Windows reserved names (e.g., `CON`, `PRN`, `AUX`)
6. Falls back to `eln_entry` for empty or invalid names

When a filename is sanitized, the attachments panel displays a **⚠ WARNING** icon next to the sanitized name. Hover over the icon to see the original → sanitized transformation.

### Editing Filenames
You can edit attachment filenames by clicking the **pencil button** (🖊) next to any filename. The inline editor allows you to:
- Rename files before creating the archive
- Use Enter/Tab to save or click the ✔ button
- Cancel with the ✕ button

All edited filenames are automatically sanitized using the same rules above, ensuring filesystem safety. Duplicate filenames are prevented, and validation errors are shown in the status bar.

## Project Layout
- `src/main.rs` — application entry; sets up eframe and fonts.
- `src/ui.rs` — overall UI composition and screens.
- `src/editor.rs` — encapsulated Markdown editor component.
- `src/attachments.rs` — attachments panel (list, thumbnails, file dialogs).
- `src/keywords.rs` — keywords editor with inline editing and add-keywords modal.
- `src/datetime_picker.rs` — date/time picker for experiment timestamp.
- `src/utils.rs` — filename sanitization utilities.
- `src/archive.rs` — archive building, file handling, metadata generation.

## Development
- Run: `cargo run`
- Check: `cargo check`
- Lint: `cargo clippy --all-targets --all-features`
- Format: `cargo fmt`
- Test: `cargo test`

## Building Release
```
cargo build --release
```

## License
This project is licensed under the MIT License. See [LICENSE.md](LICENSE.md).
