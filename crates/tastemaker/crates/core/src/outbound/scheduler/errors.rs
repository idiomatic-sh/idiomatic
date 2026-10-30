//! Scheduler construction failures.
/// Why jobs could not be scheduled.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SchedulerError {
    /// The operating system refused the thread pool.
    #[error("scheduler unavailable: {reason}")]
    Unavailable {
        /// The underlying failure.
        reason: String,
    },
}
