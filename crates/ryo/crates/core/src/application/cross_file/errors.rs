//! Cross publication orchestration failures, preserving port errors at the use-case edge.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CrossError {
    #[error("{0}")]
    Contract(String),
    #[error(transparent)]
    Operating(#[from] crate::domain::operating::errors::OperatingError),
    #[error(transparent)]
    Rules(#[from] crate::domain::rules::errors::RulesError),
    #[error(transparent)]
    Facts(#[from] crate::outbound::facts::errors::FactsError),
    #[error(transparent)]
    Scheduler(#[from] crate::outbound::scheduler::errors::SchedulerError),
}
