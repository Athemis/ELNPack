## [unreleased]

### 🐛 Bug Fixes

- *(markdown)* Adapt cursor range index to CharIndex for egui 0.35
- *(ui)* Replace deprecated show_inside with show for egui 0.35

### ⚙️ Miscellaneous Tasks

- Add tokensave artifacts to gitignore
## [0.1.4] - 2026-04-20

### 🚀 Features

- *(release)* Add full changelog generation to release script

### 🐛 Bug Fixes

- Adapt app to updated egui and sha2 dependencies (#100)

### 🚜 Refactor

- Move thumbnail textures into ui state (#101)

### 📚 Documentation

- Update changelog
- Update README.md (#40)

### ⚙️ Miscellaneous Tasks

- Add clippy configuration file (#56)
- Set MSRV to 1.92.0 (#58)
- Bump version to 0.1.4 (#102)
## [0.1.3] - 2025-12-12

### 🚀 Features

- Add Enter key support for all text inputs

### 🐛 Bug Fixes

- Use conventional commit message for release version bump

### ⚙️ Miscellaneous Tasks

- Release v0.1.2 (#30)
- *(ci)* Switch to tag-based releases with git-cliff notes (#34)
- Use --pathspec-file-nul with git add to avoid parsing issues
- Improve version bump commit to filter pathspecs correctly
- Improve version bump commit staging in create_release.sh
- Improve file staging logic in create_release.sh script
- *(release)* Bump version to 0.1.3-test
- Update git-cliff-action to v4 with verbose changelog generation and header stripping
- Add step to collect release files in release workflow
- Only show latest changes in release
- *(release)* Bump version to 0.1.3
## [0.1.2] - 2025-12-10

### 🚀 Features

- Add help button opening the user guide (#27)
- Add MIME-aware file icons (#28)

### 🐛 Bug Fixes

- *(ui)* Replace keyword editor unicode buttons with phosphor icons (#24)
- *(ci)* Use semver notation for mdbook
- Only check new commits in pre-push hook

### 📚 Documentation

- Add GitHub downloads badges to README.md
- Update features section in README.md
- Update README.md and CONTRIBUTING.md
- Update README install/runtime notes
- Link README to contributing guide
- Update Windows VC++ Redistributable link in README.md
- Reorder sections in README
- Update README
- Update Windows VC++ Redistributable URLs
- Add user guide (#26)
- Add link to user guide
- Add license information to user guide

### ⚙️ Miscellaneous Tasks

- Release v0.1.0
- Add Apache License 2.0 file
- Update issue/PR templates
- Trigger mdBook build only on relevant docs changes
- Add CC-BY 4.0 license and update REUSE configuration
- Add REUSE lint step to CI workflow
- Bump version to 0.1.1
- Release v0.1.1 (#29)
- Bump version to 0.1.2
## [0.1.0] - 2025-12-03

### 🚀 Features

- Implement ELN archive builder with egui GUI
- Add date picker and validate timestamps
- Show image thumbnails in attachments
- Add theme toggle and image thumbnails
- Add svg thumbnails
- Add markdown preview and html export
- Add simple markdown edit/preview controls
- Add heading dropdown to markdown editor
- Enrich RO-Crate metadata with genre and keywords
- Improve keyword editor layout
- Improve markdown editor and add side attachments pane
- Show attachment metadata and move panel inline
- Make markdown editor vertically resizable with full-width layout
- Improve filename sanitization and hash verification
- Table support in editor
- Add table size picker
- Support math expressions
- Reject duplicate attachment names
- Add export format selection for main text
- Add elabftw extra fields import scaffold
- Enhance extra fields validation and UI
- Allow renaming imported metadata groups
- Make metadata select fields toggleable
- Add RemoveField message handler to extra fields
- Inline edit metadata field titles and descriptions
- Edit metadata fields via modal
- Add delete control for imported metadata fields
- Allow removing metadata groups and unassign fields
- Add metadata fields via existing modal
- Allow to edit group assignment of fields
- Allow changing field group in modal with default fallback
- Show empty metadata groups and support add-group/add-field flow
- Add fields from group context and default section
- Auto-create default group when adding first metadata field
- *(ui)* Hide empty default section; only show when ungrouped fields exist
- Allow choosing field type when creating extra fields
- *(ui)* Prevent deleting the last extra-field group
- *(ui)* Make extra-field groups collapsible
- Enforce unique extra-field names with inline warning
- Block save on empty required extra fields and highlight them
- Disable save until required extra fields are filled
- Highlight empty required extra fields and block save
- Remove redundant required-fields modal
- Validate url extra fields
- Validate numeric extra field types
- Block saving when fields are invalid
- Add read-only support to extra fields
- Add Windows 7 release target (#11)

### 🐛 Bug Fixes

- Sanitize html and filenames
- Tighten archive name sanitization
- *(datetime)* Align default picker with local time and clarify ui
- *(editor)* Fix styling of text selections
- Keep file extensions; improve filename sanitation
- Always reassign fields to a Default group when removing groups
- Reuse existing group when adding fields and simplify group picker
- Align elabftw export for importer compatibility
- Document eln format version constant
- Validate email extra fields
- Respect extra field readonly in editor and selectors
- Reset modal edit state on import
- Handle empty file selection in attachments update function (#9)

### 🚜 Refactor

- Extract markdown editor component
- Unify markdown style application helpers
- Simplify attachments panel layout
- Simplify attachments and fix all clippy warnings
- *(ui)* Extract keywords and datetime picker components
- Align project layout with MVU structure
- Extract shared validation logic for git hooks
- *(ui)* Reuse a shared toggle switch and separate multi-select control
- *(ui)* Streamline metadata field layout and unit handling
- *(ui)* Refactor group header rendering with improved layout
- Centralize extra field validation for ui and save paths
- Deduplicate extra field trimming helpers
- Centralize extra field group helpers
- Reuse draft application for extra field creation/edit
- Extract extra field view helpers
- Clarify lowest_position_group_id naming.
- Simplify name_conflict comparison
- Simplify extra field value serialization
- Simplify group renaming logic

### 📚 Documentation

- Update contributor guidelines
- Update AGENTS for editor module
- Refresh naming and licensing
- Add rustdoc to ui components
- Document REUSE usage and add MIT license file
- Refine README and update project guidelines
- Add rustdoc comments to core models and UI components
- Update README.md
- Update git hooks documentation in README and install script
- Add MVU layering guidance to AGENTS.md
- Clarify ExtraField value conversion
- Improve inline code documentation (#10)
- Update contributing guide and README
- Add Windows prerequisites section to README with VC++ info

### 🎨 Styling

- Use icon for attachment removal button
- Use phosphor icons for confirmations
- Use phosphor icons for heading selection
- Drop RichText for icon display
- Add icon to save button
- *(ui)* Modify extra fields metadata import UI labels
- Add missing newlines at end of files
- Fix clippy warnings
- Rename test to clarify group display name fallback behavior

### 🧪 Testing

- Add coverage for archive helpers and attachment thumbnails
- Improve math syntax checks
- Add tests for field/group edits
- Add helpers for extra field fixtures
- Clarify group removal coverage for extra fields
- Remove redundant field removal test

### ⚙️ Miscellaneous Tasks

- Streamline image dependencies
- Use egui dark/light mode switch
- Place theme switch in top toolbar
- Simplify theme switch toolbar
- Streamline heading picker
- Tidy heading dropdown placement
- Restyle date/time inputs
- Update egui stack and toolbar icons
- Update AGENTS.md
- Update AGENTS.md
- Rename LICENSE.md to LICENSE
- Update copyright statement in LICENSE
- *(license)* Add SPDX headers and fix copyright holder
- Update README
- Remove outdated docs
- Add CI workflow and GitHub project templates
- Add initial Dependabot configuration
- Pin rust toolchain to stable
- Add dependabot GitHub Actions updates
- Fix clippy warnings
- Fix code formatting
- Add distributable git hooks and install script
- Add read permission in CI workflow
- Add missing SPDX license headers
- Fix format of SPDX headers
- Reformat SPDX header in README.md
- Add REUSE dep5 file
- Update SPDX headers
- Migrate from dep5 to REUSE.toml
- Ignore REUSE checks in AGENTS.md
- Add CI and CodeQL badges to README.md
- Reinstate SPDX headers in issue templates
- Update format check in pre-commit hook
- Update pre-commit-hook
- Add static release pipeline
- Add manual trigger to release pipeline
- Cache cargo deps in release checks
- Keep other release builds running on failure
- Fix release build target expansion on windows
- Cache cargo artifacts in CI workflow
- Use dtolnay/rust-toolchain in workflows
- Bump checkout action version
- Pin rust-toolchan action version
- Add math library linking for musl target
- Add musl linker configuration
- Add strip binary step for Linux builds
- Remove musl Linux build
- Release profile optimizations
- Add pre-push hook to validate conventional commit messages
- Add 32-bit linux/windows targets to release builds
- Add i586 Linux target to release workflow
- Sanitize ref names in release artifacts
- Fix release artifact naming with safe ref env
- Update release builds
- Rely on cargo strip and drop workflow strip step
- Simplify metadata field UI and move allow-multi to modal
- Remove unnecessary v5 uuid feature
- Add comment explaining group name rename logic
- Cover configs/docs via REUSE annotations
- Clean SPDX headers for configs/docs covered by REUSE.toml
- Refactor release pipeline (#12)
- Use stable rust toolchain in workflow
