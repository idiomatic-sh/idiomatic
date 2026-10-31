//! Builds the execution adapters from dependencies chosen by the composition.
use super::adapters::process::adapter::ExecutionProcessAdapter;
/// Construct the process implementation of the execution contract.
#[must_use]
pub fn process() -> impl super::interface::ExecutionPort {
    ExecutionProcessAdapter
}

/// Construct child resource instrumentation from an explicitly pinned GNU time binary.
#[must_use]
pub fn timed(
    executable: std::path::PathBuf,
    sha256: String,
) -> impl super::interface::ExecutionPort {
    super::adapters::timed::adapter::ExecutionTimedAdapter {
        settings: super::adapters::timed::settings::ExecutionTimedSettings { executable, sha256 },
    }
}
