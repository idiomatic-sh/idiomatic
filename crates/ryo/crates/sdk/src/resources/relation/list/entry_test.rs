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
    // 57 syntax facts, six manifest facts, and four engine facts.
    expect: Expect::Value(67),
}];

fn subject((): Input) -> Result<Value, Failure> {
    list().map(|report| report.relations.len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
