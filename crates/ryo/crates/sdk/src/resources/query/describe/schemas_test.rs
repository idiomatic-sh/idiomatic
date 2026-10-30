//! What the schemas of this entry accept and refuse, one row of `CASES` for each input.

use super::super::entry::describe;
use super::super::errors::DescribeError;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// How many productions come across with a blank name, form or example.
type Value = usize;
type Failure = DescribeError;

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_production_keeps_its_name_form_and_example"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(0),
}];

/// Describes the grammar and counts the productions with a blank part.
fn subject((): Input) -> Result<Value, Failure> {
    describe().map(|report| {
        report
            .productions
            .iter()
            .filter(|each| {
                each.name.0.trim().is_empty()
                    || each.form.0.trim().is_empty()
                    || each.example.0.trim().is_empty()
            })
            .count()
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
