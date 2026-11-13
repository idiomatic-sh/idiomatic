//! Comparable failure categories retain the actual boundary error variant in case tables.
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// The error variant a case refuses with.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BoundaryFailure {
    Invalid,
    Io(std::io::ErrorKind),
    Json,
    Toml,
}

pub(crate) fn failure(error: Error) -> BoundaryFailure {
    match error {
        Error::Invalid(_) => BoundaryFailure::Invalid,
        Error::Io(error) => BoundaryFailure::Io(error.kind()),
        Error::Json(_) => BoundaryFailure::Json,
        Error::Toml(_) => BoundaryFailure::Toml,
    }
}
