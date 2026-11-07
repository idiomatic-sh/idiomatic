//! The `table_scratch` service's cases, on values alone.

use super::service::kind_has;
use super::types::{KindMask, KindSet};
use crate::domain::table_model::types::SetMembership;
use ryo_relations::domain::relations::types::KindBits;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// A prepared set of kinds, and the kind asked of it.
type Input = (KindSet, KindBits);
/// Whether the kind is in the set.
type Value = SetMembership;
/// Asking a set never fails.
type Failure = std::convert::Infallible;

/// A set holding the kinds `3` and `64`, one in each word of its mask.
fn two_words() -> KindSet {
    KindSet {
        bits: KindMask(vec![1 << 3, 1]),
        kinds: vec![KindBits(3), KindBits(64)],
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_of_the_first_word_is_in_the_set"),
        word: Word::Succeeds,
        input: || (two_words(), KindBits(3)),
        expect: Expect::Value(SetMembership::In),
    },
    Case {
        name: CaseName("a_kind_of_a_later_word_is_in_the_set"),
        word: Word::Succeeds,
        input: || (two_words(), KindBits(64)),
        expect: Expect::Value(SetMembership::In),
    },
    Case {
        name: CaseName("a_kind_the_mask_does_not_hold_is_out_of_the_set"),
        word: Word::Succeeds,
        input: || (two_words(), KindBits(4)),
        expect: Expect::Value(SetMembership::Out),
    },
    Case {
        name: CaseName("a_kind_beyond_the_mask_is_out_of_the_set"),
        word: Word::Succeeds,
        input: || (two_words(), KindBits(500)),
        expect: Expect::Value(SetMembership::Out),
    },
];

fn subject((set, kind): Input) -> Result<Value, Failure> {
    Ok(kind_has(&set, kind))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
