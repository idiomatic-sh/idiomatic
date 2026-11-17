//! The `ReadSet` value's cases.

use super::value::ReadSet;
use crate::domain::trust::types::SetSharing;

#[test]
fn a_set_and_its_clone_are_one_stored_set() {
    let set = ReadSet::default();
    assert_eq!(set.sharing(&set.clone()), SetSharing::Shared);
}
