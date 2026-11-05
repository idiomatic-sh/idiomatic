//! Cross publication orchestration failures, preserving port errors at the use-case edge.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CrossError {
    #[error(transparent)]
    Operating(#[from] ryo_execution::inbound::use_cases::execution::types::OperatingError),
    #[error(transparent)]
    Rules(#[from] ryo_values::domain::rule_requests::errors::RulesError),
    #[error(transparent)]
    Facts(#[from] ryo_execution::inbound::use_cases::execution::types::facts::FactsError),
    #[error(transparent)]
    Scheduler(#[from] crate::outbound::scheduler::errors::SchedulerError),
}
