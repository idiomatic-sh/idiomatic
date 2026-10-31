//! Candidate worker invocation protocol.
/// Candidate-specific retained state directory within one observation.
pub(super) const CACHE: &str = "ryo";
/// Executable subcommand accepting one explicit JSON worker request.
pub(super) const WORKER: &str = "run-ryo";
