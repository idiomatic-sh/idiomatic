//! Failures of reading what the last run kept beside the repository it lists.
use crate::domain::budget::errors::BudgetError;
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::memory::errors::MemoryError;
use crate::outbound::scheduler::errors::SchedulerError;

/// Why the repository and what the last run kept could not be read.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum StoringError {
    /// The files under the root could not be listed.
    #[error(transparent)]
    Corpus(#[from] CorpusError),
    /// What the last run kept could not be read.
    #[error(transparent)]
    Store(#[from] LintStoreError),
    /// The memory the run may hold could not be read.
    #[error(transparent)]
    Memory(#[from] MemoryError),
    /// The repository does not fit the memory the run may hold.
    #[error(transparent)]
    Budget(#[from] BudgetError),
    /// The two listing jobs could not be run.
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
}
