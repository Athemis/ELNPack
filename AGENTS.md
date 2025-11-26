# Repository Guidelines

## Project Structure & Module Organization

- Rust 2024 crate; entry point at `src/main.rs`, which initializes the eframe application (ELNPack).
- UI shell in `src/ui.rs`; it wires the overall layout and delegates to components.
- Markdown editor encapsulated in `src/editor.rs`; all toolbar, caret handling, and text editing live here.
- Attachments panel in `src/attachments.rs`; handles attachment list, thumbnails, file dialogs, and computes sanitized filenames when attachments are added. Shows WARNING icon when original and sanitized names differ.
- Keywords editor in `src/keywords.rs`; manages the keyword list, inline editing, and the add-keywords modal.
- Date/time picker in `src/datetime_picker.rs`; encapsulates performed-at selection and conversion to `OffsetDateTime`.
- Filename sanitization utilities in `src/utils.rs`; provides `sanitize_component` function used by attachments and archive modules to ensure cross-platform filename compatibility while preserving extensions.
- Business logic in `src/archive.rs`; handles RO-Crate archive creation, file operations, and metadata generation.
- Add new Rust modules under `src/` and integration tests under `tests/` to keep responsibilities clear.
- No build script needed; egui compiles directly with the Rust code.

## Build, Test, and Development Commands

- `cargo run` — build the crate and launch the desktop app.
- `cargo check` — fast typecheck without running; use before pushing.
- `cargo fmt` — format Rust sources via rustfmt (enforced style).
- `cargo clippy --all-targets --all-features` — lint for common pitfalls; aim for zero warnings.
- `cargo test` — run the test suite (add tests as you extend logic).
- `cargo build --release` — produce an optimized binary for distribution.

## Documentation & Reference Lookup

- Use the Context7 MCP when you need library or API documentation: resolve the library id, then fetch docs via the Context7 MCP get-library-docs endpoint.
- Prefer official documentation domains returned by Context7; avoid ad-hoc web searches unless Context7 lacks coverage.

### Rustdoc conventions to follow

- Use `///` for public items and `//!` for module-level docs; start with a one-line summary ending with a period.
- Structure details with `#` headings (e.g., `# Examples`, `# Errors`, `# Panics`, `# Safety`, `# Performance`).
- Include small, runnable examples marked `no_run`/`ignore` when side effects exist; keep them minimal and dependency-free.
- Explain invariants, panics, and error cases explicitly; prefer present tense and describe behavior, not intent.
- Link related items with intra-doc links like ``[`TypeName`]`` or ``[`module::function`]``; disambiguate with full paths when needed.
- Document private helpers when it aids maintainers; keep explanations concise.

## Coding Style & Naming Conventions

- Follow rustfmt defaults (4-space indent, trailing commas where appropriate); run `cargo fmt` before committing.
- Prefer `snake_case` for functions/variables/files and `CamelCase` for types; keep module files small and focused.
- UI strings live directly in `src/ui.rs` within the egui code; prefer short, actionable labels.
- Validate user input in business logic before reflecting it in the UI to avoid inconsistent state.
- Use English language within the codebase.
- Separate UI concerns (`src/ui.rs`) from business logic (`src/archive.rs`) for maintainability and testability.
- Code comments: use sparingly to explain intent, invariants, or non-obvious control flow; avoid restating what the code already makes clear.
- Phosphor icons: the font is registered in `src/main.rs`; use `egui_phosphor::regular::NAME` (via `RichText` or button labels) instead of embedding SVGs. Keep icon+text buttons short (`format!("{} Label", icon)`) and reuse the helpers already in `src/ui.rs` where possible.

## Module Responsibilities

- **`src/main.rs`**: Application entry point; sets up eframe and launches the UI.
- **`src/ui.rs`**: UI composition and screens; delegates text editing to `editor`, attachments to `attachments`, keywords to `keywords`, and performed-at selection to `datetime_picker`, and calls `archive` for business operations. When exporting, pass the selected `ArchiveGenre` and the keywords from `KeywordsEditor` into `build_and_write_archive`; default to `ArchiveGenre::Experiment` with an empty keyword list if no input is given.
- **`src/editor.rs`**: Markdown editor component (toolbar, cursor-aware insertions, text area).
- **`src/attachments.rs`**: Attachments panel handling list, thumbnails, file dialogs, and inline filename editing. Computes `sanitized_name` for each attachment using `sanitize_component` from `utils` and displays WARNING icon (via `egui_phosphor::regular::WARNING`) when the sanitized name differs from the original filename. Hovering the icon shows the original → sanitized transformation. Users can edit filenames via a pencil button (via `egui_phosphor::regular::PENCIL_SIMPLE`); edited names are sanitized before storage, with validation preventing empty/invalid names and duplicates.
- **`src/keywords.rs`**: Keywords editor component that manages the keyword list, inline keyword edits, and the add-keywords modal, exposing the final keyword `Vec<String>` to `ui`.
- **`src/datetime_picker.rs`**: Date/time picker component that encapsulates performed-at selection (date, hour, minute) and conversion to `OffsetDateTime` in UTC.
- **`src/utils.rs`**: Centralized filename sanitization utilities. The `sanitize_component` function transliterates Unicode with `deunicode`, allows ASCII alphanumerics, hyphens, underscores, and dots (preserving file extensions including multi-part extensions like `.tar.gz`), deduplicates consecutive dots, removes `_.` sequences, trims trailing dots/spaces for Windows compatibility, guards against Windows reserved names (CON/PRN/AUX/NUL/COM1-9/LPT1-9), and falls back to `eln_entry` for empty/invalid names. Includes comprehensive unit tests.
- **`src/archive.rs`**: Pure business logic for archive creation, file handling, and RO-Crate metadata generation. `ro-crate-metadata.json` inside archives must conform to RO-Crate 1.2 (https://w3id.org/ro/crate/1.2), and the archive structure follows the ELN File Format specification (https://github.com/TheELNConsortium/TheELNFileFormat/blob/master/SPECIFICATION.md). The RO-Crate `Dataset` for the experiment includes `genre` (via the `ArchiveGenre` enum: `Experiment` or `Resource`) and a string `keywords` array. Uses pre-computed `sanitized_name` from `AttachmentMeta` for ZIP paths and RO-Crate File nodes. `suggested_archive_name` uses `sanitize_component` from `utils`. No UI dependencies.

## Testing Guidelines

- Use `cargo test` for unit and integration coverage; colocate simple unit tests with modules and broader scenarios under `tests/`.
- Name tests after behavior (e.g., `submits_trimmed_input`) and keep them deterministic.
- Test business logic in `archive.rs` independently of UI; mock file system operations where appropriate.
- UI testing can focus on state management and callback logic without rendering.

## Commit & Pull Request Guidelines

- No existing history; use clear, present-tense commit messages following the Conventional Commits spec (e.g., `feat: add submit handler validation`).
- Keep commits scoped and reviewable; avoid bundling formatting-only changes with behavior changes.
- Pull requests should describe the change, note any UI updates, and link related issues. Include screenshots or short screen recordings when modifying UI behavior.
- Document manual test steps or limitations in the PR description so reviewers can reproduce quickly.

## Security & Configuration Tips

- No external assets are fetched at runtime; all UI is generated by egui at runtime.
- Do not commit secrets or tokens; use environment variables or local config files ignored by Git.
- File dialog operations use `rfd` crate for cross-platform native dialogs.
- For archive structure and metadata, align with the ELN File Format specification: https://raw.githubusercontent.com/TheELNConsortium/TheELNFileFormat/refs/heads/master/SPECIFICATION.md
