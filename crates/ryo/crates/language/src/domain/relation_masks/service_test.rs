//! The `relation_masks` service's cases, on values alone.

use super::service::{add, none, places};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Count;

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

/// The relations added are listed once each, in ascending order, whatever the order they were added in.
fn added_relations_are_listed_in_order() -> usize {
    let mut mask = none();
    for place in [Count(65), Count(3), Count(64), Count(3), Count(0)] {
        add(&mut mask, place);
    }
    usize::from(
        places(&mask).collect::<Vec<_>>() == [Count(0), Count(3), Count(64), Count(65)]
            && places(&none()).next().is_none(),
    )
}

/// A relation past the catalog is not added.
fn a_relation_past_the_catalog_is_not_added() -> usize {
    let mut mask = none();
    add(&mut mask, Count(usize::MAX));
    usize::from(places(&mask).next().is_none())
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("added_relations_are_listed_in_order"),
        word: Word::Succeeds,
        input: || added_relations_are_listed_in_order,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_relation_past_the_catalog_is_not_added"),
        word: Word::Succeeds,
        input: || a_relation_past_the_catalog_is_not_added,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
