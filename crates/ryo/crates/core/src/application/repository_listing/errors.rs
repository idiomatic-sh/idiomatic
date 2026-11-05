//! Failures while opening the repository inventory and prior store.
use crate::domain::budget::errors::BudgetError;
use crate::outbound::{
    corpus::errors::CorpusError, lint_store::errors::LintStoreError, memory::errors::MemoryError,
    scheduler::errors::SchedulerError,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RepositoryListingError {
    #[error(transparent)]
    Corpus(#[from] CorpusError),
    #[error(transparent)]
    Store(#[from] LintStoreError),
    #[error(transparent)]
    Memory(#[from] MemoryError),
    #[error(transparent)]
    Budget(#[from] BudgetError),
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
}
