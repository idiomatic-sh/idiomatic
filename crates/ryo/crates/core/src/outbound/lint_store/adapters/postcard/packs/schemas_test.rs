//! What the packs document of the store accepts: packs read back as they were written, and bytes that are no packs read as none.

use super::{packs_bytes, packs_of};

use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Bytes;
use ryo_values::domain::stores::types::{PackKey, StoredPack};

/// What a row of the cases reads back.
#[derive(Clone, Copy, Debug)]
enum Written {
    /// Two packs under two keys, one of them with no byte.
    TwoPacks,
    /// Bytes that are no packs.
    NoPacks,
}

type Input = Written;
/// How many packs read back, and whether they are the ones written.
type Value = (usize, bool);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("packs_read_back_as_they_were_written"),
        word: Word::Succeeds,
        input: || Written::TwoPacks,
        expect: Expect::Value((2, true)),
    },
    Case {
        name: CaseName("bytes_that_are_no_packs_read_as_none"),
        word: Word::Succeeds,
        input: || Written::NoPacks,
        expect: Expect::Value((0, false)),
    },
];

fn subject(written: Input) -> Result<Value, Failure> {
    let packs = vec![
        StoredPack {
            key: PackKey([1; 32]),
            bytes: Bytes(vec![7, 0, 255]),
        },
        StoredPack {
            key: PackKey([2; 32]),
            bytes: Bytes(Vec::new()),
        },
    ];
    let bytes = match written {
        Written::TwoPacks => packs_bytes(&packs).map_err(|_| ())?,
        Written::NoPacks => vec![255; 3],
    };
    let read = packs_of(&bytes);
    Ok((read.len(), read == packs))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
