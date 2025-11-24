# ELNPack

A lightweight electronic lab notebook (ELN) entry packager built with Rust, eframe/egui, and RO-Crate metadata. Users can write Markdown notes, attach files, preview thumbnails, and export a `.eln` archive containing the experiment text plus attachments and RO-Crate metadata.

## Features
- Markdown editor with quick-insert toolbar (headings, inline/block code, lists, links, quotes, images, strike/underline, rules) and caret-aware insertion.
- Attachment handling with image thumbnails.
- RO-Crate metadata generation and ZIP-based `.eln` archive output.
- Date/time selection for experiment timestamp.

## Project Layout
- `src/main.rs` — application entry; sets up eframe and fonts.
- `src/ui.rs` — overall UI composition and screens.
- `src/editor.rs` — encapsulated Markdown editor component.
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
