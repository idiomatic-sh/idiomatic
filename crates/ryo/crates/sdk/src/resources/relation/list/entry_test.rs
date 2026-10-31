//! What `relation list` promises its caller, one row of `CASES` for each case it has.

use super::entry::list;
use super::errors::ListError;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// How many relations are listed.
type Value = usize;
type Failure = ListError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("lists_relations"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(36),
}];

fn subject((): Input) -> Result<Value, Failure> {
    list().map(|report| report.relations.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
