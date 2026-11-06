//! Ruff boundary checks run without requiring an installed competitor.
use self::engine_test::constants::{DIAGNOSTIC_RYO, DIAGNOSTIC_UNKNOWN, PROFILE_RYO, RUN_RYO};
mod adapter;
#[path = "adapter_test.rs"]
mod adapter_test;
mod constants;
#[path = "composition/diagnostic_protocol.rs"]
mod diagnostic_protocol;
mod engine_test;
mod schemas;
#[path = "schemas_test.rs"]
mod schemas_test;
mod settings;
mod worker_request_input;

/// Comparable failure categories retain the actual boundary error variant in case tables.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BoundaryFailure {
    Invalid,
    Io(std::io::ErrorKind),
    Json,
    Toml,
}

pub(crate) fn failure(
    error: ryo_benchmark_contracts::domain::reports::errors::ReportsError,
) -> BoundaryFailure {
    use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
    match error {
        Error::Invalid(_) => BoundaryFailure::Invalid,
        Error::Io(error) => BoundaryFailure::Io(error.kind()),
        Error::Json(_) => BoundaryFailure::Json,
        Error::Toml(_) => BoundaryFailure::Toml,
    }
}

#[test]
fn worker_modes_are_explicit_and_keep_diagnostics_out_of_timed_run() {
    use diagnostic_protocol::{WorkerTracing, tracing_for_command};

    assert_eq!(tracing_for_command(RUN_RYO), Some(WorkerTracing::Disabled));
    assert_eq!(
        tracing_for_command(PROFILE_RYO),
        Some(WorkerTracing::Chrome)
    );
    assert_eq!(
        tracing_for_command(DIAGNOSTIC_RYO),
        Some(WorkerTracing::JsonLines)
    );
    assert_eq!(tracing_for_command(DIAGNOSTIC_UNKNOWN), None);
}

mod host_policy;

mod frontend_projection;
