//! The blake3 adapter of the digest port: the contract, then the fingerprint the first Tastemaker gave a known finding.

use super::adapter::DigestBlake3Adapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::{Identifying, Ordinal};
use crate::outbound::digest::fixtures::contract::check;
use crate::outbound::digest::interface::DigestPort as _;

type Input = u64;
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_first_with_its_parts_hashes_no_ordinal"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_later_one_with_the_same_parts_differs"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value(false),
    },
];

fn subject(ordinal: Input) -> Value {
    let identifying = |count| Identifying {
        parts: crate::domain::findings::types::IdentityParts([
            "rule".into(),
            "path".into(),
            "line".into(),
        ]),
        ordinal: Ordinal(count),
    };
    DigestBlake3Adapter.fingerprint(&identifying(ordinal))
        == DigestBlake3Adapter.fingerprint(&identifying(0))
}

#[test]
fn succeeds() {
    check(&DigestBlake3Adapter);
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
