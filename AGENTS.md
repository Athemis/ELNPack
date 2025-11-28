<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: 2025 Alexander Minges
-->

# Repository Guidelines

## Project Structure & Module Organization

<!-- REUSE-IgnoreStart -->

- Rust 2024 crate; entry point at `src/main.rs`, which calls `app::run()` to start eframe/egui. SPDX headers on sources: `SPDX-License-Identifier: MIT` plus one or more `SPDX-FileCopyrightText` lines naming actual authors (add additional lines for significant contributors).
<!-- REUSE-IgnoreEnd -->
- License text duplicated for REUSE: root `LICENSE` plus `LICENSES/MIT.txt` (SPDX MIT identifier).
- See "Module Responsibilities" below for detailed module-level responsibilities and file layout.
- No build script needed; egui compiles directly with the Rust code.

### MVU paradigm & layering

- **Model**: `mvu::AppModel` plus per-component models (`MarkdownModel`, `DateTimeModel`, `KeywordsModel`, `AttachmentsModel`).
- **View**: Component `view(...) -> Vec<Msg>` in `ui/components/*`; `ui::ElnPackApp` composes and wraps into `mvu::Msg`.
- **Update**: `mvu::update` routes messages to reducers, validates save requests, and enqueues `Command`s. `run_command` performs side-effects and emits follow-up messages.
- **Commands**: `PickFiles`, `HashFile`, `LoadThumbnail`, `SaveArchive`. Results feed back as messages (`AttachmentsMsg::FilesPicked/HashComputed/ThumbnailReady`, `Msg::SaveCompleted`).
- **Flow**: UI event → component `Msg` → `mvu::update` mutates model/enqueues commands → `run_command` performs IO → resulting `Msg` goes back into `update` → views re-render from `AppModel`.
- **Data vs UI**: Pure data in `src/models/`; business/format logic in `src/logic/eln.rs`; MVU kernel in `src/mvu/`; UI composition in `src/ui/`; components in `src/ui/components/`; utilities in `src/utils/`.

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
- Separate UI concerns (`src/ui.rs`) from business logic (`src/logic/eln.rs`) for maintainability and testability.
- Code comments: use sparingly to explain intent, invariants, or non-obvious control flow; avoid restating what the code already makes clear.
- Phosphor icons: the font is registered in `src/main.rs`; use `egui_phosphor::regular::NAME` (via `RichText` or button labels) instead of embedding SVGs. Keep icon+text buttons short (`format!("{} Label", icon)`) and reuse the helpers already in `src/ui.rs` where possible.

## Module Responsibilities

- **`src/main.rs`**: Entry; loads modules and calls `app::run()`.
- **`src/app/`**: eframe bootstrap and font/theme setup.
- **`src/ui/`**: UI composition and screens; collects component messages and feeds them into `mvu::update`/`run_command`. Save flow: opens file dialog, dispatches `Msg::SaveRequested`; kernel validates and calls logic.
- **`src/ui/components/markdown.rs`**: Markdown editor (toolbar, cursor-aware insertions, text area).
- **`src/ui/components/attachments.rs`**: Attachments panel (list, thumbnails, inline filename editing). Computes `sanitized_name` using `sanitize_component`; shows WARNING icon on sanitized mismatch; emits commands for file picking/hashing/thumbnails; edited names are sanitized/deduped.
- **`src/ui/components/keywords.rs`**: Keywords editor with inline edits and add-keywords modal.
- **`src/ui/components/datetime_picker.rs`**: Date/time picker; converts to `OffsetDateTime`.
- **`src/utils/`**: Helpers (`sanitize_component`, `hash_file`).
- **`src/models/`**: Pure data/validation (`attachment`, `keywords`).
- **`src/logic/eln.rs`**: ELN RO-Crate build/write, metadata, suggested archive name. Conforms to RO-Crate 1.2 and ELN File Format spec; uses pre-sanitized names from attachments. No UI deps.

## Testing Guidelines

- Use `cargo test` for unit and integration coverage; colocate simple unit tests with modules and broader scenarios under `tests/`.
- Name tests after behavior (e.g., `submits_trimmed_input`) and keep them deterministic.
- Test business logic in `src/logic/eln.rs` independently of UI; mock file system operations where appropriate.
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
