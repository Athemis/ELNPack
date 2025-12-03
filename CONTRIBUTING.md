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
- Add SPDX headers to new **source** files (Rust); configs/docs are covered via `REUSE.toml`:
  ```
  // SPDX-License-Identifier: MIT
  // SPDX-FileCopyrightText: 2025 Your Name
  ```
- Prefer MVU layering already used in the project: model/update/view separation, UI-free domain logic.

## Commit & PR Guidelines

- Use Conventional Commits (e.g., `feat: ...`, `fix: ...`, `docs: ...`).
- Valid types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- Examples: `feat: add new feature`; `fix(ui): correct button alignment`; `docs: update README`.
- Include a short PR description:
  - What changed
  - How to test
  - Screenshots for UI tweaks
- Add tests when fixing bugs or adding logic.

## Git hooks

Repository hooks live in `.githooks`. Run `./scripts/install-git-hooks.sh` once to point `core.hooksPath` there.

- `commit-msg` — validates commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
- `pre-commit` — runs `cargo fmt` (writes changes), `cargo clippy`, and `cargo test`; set `SKIP_PRE_COMMIT=1` to bypass locally.
- `pre-push` — re-validates commit messages for pushed commits (catches rebases that skipped `commit-msg`).

`commit-msg` and `pre-push` share `validate-commit-msg.sh` for consistent checks.

## Filing Issues

- For bugs, include steps to reproduce, expected/actual behavior, OS, and screenshots if UI-related.
- For feature requests, describe the use case and desired outcome.

## Security / Privacy

- The app is local-only; no network calls are performed at runtime. Please avoid adding network I/O without discussion.

Thanks for helping make ELNPack better!
