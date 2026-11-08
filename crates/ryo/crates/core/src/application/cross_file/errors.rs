//! The `cross_file` operation's failures: cross publication orchestration, preserving port errors at the use-case edge.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CrossFileError {
    #[error(transparent)]
    Preparation(#[from] super::preparation::errors::BatchPreparationError),
    #[error(transparent)]
    Budget(#[from] crate::domain::budget::errors::BudgetError),
    #[error(transparent)]
    Memory(#[from] crate::outbound::memory::errors::MemoryError),
    #[error(transparent)]
    Operating(#[from] ryo_execution::inbound::use_cases::execution::interface::OperatingError),
    #[error(transparent)]
    Rules(#[from] ryo_values::domain::rule_requests::errors::RuleRequestsError),
    #[error(transparent)]
    Facts(#[from] ryo_execution_runtime::outbound::facts::errors::FactsError),
    #[error(transparent)]
    Scheduler(#[from] crate::outbound::scheduler::errors::SchedulerError),
}
