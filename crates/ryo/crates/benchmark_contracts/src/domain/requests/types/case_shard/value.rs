//! A validated deterministic partition of a complete case inventory.
use crate::domain::reports::errors::ReportsError;
/// Validated zero-based shard coordinates.
/// ```compile_fail
/// use ryo_benchmark_contracts::domain::requests::types::case_shard::value::CaseShard;
/// let _ = CaseShard { index: 0, count: 0 };
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseShard {
    index: usize,
    count: usize,
}
impl CaseShard {
    /// Require a nonzero shard count and an index inside its bounds.
    /// # Errors
    /// Refuses a zero count or index at/above the count.
    pub fn new(index: usize, count: usize) -> Result<Self, ReportsError> {
        if count == 0 || index >= count {
            return Err(ReportsError::Invalid(
                crate::domain::requests::constants::INVALID_SHARD.into(),
            ));
        }
        Ok(Self { index, count })
    }
    /// Validated zero-based partition coordinate.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
    /// Validated nonzero number of partitions.
    #[must_use]
    pub const fn count(self) -> usize {
        self.count
    }
}
