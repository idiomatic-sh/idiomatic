//! What `operator list` promises its caller, one row of `CASES` for each case it has.

use super::entry::list;
use super::errors::ListError;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// How many operators are listed.
type Value = usize;
type Failure = ListError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("lists_all_operators"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(21),
}];

fn subject((): Input) -> Result<Value, Failure> {
    list().map(|report| report.operators.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
