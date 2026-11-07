//! The `budget` concept's failures.

use ryo_values::domain::runs::types::MemoryBytes;
use ryo_values::domain::runs::types::RuleId;

/// Why a run is refused for the memory it needs.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum BudgetError {
    /// The repository does not fit the memory budget.
    #[error("the repository is {} bytes and the budget is {}", size.0, budget.0)]
    TooLarge {
        /// The bytes of the repository.
        size: MemoryBytes,
        /// The bytes the run may use.
        budget: MemoryBytes,
    },
    /// The process holds more memory than the budget while it runs.
    #[error("the run holds {} bytes and the budget is {}", used.0, budget.0)]
    Exceeded {
        /// The bytes the process holds.
        used: MemoryBytes,
        /// The bytes the run may use.
        budget: MemoryBytes,
        /// The rules the run was working for: the ones that govern the file, or the one whose finish it was making.
        plans: Vec<RuleId>,
    },
}
