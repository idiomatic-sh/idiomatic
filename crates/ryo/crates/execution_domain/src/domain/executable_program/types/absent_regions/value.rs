//! Regions whose skip formula fails on one file, one bit each: the set's representation is its own.
use crate::domain::executable_program::presence::types::RegionPresence;
use crate::domain::executable_program::types::{RegionCount, RegionId};

/// Regions whose skip formula fails on one file, one bit each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbsentRegions(Box<[u64]>);

impl AbsentRegions {
    /// No region, over a program of `regions` regions.
    pub fn none(regions: RegionCount) -> Self {
        Self(vec![0; regions.0.div_ceil(64)].into_boxed_slice())
    }
    /// Adds a region; one past the program is ignored.
    pub fn insert(&mut self, region: RegionId) {
        if let Some(word) = self.0.get_mut(region.0 / 64) {
            *word |= 1 << (region.0 % 64);
        }
    }
    /// Whether the set holds a region.
    pub fn presence(&self, region: RegionId) -> RegionPresence {
        if self
            .0
            .get(region.0 / 64)
            .is_some_and(|word| word & (1 << (region.0 % 64)) != 0)
        {
            RegionPresence::Absent
        } else {
            RegionPresence::Present
        }
    }
    /// How many regions it holds.
    pub fn count(&self) -> RegionCount {
        RegionCount(self.0.iter().map(|word| word.count_ones() as usize).sum())
    }
}
