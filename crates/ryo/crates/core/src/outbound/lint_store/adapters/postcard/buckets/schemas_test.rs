//! What the schemas of the words of a bucket accept: the words of a bucket read back as they were written.

use super::{bucket_bytes, bucket_from};
use ryo_values::domain::stored_facts::types::{Bucket, PackedWords};
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::{StoredFrontierProof, StoredWords};

use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases asks of the schemas.
type Input = ();
/// Whether the schemas answered as the row expects.
type Value = bool;
/// What a row of the cases expects when it is refused.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_words_of_a_bucket_read_back_as_they_were_written"),
        word: Word::Succeeds,
        input: || (),
        expect: Expect::Value(true),
    },
];

/// Writes the words of one bucket and reads them back with the proof that accompanies them.
fn subject() -> Value {
    let proof = crate::outbound::lint_store::fixtures::proof();
    let words = StoredWords {
        file: StoredFileNumber(2),
        bucket: Bucket(7),
        proof: proof.clone(),
        words: PackedWords(vec![1, 2, 3]),
    };
    let proofs = [StoredFrontierProof {
        file: StoredFileNumber(2),
        bucket: Bucket(7),
        proof,
    }];
    bucket_bytes(&[&words])
        .ok()
        .and_then(|bytes| bucket_from(Bucket(7), &bytes, &proofs))
        == Some(vec![words])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| Ok(subject()));
}
