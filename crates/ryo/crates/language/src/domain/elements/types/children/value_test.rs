//! The children of an element are its direct children, in order, each subtree skipped.
use super::value::Children;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::Position;

type Value = usize;

const CASES: &[Case<(), Value, ()>] = &[Case {
    name: CaseName("an_empty_range_has_no_child"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| {
        let elements = Elements::default();
        Ok(Children {
            elements: &elements,
            next: Position(1),
            last: Position(0),
        }
        .count())
    });
}
