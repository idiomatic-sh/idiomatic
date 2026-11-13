//! A region set holds the regions inserted into it, ignores one past the program, and counts its members.
use super::value::AbsentRegions;
use crate::domain::executable_program::presence::types::RegionPresence;
use crate::domain::executable_program::types::{RegionCount, RegionId};

#[test]
fn an_inserted_region_is_absent_and_counted() {
    let mut regions = AbsentRegions::none(RegionCount(70));
    regions.insert(RegionId(1));
    regions.insert(RegionId(65));
    regions.insert(RegionId(200));
    assert_eq!(regions.presence(RegionId(1)), RegionPresence::Absent);
    assert_eq!(regions.presence(RegionId(65)), RegionPresence::Absent);
    assert_eq!(regions.presence(RegionId(2)), RegionPresence::Present);
    assert_eq!(regions.presence(RegionId(200)), RegionPresence::Present);
    assert_eq!(regions.count().0, 2);
}
