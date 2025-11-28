# ELNPack

A lightweight electronic lab notebook (ELN) entry packager built with Rust, eframe/egui, and RO-Crate metadata. Users can write Markdown notes, attach files, preview thumbnails, and export a `.eln` archive containing the experiment text plus attachments and RO-Crate metadata.

## Features

- Markdown editor with quick-insert toolbar (headings, inline/block code, lists, links, quotes, images, strike/underline, rules) and caret-aware insertion.
- Attachments panel with image thumbnails, SHA-256 duplicate detection, sanitized filenames (with warning indicator), inline rename, and MIME/size display.
- Keywords editor with inline chips and add-keywords modal (comma-split with duplicate filtering).
- Date/time picker with local-time selection stored as UTC.
- Save flow that enforces `.eln` extension, surfaces cancel/success/errors, and background command tracking.
- RO-Crate 1.2 metadata + ZIP-based `.eln` archive output (experiment text, attachments, keywords, genre).

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

- `src/main.rs` — entry; calls `app::run()` to launch eframe/egui.
- `src/app/` — app bootstrap and font/options setup.
- `src/mvu/` — MVU kernel (`AppModel`, `Msg`, `Command`, `update`, `run_command`).
- `src/ui/` — top-level UI shell; routes worker messages through `mvu::update`.
- `src/ui/components/` — feature UIs (markdown, attachments, keywords, datetime picker).
- `src/logic/eln.rs` — ELN/RO-Crate build + metadata + suggested archive name.
- `src/models/` — pure data/validation (`attachment`, `keywords`).
- `src/utils/` — helpers (`sanitize_component`, `hash_file`).
- Tests: colocated unit tests plus integration tests under `tests/` (if added).

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

Source files include SPDX headers:

```
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 <Actual Author Name>
```

Add additional `SPDX-FileCopyrightText` lines for significant contributors.

This repository follows the [REUSE Software](https://reuse.software/) specification:

- License terms are defined centrally in `LICENSE`.
- Source files carry SPDX headers as shown above, so tools can automatically detect license and copyright.
- If additional licenses are needed in the future, the corresponding texts will be placed under a `LICENSES/` directory according to REUSE conventions.
