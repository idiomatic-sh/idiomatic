//! Candidate worker invocation protocol.
/// Candidate-specific retained state directory within one observation.
pub(super) const CACHE: &str = "ryo";
/// Executable subcommand accepting one explicit JSON worker request.
pub(super) const WORKER: &str = "run-ryo";
/// Worker argument introducing a staged JSON request file.
pub(super) const REQUEST_FILE: &str = "--request-file";
/// Stable request filename inside the owned worker state directory.
pub(super) const REQUEST_FILENAME: &str = "request.json";
