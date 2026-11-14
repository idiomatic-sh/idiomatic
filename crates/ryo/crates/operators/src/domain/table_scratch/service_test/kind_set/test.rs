//! The `table_scratch` service's cases, on values alone.
use super::fixtures::set;
use crate::domain::table_scratch::service::kind_has;
use crate::domain::table_scratch::types::SetMembership;
use ryo_relations::domain::relations::types::KindBits;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// A row's input: the kind asked of the set holding kind 2.
type Input = KindBits;

/// Whether the set holds the kind.
type Value = SetMembership;

/// Asking a set never fails.
type Failure = ();

/// The rows: a held kind, a kind whose bit is clear, and a kind beyond the mask.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_kind_is_in_the_set_by_its_bit"),
        word: Word::Succeeds,
        input: || KindBits(2),
        expect: Expect::Value(SetMembership::In),
    },
    Case {
        name: CaseName("a_kind_whose_bit_is_clear_is_out"),
        word: Word::Succeeds,
        input: || KindBits(3),
        expect: Expect::Value(SetMembership::Out),
    },
    Case {
        name: CaseName("a_kind_beyond_the_mask_is_out"),
        word: Word::Succeeds,
        input: || KindBits(130),
        expect: Expect::Value(SetMembership::Out),
    },
];

/// Asks the set for one kind.
fn subject(kind: Input) -> Result<Value, Failure> {
    Ok(kind_has(&set(), kind))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
