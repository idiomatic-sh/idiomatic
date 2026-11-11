//! The absent roots of a file answer from the regions decided for it.
use super::super::absent_regions::value::AbsentRegions;
use super::value::AbsentRoots;
use crate::domain::executable_program::presence::types::RegionPresence;
use crate::domain::executable_program::types::{RegionCount, RegionId};

#[test]
fn decided_roots_answer_from_their_regions() {
    let mut regions = AbsentRegions::none(RegionCount(4));
    regions.insert(RegionId(3));
    let absent = AbsentRoots::decided(regions);
    assert_eq!(absent.presence(RegionId(3)), RegionPresence::Absent);
    assert_eq!(absent.presence(RegionId(0)), RegionPresence::Present);
    assert_eq!(absent.count().0, 1);
    assert_eq!(AbsentRoots::default().count().0, 0);
}
