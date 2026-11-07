//! The `syntax` service's cases.

use super::service::{kind_bits, kind_name, kind_of, kinds};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Nothing: the service is asked about every kind.
type Input = ();
/// Whether every kind's number names it again, and how many kinds there are.
type Value = (bool, usize);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_kind_is_found_by_its_number_and_has_a_name"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((true, 34)),
}];

fn subject((): Input) -> Value {
    let round =
        kinds().all(|kind| kind_of(kind_bits(kind)) == kind && !kind_name(kind).0.is_empty());
    (round, kinds().count())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| {
        Ok::<Value, Failure>(subject(input))
    });
}
