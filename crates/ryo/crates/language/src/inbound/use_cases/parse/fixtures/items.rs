//! What every item of a language's `relations` collection must pass, whatever the language: the rows its collection contract runs.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::answers::types::ReadItem;
use crate::domain::relations::collections::relations::interface::RelationsItem;

/// What a shared row is given: nothing, the item is asked about itself.
type Input = ();
/// Whether the relation the item writes has columns.
type Value = bool;
/// What a shared row expects when it is refused.
type Failure = ();

/// The shared rows: a name, one case word, an input and the expected value or error variant. Every item runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("each_item_writes_a_relation_with_columns"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Runs the shared rows against one item.
///
/// # Panics
///
/// When the item breaks the contract.
pub fn check<Item>()
where
    Item: ReadItem,
{
    run(CASES, Word::Succeeds, |()| {
        Ok(!<Item::Relation as RelationsItem>::columns().is_empty())
    });
}

#[cfg(test)]
#[path = "items_test.rs"]
mod tests;
