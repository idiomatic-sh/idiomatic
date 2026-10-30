//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use super::super::entry::list;
use super::super::errors::ListError;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// How many operators are listed without a name.
type Value = usize;
type Failure = ListError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_listed_operator_has_a_name"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

/// Lists the operators and counts those whose name is blank.
fn subject((): Input) -> Result<Value, Failure> {
    list().map(|report| {
        report
            .operators
            .iter()
            .filter(|each| each.name.0.trim().is_empty())
            .count()
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
