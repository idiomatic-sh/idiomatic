//! The `set_rows` service's cases, on values alone.

use super::service::lower;
use super::types::{SetKind, SetLowering};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::equality_join::types::{EmptyRight, ProbeKey, ProbeMode};
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// The probe a set kind lowers to, its mode, key, repeats and empty-right answer.
fn probe_of(kind: SetKind) -> Option<(ProbeMode, ProbeKey, RepeatedRows, EmptyRight)> {
    match lower(kind, &[ColumnIndex(0)], &[ColumnIndex(1)]) {
        SetLowering::Probe(probe) => {
            Some((probe.mode, probe.key, probe.repeated, probe.empty_right))
        }
        SetLowering::Distinct => None,
    }
}

/// A union is each row once, of either side.
fn union_is_distinct() -> bool {
    lower(SetKind::Union, &[], &[]) == SetLowering::Distinct
}

/// An intersection keeps each distinct left row with a whole-row match on the right.
fn intersection_is_a_whole_row_semi_probe() -> bool {
    probe_of(SetKind::Intersection)
        == Some((
            ProbeMode::Semi,
            ProbeKey::Whole,
            RepeatedRows::Dropped,
            EmptyRight::KeyRead,
        ))
}

/// A difference keeps each distinct left row with no whole-row match on the right.
fn difference_is_a_whole_row_anti_probe() -> bool {
    probe_of(SetKind::Difference)
        == Some((
            ProbeMode::Anti,
            ProbeKey::Whole,
            RepeatedRows::Dropped,
            EmptyRight::KeyRead,
        ))
}

/// An antijoin keeps every left row whose key has no right row, repeats kept, and every left row of no right rows.
fn antijoin_is_a_keyed_anti_probe() -> bool {
    probe_of(SetKind::Antijoin)
        == Some((
            ProbeMode::Anti,
            ProbeKey::Columns {
                left: vec![ColumnIndex(0)],
                right: vec![ColumnIndex(1)],
            },
            RepeatedRows::Kept,
            EmptyRight::EveryLeftRow,
        ))
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_union_is_each_row_once"),
        word: Word::Succeeds,
        input: || union_is_distinct,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_intersection_is_a_whole_row_semi_probe"),
        word: Word::Succeeds,
        input: || intersection_is_a_whole_row_semi_probe,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_difference_is_a_whole_row_anti_probe"),
        word: Word::Succeeds,
        input: || difference_is_a_whole_row_anti_probe,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_antijoin_is_a_keyed_anti_probe_of_every_left_row"),
        word: Word::Succeeds,
        input: || antijoin_is_a_keyed_anti_probe,
        expect: Expect::Value(true),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
