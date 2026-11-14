//! Regions whose skip formula fails on one file, one bit each: the set's representation is its own.
use crate::domain::interpreted_program::presence::types::RegionPresence;
use crate::domain::interpreted_program::types::{RegionCount, RegionId};

/// Regions whose skip formula fails on one file, one bit each.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::interpreted_program::presence::types::RegionPresence;
/// use ryo_execution_domain::domain::interpreted_program::types::absent_regions::value::AbsentRegions;
/// use ryo_execution_domain::domain::interpreted_program::types::{RegionCount, RegionId};
/// let mut absent = AbsentRegions::none(RegionCount(4));
/// absent.insert(RegionId(2));
/// assert_eq!(absent.presence(RegionId(2)), RegionPresence::Absent);
/// assert_eq!(absent.count().0, 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbsentRegions(Box<[u64]>);

impl AbsentRegions {
    /// No region, over a program of `regions` regions.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::interpreted_program::presence::types::RegionPresence;
    /// use ryo_execution_domain::domain::interpreted_program::types::absent_regions::value::AbsentRegions;
    /// use ryo_execution_domain::domain::interpreted_program::types::{RegionCount, RegionId};
    /// let mut absent = AbsentRegions::none(RegionCount(4));
    /// absent.insert(RegionId(2));
    /// assert_eq!(absent.presence(RegionId(2)), RegionPresence::Absent);
    /// assert_eq!(absent.count().0, 1);
    /// ```
    pub fn none(regions: RegionCount) -> Self {
        Self(vec![0; regions.0.div_ceil(64)].into_boxed_slice())
    }
    /// Adds a region; one past the program is ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::interpreted_program::presence::types::RegionPresence;
    /// use ryo_execution_domain::domain::interpreted_program::types::absent_regions::value::AbsentRegions;
    /// use ryo_execution_domain::domain::interpreted_program::types::{RegionCount, RegionId};
    /// let mut absent = AbsentRegions::none(RegionCount(4));
    /// absent.insert(RegionId(2));
    /// assert_eq!(absent.presence(RegionId(2)), RegionPresence::Absent);
    /// assert_eq!(absent.count().0, 1);
    /// ```
    pub fn insert(&mut self, region: RegionId) {
        if let Some(word) = self.0.get_mut(region.0 / 64) {
            *word |= 1 << (region.0 % 64);
        }
    }
    /// Whether the set holds a region.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::interpreted_program::presence::types::RegionPresence;
    /// use ryo_execution_domain::domain::interpreted_program::types::absent_regions::value::AbsentRegions;
    /// use ryo_execution_domain::domain::interpreted_program::types::{RegionCount, RegionId};
    /// let mut absent = AbsentRegions::none(RegionCount(4));
    /// absent.insert(RegionId(2));
    /// assert_eq!(absent.presence(RegionId(2)), RegionPresence::Absent);
    /// assert_eq!(absent.count().0, 1);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::interpreted_program::presence::types::RegionPresence;
    /// use ryo_execution_domain::domain::interpreted_program::types::absent_regions::value::AbsentRegions;
    /// use ryo_execution_domain::domain::interpreted_program::types::{RegionCount, RegionId};
    /// let mut absent = AbsentRegions::none(RegionCount(4));
    /// absent.insert(RegionId(2));
    /// assert_eq!(absent.presence(RegionId(2)), RegionPresence::Absent);
    /// assert_eq!(absent.count().0, 1);
    /// ```
    pub fn count(&self) -> RegionCount {
        RegionCount(self.0.iter().map(|word| word.count_ones() as usize).sum())
    }
}
