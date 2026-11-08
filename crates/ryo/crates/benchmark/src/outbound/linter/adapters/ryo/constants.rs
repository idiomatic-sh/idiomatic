//! Candidate worker invocation protocol.
/// Candidate-specific retained state directory within one observation.
pub(super) const CACHE: &str = "ryo";
/// Executable subcommand accepting one explicit JSON worker request.
pub(super) const WORKER: &str = "run-ryo";
/// Worker argument introducing a staged JSON request file.
pub(super) const REQUEST_FILE: &str = "--request-file";
/// Stable request filename inside the owned worker state directory.
pub(super) const REQUEST_FILENAME: &str = "request.json";
/// Text of the accepted syntax in this part.
pub(super) const ACCEPTED_SYNTAX: &str =
    ryo_benchmark_contracts::domain::requests::constants::ACCEPTED_SYNTAX;
/// Text of the candidate skipped an undeclared or always in this part.
pub(super) const CANDIDATE_SKIPPED_AN_UNDECLARED_OR_ALWAYS: &str =
    "candidate skipped an undeclared or always-applicable rule";
/// Text of the candidate context mode manifest and receipt in this part.
pub(super) const CANDIDATE_CONTEXT_MODE_MANIFEST_AND_RECEIPT: &str =
    "candidate context mode, manifest and receipt disagree";
/// The environment variable the worker's pool takes its thread count from.
pub(super) const WORKERS_ENV: &str = "RAYON_NUM_THREADS";
/// One worker thread: the candidate runs on one thread, as the reference tool does.
pub(super) const ONE_WORKER: &str = "1";
