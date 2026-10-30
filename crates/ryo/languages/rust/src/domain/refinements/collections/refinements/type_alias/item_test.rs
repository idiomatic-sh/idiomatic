//! The `type-alias` item's cases: the text a row carries it as.

use super::super::interface::RefinementsItem as _;
use super::item::TypeAliasItem;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Nothing: the item is asked about itself.
type Input = ();
/// The text the refinement is carried as.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("type_alias_is_carried_as_its_name"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value("type-alias"),
}];

fn subject(_input: Input) -> Value {
    TypeAliasItem::TEXT
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
