//! The `trait` item's cases: the text a row carries it as.

use super::super::interface::RefinementsItem as _;
use super::item::TraitItem;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Nothing: the item is asked about itself.
type Input = ();
/// The text the refinement is carried as.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("trait__is_carried_as_its_name"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value("trait"),
}];

fn subject(_input: Input) -> Value {
    TraitItem::TEXT
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
