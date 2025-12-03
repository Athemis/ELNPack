# Contributing to ELNPack

Thanks for your interest in improving ELNPack! This guide keeps contributions consistent and easy to review.

## Development Workflow

- Use stable Rust
- Run locally before opening a PR:
  - `cargo fmt`
  - `cargo clippy --all-targets --all-features`
  - `cargo test`
  - `cargo doc` (optional but preferred; fix warnings)
- Keep changes focused; avoid mixing refactors with behavior changes.

## Coding Style

- Follow rustfmt defaults.
- Add SPDX headers to new source files:
  ```
  // SPDX-License-Identifier: MIT
  // SPDX-FileCopyrightText: 2025 Your Name
  ```
- Prefer MVU layering already used in the project: model/update/view separation, UI-free domain logic.

## Commit & PR Guidelines

- Use Conventional Commits (e.g., `feat: ...`, `fix: ...`, `docs: ...`).
- Include a short PR description:
  - What changed
  - How to test
  - Screenshots for UI tweaks
- Add tests when fixing bugs or adding logic.

## Filing Issues

- For bugs, include steps to reproduce, expected/actual behavior, OS, and screenshots if UI-related.
- For feature requests, describe the use case and desired outcome.

## Security / Privacy

- The app is local-only; no network calls are performed at runtime. Please avoid adding network I/O without discussion.

Thanks for helping make ELNPack better!
