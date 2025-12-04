# ELNPack

[![CI](https://github.com/Athemis/ELNPack/actions/workflows/ci.yml/badge.svg)](https://github.com/Athemis/ELNPack/actions/workflows/ci.yml)
[![CodeQL](https://github.com/Athemis/ELNPack/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/Athemis/ELNPack/actions/workflows/github-code-scanning/codeql)
[![REUSE status](https://api.reuse.software/badge/github.com/Athemis/ELNPack)](https://api.reuse.software/info/github.com/Athemis/ELNPack)
![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/Athemis/ELNPack/total)
![GitHub Downloads (all assets, latest release)](https://img.shields.io/github/downloads/Athemis/ELNPack/latest/total)

A lightweight electronic lab notebook (ELN) entry packager built with [Rust](https://rust-lang.org), [egui](https://www.egui.rs), and [RO-Crate](https://www.researchobject.org/ro-crate) metadata. Users can write Markdown notes, attach files, add keywords, and export a `.eln` archive (see [The ELN Consortium](https://the.elnconsortium.org)) containing the experiment text plus attachments and RO-Crate metadata. `.eln` archives can be imported into a wide range of ELNs, the current focus of ELNPack is however compatibility with [eLabFTW](https://www.elabftw.net).

## Features

- Simple **Markdown** editor with quick-insert toolbar
- **Attachments** panel with image thumbnails, duplicate detection and filename sanitization
- Keywords editor, supporting mass import of comma-separated keywords
- **Metadata** editor, allowing users to add custom metadata fields ad import from eLabFTW extra field JSON files

## Filename Sanitization & Editing

When you attach files, ELNPack automatically sanitizes filenames to ensure cross-platform compatibility. The sanitization process:

1. Transliterates Unicode characters (e.g., `Café` → `Cafe`)
2. Replaces separators and special characters with underscores
3. Guards against Windows reserved names (e.g., `CON`, `PRN`, `AUX`)

When a filename is sanitized, the attachments panel displays a **⚠ WARNING** icon next to the sanitized name. Hover over the icon to see the original → sanitized transformation.

### Editing Filenames

You can edit attachment filenames by clicking the **pencil button** (🖊) next to any filename. The inline editor allows you to rename files before creating the archive

All edited filenames are automatically sanitized using the same rules above. Duplicate filenames are prevented, and validation errors are shown in the status bar.

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

This project is licensed under the MIT License. See [LICENSE](LICENSE).

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

## Security & Privacy

- ELNPack runs locally and does not make outbound network requests.
- File dialogs use native OS pickers; archives are written only to user-selected locations.

## Windows prerequisites

- The binaries rely on the Microsoft Visual C++ Redistributable v14. On Windows 10+, install the latest package from Microsoft: https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist
- For Windows 7 targets (`*-win7-windows-msvc`), use the older 2015–2019 redistributable which still supports Win7:
  - x64: https://aka.ms/vs/16/release/vc_redist.x64.exe
  - x86: https://aka.ms/vs/16/release/vc_redist.x86.exe
- Pick the x86 installer for 32-bit systems.

## Contributing

We welcome issues and PRs! See [CONTRIBUTING](CONTRIBUTING.md) for coding standards, testing checklist, git hooks, and commit conventions.

## FAQ

- **Why is there a AGENTS.md? Is ELNPack AI created?**

  Short answer: No, ELNPack is not AI created. However, I'd like to use available AI tools
  to provide support in e.g. bug solving, quality control and documentation. AGENTS.md is read and
  understood by most of these tools.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).

Source files include SPDX headers:

```
// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 <Actual Author Name>
```
