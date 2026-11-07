//! The `execution` port's failures.

/// Why an operation of `execution` failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ExecutionError {
    /// The platform reported neither an exit code nor a supported signal.
    #[error("child status has neither an exit code nor a supported signal")]
    UnobservedExit,
}
