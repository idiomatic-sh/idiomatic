//! What every `digest` adapter must pass.

use ryo_execution::inbound::use_cases::execution::types::digest::DigestPort;
use ryo_execution::inbound::use_cases::execution::types::digest::{DigestBytes, DigestWords};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the contract is given: which question to ask of the adapter.
type Input = Question;
/// What a row of the contract expects when it succeeds: whether the answer was the right one.
type Value = bool;
/// What a row of the contract expects when it is refused.
type Failure = ();

/// A question the contract asks of an adapter.
#[derive(Clone, Copy, Debug)]
pub enum Question {
    /// The same bytes come to the same digest.
    SameBytesSameDigest,
    /// Bytes that differ come to digests that differ.
    DifferentBytesDifferentDigests,
    /// Words come to what their little-endian bytes come to.
    WordsAreTheirBytes,
    /// Digests combined in another order come to another digest.
    CombiningDependsOnOrder,
}

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_same_bytes_come_to_the_same_digest"),
        word: Word::Succeeds,
        input: || Question::SameBytesSameDigest,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("bytes_that_differ_come_to_digests_that_differ"),
        word: Word::Succeeds,
        input: || Question::DifferentBytesDifferentDigests,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("words_come_to_what_their_bytes_come_to"),
        word: Word::Succeeds,
        input: || Question::WordsAreTheirBytes,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("combining_depends_on_the_order"),
        word: Word::Succeeds,
        input: || Question::CombiningDependsOnOrder,
        expect: Expect::Value(true),
    },
];

/// Asks one question of the adapter.
fn answer(adapter: &impl DigestPort, question: Question) -> Value {
    match question {
        Question::SameBytesSameDigest => {
            adapter.digest(DigestBytes(b"one")) == adapter.digest(DigestBytes(b"one"))
        }
        Question::DifferentBytesDifferentDigests => {
            adapter.digest(DigestBytes(b"one")) != adapter.digest(DigestBytes(b"two"))
        }
        Question::WordsAreTheirBytes => {
            adapter.digest_words(DigestWords(&[1, 2]))
                == adapter.digest(DigestBytes(&[
                    1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
                ]))
        }
        Question::CombiningDependsOnOrder => {
            let (first, second) = (
                adapter.digest(DigestBytes(b"one")),
                adapter.digest(DigestBytes(b"two")),
            );
            adapter.combine(&[first, second]) != adapter.combine(&[second, first])
        }
    }
}

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl DigestPort) {
    run(CASES, Word::Succeeds, |question| {
        Ok(answer(adapter, question))
    });
}
