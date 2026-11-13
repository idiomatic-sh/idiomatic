//! The blake3 adapter of the digest port: the core's contract rows run through it, then the cases specific to it.

use super::adapter::DigestBlake3Adapter;
use crate::outbound::digest::fixtures::contract::check;
use crate::outbound::digest::interface::DigestPort as _;
use crate::outbound::digest::types::DigestBytes;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

type Input = ();
/// The digest the adapter answers.
type Value = [u8; 16];
type Failure = ();

/// The rows of what is specific to this adapter: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("empty_bytes_come_to_the_blake3_hash_of_nothing"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40, 0x4d, 0xea, 0x36, 0xdc, 0xc9,
        0x49,
    ]),
}];

/// Calls the adapter on one input, for its own rows.
fn subject(_input: Input) -> Value {
    DigestBlake3Adapter.digest(DigestBytes(&[])).0
}

#[test]
fn succeeds() {
    check(&DigestBlake3Adapter);
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
