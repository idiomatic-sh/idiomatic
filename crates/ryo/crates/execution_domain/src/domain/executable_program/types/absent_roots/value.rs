//! The file roots found absent on one file, decided once when the file binds its intents.
use super::super::absent_regions::value::AbsentRegions;
use crate::domain::executable_program::presence::types::RegionPresence;
use crate::domain::executable_program::types::{RegionCount, RegionId};

/// The file roots whose skip formula fails on one file: each publishes the empty table there without demanding an input,
/// and the file's reader counts never follow its edges. Decided once per file, when the file binds its intents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbsentRoots(AbsentRegions);

impl AbsentRoots {
    /// The roots a file found absent.
    pub fn decided(absent: AbsentRegions) -> Self {
        Self(absent)
    }
    /// Whether a region is one of them.
    pub fn presence(&self, region: RegionId) -> RegionPresence {
        self.0.presence(region)
    }
    /// How many there are.
    pub fn count(&self) -> RegionCount {
        self.0.count()
    }
}
