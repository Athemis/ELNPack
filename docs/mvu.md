# MVU in ELNPack

- **Model**: `mvu::AppModel` holds top-level UI state (`entry_title`, `archive_genre`, plus component models). Component models live in their modules: `MarkdownModel`, `DateTimeModel`, `KeywordsModel`, `AttachmentsModel`.
- **View**: Each component exposes `view(...) -> Vec<Msg>`; `ui.rs` composes them and wraps messages into `mvu::Msg`.
- **Update**: `mvu::update` reduces messages into the model and enqueues side-effect `Command`s (file dialogs, hashing, saving). `run_command` executes commands synchronously for now and returns follow-up messages.
- **Commands**: `PickFiles`, `HashFile`, `SaveArchive`. They feed back into `Msg::Attachments` or `Msg::SaveCompleted`.
- **Flow**: UI event → component `Msg` → `mvu::update` mutates model/enqueues commands → `run_command` performs IO → resulting `Msg` goes back into `update` → views re-render from `AppModel`.
- **Domain vs UI**: Pure data lives in `src/domain/`; business logic in `src/archive.rs` and `src/utils.rs`; MVU kernel is `src/mvu.rs`; UI composition is `src/ui.rs`; components stay in their respective files.
