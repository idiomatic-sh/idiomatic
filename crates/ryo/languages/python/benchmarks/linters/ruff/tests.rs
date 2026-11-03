//! Ruff boundary checks run without requiring an installed competitor.
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
    error: ryo_benchmark::domain::diagnostics::errors::DiagnosticsError,
) -> BoundaryFailure {
    use ryo_benchmark::domain::diagnostics::errors::DiagnosticsError as Error;
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

    assert_eq!(
        tracing_for_command("run-ryo"),
        Some(WorkerTracing::Disabled)
    );
    assert_eq!(
        tracing_for_command("profile-ryo"),
        Some(WorkerTracing::Chrome)
    );
    assert_eq!(
        tracing_for_command("diagnostic-ryo"),
        Some(WorkerTracing::JsonLines)
    );
    assert_eq!(tracing_for_command("diagnostic-unknown"), None);
}


mod host_policy;
