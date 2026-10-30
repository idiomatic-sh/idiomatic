//! Failures preparing a report through injected resources.
/// A required reporting operation failed.
#[derive(Debug, thiserror::Error)]
pub enum ReportingError {
    /// The reporting work could not be scheduled.
    #[error(transparent)]
    Scheduler(#[from] crate::outbound::scheduler::errors::SchedulerError),
}
