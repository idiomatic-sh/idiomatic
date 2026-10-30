//! The `budget` service: free functions named by verb over the memory a run may use, with no IO and no ports.

use super::errors::BudgetError;
use ryo_values::domain::files::types::CorpusEntry;
use ryo_values::domain::runs::types::MemoryBytes;
use ryo_values::domain::runs::types::RuleId;

/// The bytes of the files together.
#[must_use]
pub(crate) fn repository_size(files: &[CorpusEntry]) -> MemoryBytes {
    MemoryBytes(files.iter().map(|file| file.size.0).sum())
}

/// Refuses a repository larger than the budget.
///
/// # Errors
///
/// `TooLarge` when the files together are more than the budget.
pub(crate) fn fit(size: MemoryBytes, budget: MemoryBytes) -> Result<(), BudgetError> {
    if size > budget {
        return Err(BudgetError::TooLarge { size, budget });
    }
    Ok(())
}

/// Refuses when the process holds more than the budget, naming the rules the run was working for.
///
/// # Errors
///
/// `Exceeded` when `used` is more than `budget`.
pub(crate) fn within(
    used: MemoryBytes,
    budget: MemoryBytes,
    plans: impl FnOnce() -> Vec<RuleId>,
) -> Result<(), BudgetError> {
    if used > budget {
        return Err(BudgetError::Exceeded {
            used,
            budget,
            plans: plans(),
        });
    }
    Ok(())
}
