//! What `query describe` promises its caller, one row of `CASES` for each case it has.

use super::entry::describe;
use super::errors::DescribeError;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// Whether the first production is the whole program and the last a word.
type Value = (bool, bool);
type Failure = DescribeError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("describes_the_grammar_from_the_program_down_to_a_word"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((true, true)),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let report = describe()?;
    let named = |production: Option<&super::types::Production>, name: &str| {
        production.is_some_and(|production| &*production.name.0 == name)
    };
    Ok((
        named(report.productions.first(), "program"),
        named(report.productions.last(), "word"),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
