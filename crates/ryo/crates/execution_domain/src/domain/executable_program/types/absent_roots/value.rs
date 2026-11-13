//! The file roots found absent on one file, decided once when the file binds its intents.
use super::super::absent_regions::value::AbsentRegions;
use crate::domain::executable_program::presence::types::RegionPresence;
use crate::domain::executable_program::types::{RegionCount, RegionId};

/// The file roots whose skip formula fails on one file: each publishes the empty table there without demanding an input,
/// and the file's reader counts never follow its edges. Decided once per file, when the file binds its intents.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::presence::types::RegionPresence;
/// use ryo_execution_domain::domain::executable_program::types::absent_regions::value::AbsentRegions;
/// use ryo_execution_domain::domain::executable_program::types::absent_roots::value::AbsentRoots;
/// use ryo_execution_domain::domain::executable_program::types::{RegionCount, RegionId};
/// let mut regions = AbsentRegions::none(RegionCount(4));
/// regions.insert(RegionId(1));
/// let absent = AbsentRoots::decided(regions);
/// assert_eq!(absent.presence(RegionId(1)), RegionPresence::Absent);
/// assert_eq!(absent.count().0, 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbsentRoots(AbsentRegions);

impl AbsentRoots {
    /// The roots a file found absent.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::executable_program::presence::types::RegionPresence;
    /// use ryo_execution_domain::domain::executable_program::types::absent_regions::value::AbsentRegions;
    /// use ryo_execution_domain::domain::executable_program::types::absent_roots::value::AbsentRoots;
    /// use ryo_execution_domain::domain::executable_program::types::{RegionCount, RegionId};
    /// let mut regions = AbsentRegions::none(RegionCount(4));
    /// regions.insert(RegionId(1));
    /// let absent = AbsentRoots::decided(regions);
    /// assert_eq!(absent.presence(RegionId(1)), RegionPresence::Absent);
    /// assert_eq!(absent.count().0, 1);
    /// ```
    pub fn decided(absent: AbsentRegions) -> Self {
        Self(absent)
    }
    /// Whether a region is one of them.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::executable_program::presence::types::RegionPresence;
    /// use ryo_execution_domain::domain::executable_program::types::absent_regions::value::AbsentRegions;
    /// use ryo_execution_domain::domain::executable_program::types::absent_roots::value::AbsentRoots;
    /// use ryo_execution_domain::domain::executable_program::types::{RegionCount, RegionId};
    /// let mut regions = AbsentRegions::none(RegionCount(4));
    /// regions.insert(RegionId(1));
    /// let absent = AbsentRoots::decided(regions);
    /// assert_eq!(absent.presence(RegionId(1)), RegionPresence::Absent);
    /// assert_eq!(absent.count().0, 1);
    /// ```
    pub fn presence(&self, region: RegionId) -> RegionPresence {
        self.0.presence(region)
    }
    /// How many there are.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::executable_program::presence::types::RegionPresence;
    /// use ryo_execution_domain::domain::executable_program::types::absent_regions::value::AbsentRegions;
    /// use ryo_execution_domain::domain::executable_program::types::absent_roots::value::AbsentRoots;
    /// use ryo_execution_domain::domain::executable_program::types::{RegionCount, RegionId};
    /// let mut regions = AbsentRegions::none(RegionCount(4));
    /// regions.insert(RegionId(1));
    /// let absent = AbsentRoots::decided(regions);
    /// assert_eq!(absent.presence(RegionId(1)), RegionPresence::Absent);
    /// assert_eq!(absent.count().0, 1);
    /// ```
    pub fn count(&self) -> RegionCount {
        self.0.count()
    }
}
