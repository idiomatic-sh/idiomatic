//! What every item of the `operators` collection must pass.

use crate::domain::operator::constants::describe;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plans::types::QueryOperator;

/// What a shared row is given: nothing, the item is asked about itself.
type Input = ();
/// Whether the item is described well.
type Value = bool;
/// What a shared row expects when it is refused.
type Failure = ();

/// The shared rows: a name, one case word, an input and the expected value or error variant. Every item runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("each_operator_describes_itself"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(true),
}];

/// Runs the shared rows against one item.
///
/// # Panics
///
/// When the item breaks the contract.
pub(crate) fn check(operator: QueryOperator) {
    run(CASES, Word::Succeeds, |()| {
        let description = describe(operator);
        Ok(!description.name.0.is_empty()
            && !description.summary.0.is_empty()
            && description.example.0.contains("example"))
    });
}
