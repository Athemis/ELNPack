# ELNPack

[![CI](https://github.com/Athemis/ELNPack/actions/workflows/ci.yml/badge.svg)](https://github.com/Athemis/ELNPack/actions/workflows/ci.yml)
[![CodeQL](https://github.com/Athemis/ELNPack/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/Athemis/ELNPack/actions/workflows/github-code-scanning/codeql)
[![REUSE status](https://api.reuse.software/badge/github.com/Athemis/ELNPack)](https://api.reuse.software/info/github.com/Athemis/ELNPack)
![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/Athemis/ELNPack/total)
![GitHub Downloads (all assets, latest release)](https://img.shields.io/github/downloads/Athemis/ELNPack/latest/total)

A lightweight electronic lab notebook (ELN) entry packager built with [Rust](https://rust-lang.org), [egui](https://www.egui.rs), and [RO-Crate](https://www.researchobject.org/ro-crate) metadata. Users can write Markdown notes, attach files, add keywords, and export an `.eln` archive (see [The ELN Consortium](https://the.elnconsortium.org)) containing the experiment text plus attachments and RO-Crate metadata. `.eln` archives can be imported into many ELNs; ELNPack currently focuses on compatibility with [eLabFTW](https://www.elabftw.net).

For detailed usage instructions, see the **[User Guide](https://athemis.github.io/ELNPack/)**.

## Features

- Simple **Markdown** editor with quick-insert toolbar - choose Markdown or HTML at export time
- **Attachments** panel with image thumbnails, duplicate detection by sanitized name and SHA-256, and filename sanitization
- Keywords editor, supporting mass import of comma-separated keywords
- **Metadata** editor with eLabFTW-style extra fields/groups (import, edit, validate) - exports per-field `PropertyValue` nodes plus an `elabftw_metadata` blob for RO-Crate/ELN File Format compatibility

## Installation

- Download the latest release artifacts from GitHub Releases (pick the archive matching your OS/CPU: Linux tar.gz, Windows zip, macOS tar.gz). Binaries are **not code-signed**, so Windows SmartScreen and macOS Gatekeeper may prompt; on macOS, right-click → Open to allow.
- Windows: tested on Windows 10+ (x86_64 MSVC). Install the latest VC++ Redistributable if your system lacks the Universal CRT.
- macOS: tested on current macOS releases for Intel and Apple Silicon; relies only on built-in system frameworks.
- Linux: built against glibc (e.g., Ubuntu 20.04+ / glibc ≥ 2.31). On minimal images ensure `libc6`, `libgcc-s1`, and `libm` are present.

## Platforms & runtime prerequisites

- Prebuilt release artifacts target Windows (x86_64/i686 MSVC, Windows 10+), Linux (x86_64/i686 glibc), and macOS (arm64/x86_64). You can also build locally with Cargo.
- Linux builds link only against glibc, libm, and libgcc_s (typical on mainstream distros). If you’re on an ultra-minimal image, ensure `libc6`, `libgcc-s1`, and `libm` are present.
- macOS builds rely only on built-in system frameworks.
- Windows builds rely on system DLLs available on Windows 10+ (`kernel32`, `user32`, `gdi32`, `uxtheme`, `opengl32`, API set DLLs). On older installs missing the Universal CRT, install the latest VC++ Redistributable: [x64](https://aka.ms/vs/18/release/vc_redist.x64.exe).

## Quickstart

1. Launch the app (`cargo run` or run the downloaded binary).
2. Enter a title, write your note, add keywords, metadata, and attachments.
3. Click **Save** to pick an output path; the archive is exported as `.eln` with RO-Crate metadata.

## Filename Sanitization & Editing

When you attach files, ELNPack automatically sanitizes filenames to ensure cross-platform compatibility. The sanitization process:

1. Transliterates Unicode characters (e.g., `Café` → `Cafe`)
2. Collapses runs of separators/dots to a single `_`/`.` and trims trailing dots/spaces
3. Replaces other special characters with underscores
4. Falls back to `eln_entry` for empty/dot-only names and appends “_” to Windows reserved basenames (e.g., `CON` → `CON_`)

When a filename is sanitized, the attachments panel displays a **⚠ WARNING** icon next to the sanitized name. Hover over the icon to see the original → sanitized transformation.

Attachments are also rehashed before saving to catch tampering between selection and export. Duplicate attachments are skipped if either the sanitized name or SHA-256 digest matches an existing item.

### Editing Filenames

You can edit attachment filenames by clicking the **pencil button** (🖊) next to any filename. The inline editor allows you to rename files before creating the archive.

All edited filenames are automatically sanitized using the same rules above. Duplicate filenames are prevented, and validation errors are shown in the status bar.

## Development

Quick start: `cargo fmt && cargo test` before sending changes, and `cargo run` to launch. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full workflow and release automation notes.

## Building Release

- Local release binary: `cargo build --release`
- Tagged releases (refs `v*`) trigger `.github/workflows/release.yml` to lint/test and build artifacts for Linux (x86_64/i686), macOS (arm64/x86_64), and Windows (x86_64/i686); git-cliff generates the release notes from commits.
- Release prep helper: `scripts/create_release.sh <version>` bumps workspace version, commits, and tags; push the tag to start the pipeline.

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

## Security & Privacy

- ELNPack runs locally and does not make outbound network requests.
- File dialogs use native OS pickers; archives are written only to user-selected locations.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).

Source files include SPDX headers:

```rust
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 <Actual Author Name>
```

Add additional `SPDX-FileCopyrightText` lines for significant contributors.

This repository follows the [REUSE Software](https://reuse.software/) specification:

- License terms are defined centrally in `LICENSE`.
- Source files carry SPDX headers as shown above, so tools can automatically detect license and copyright.
- If additional licenses are needed in the future, the corresponding texts will be placed under a `LICENSES/` directory according to REUSE conventions.

## FAQ

- **Why is there an AGENTS.md? Is ELNPack AI created?**

  Short answer: No, ELNPack is not AI created. However, I'd like to use available AI tools
  to provide support in e.g. bug solving, quality control and documentation. AGENTS.md is read and
  understood by most of these tools.
