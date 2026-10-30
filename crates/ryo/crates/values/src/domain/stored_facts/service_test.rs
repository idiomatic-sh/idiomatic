//! The `stored_facts` service's cases, on values alone.

use super::service::blankness_of;
use super::types::{Digest, DigestBlankness};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// The blank digest.
    Blank,
    /// A digest that came from bytes.
    Made,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Whether the digest is blank.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_blank_digest_is_blank"),
        word: Word::Succeeds,
        input: || Scenario::Blank,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_digest_of_bytes_is_not_blank"),
        word: Word::Succeeds,
        input: || Scenario::Made,
        expect: Expect::Value(0),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let digest = match scenario {
        Scenario::Blank => Digest([0; 16]),
        Scenario::Made => Digest([1; 16]),
    };
    Ok(usize::from(blankness_of(&digest) == DigestBlankness::Blank))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
