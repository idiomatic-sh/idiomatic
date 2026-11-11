//! All nine real families reach the common admitted owner.
use super::fixtures::{CASES, subject};
use crate::cases::{Word, run};
use ryo_derived::domain::request::types::DerivedKind;
#[test]
fn succeeds() {
    let mut declared = CASES.iter().map(|case| (case.input)()).collect::<Vec<_>>();
    declared.sort_by_key(|kind| kind.ordinal());
    assert_eq!(
        declared,
        DerivedKind::ALL,
        "the independent case table must contain every authoritative family exactly once"
    );
    run(CASES, Word::Succeeds, subject);
}
