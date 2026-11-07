//! The `scheduler` port's failures.

/// Why the `scheduler` port failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SchedulerError {
    /// The workers could not be started.
    #[error("cannot start workers: {reason}")]
    Unavailable {
        /// What went wrong.
        reason: String,
    },
    /// The caller gave no worker to run the jobs on.
    #[error("no worker was given to run the jobs on")]
    NoWorkers,
}
