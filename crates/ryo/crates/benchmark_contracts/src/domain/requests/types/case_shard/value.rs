//! A validated deterministic partition of a complete case inventory.
use crate::domain::reports::errors::ReportsError;
/// Zero-based position of a shard among the partitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShardIndex(
    /// Zero-based partition coordinate.
    pub usize,
);

/// Number of partitions of a case inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShardCount(
    /// Number of partitions.
    pub usize,
);

/// Validated zero-based shard coordinates.
/// ```compile_fail,E0451
/// use ryo_benchmark_contracts::domain::requests::types::case_shard::value::{CaseShard, ShardCount, ShardIndex};
/// let _ = CaseShard { index: ShardIndex(0), count: ShardCount(1) };
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseShard {
    index: ShardIndex,
    count: ShardCount,
}
impl CaseShard {
    /// Require a nonzero shard count and an index inside its bounds.
    /// # Errors
    /// Refuses a zero count or index at/above the count.
    pub fn new(index: ShardIndex, count: ShardCount) -> Result<Self, ReportsError> {
        if count.0 == 0 || index.0 >= count.0 {
            return Err(ReportsError::Invalid(
                crate::domain::requests::constants::INVALID_SHARD.into(),
            ));
        }
        Ok(Self { index, count })
    }
    /// Validated zero-based partition coordinate.
    #[must_use]
    pub const fn index(self) -> ShardIndex {
        self.index
    }
    /// Validated nonzero number of partitions.
    #[must_use]
    pub const fn count(self) -> ShardCount {
        self.count
    }
}
