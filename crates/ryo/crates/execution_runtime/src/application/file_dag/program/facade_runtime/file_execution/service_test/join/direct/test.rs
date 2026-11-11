//! Direct catalog-relation Join routes retain their real acquired representation.
use super::fixtures::{Attempt, Joined, Keying, Refused, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationKind;
use ryo_relations::domain::relations::types::{Flag, Integer, Value as Cell};
use ryo_values::domain::rows::errors::RowsError;
use std::borrow::Cow;

type Input = Keying;
type Value = Joined;
type Failure = Refused;

/// One joined row: the payload, the left key, the right flag, the right key and the payload.
const fn row(payload: i64, left: i64, raised: bool, right: i64) -> [Cell; 5] {
    let flag = if raised { Flag::Raised } else { Flag::Lowered };
    [
        Cell::Number(Integer(payload)),
        Cell::Number(Integer(left)),
        Cell::Flag(flag),
        Cell::Number(Integer(right)),
        Cell::Number(Integer(payload)),
    ]
}

/// A run over `supplied` that answers what the computed run answered.
const fn attempt(
    supplied: RepresentationKind,
    delivered: bool,
    acquired: Option<RepresentationKind>,
    index_rows: usize,
    prefix_rows: usize,
) -> Attempt {
    Attempt {
        supplied,
        delivered,
        acquired,
        index: (index_rows, 0, 0),
        prefix_rows,
        same_answer: true,
    }
}

/// The computed and stored runs, each refused before acquiring an index.
const REFUSED: [Attempt; 4] = [
    attempt(RepresentationKind::Computed, false, None, 0, 0),
    attempt(RepresentationKind::Computed, true, None, 0, 0),
    attempt(RepresentationKind::Stored, false, None, 0, 0),
    attempt(RepresentationKind::Stored, true, None, 0, 0),
];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "stored_and_compact_direct_join_composite_reordered_empty_and_unlike_keys_do_not_materialize_rhs",
        ),
        word: Word::Succeeds,
        input: || Keying::Single,
        expect: Expect::Value(Joined {
            rows: Cow::Borrowed(&[
                row(10, 1, true, 1),
                row(11, 1, true, 1),
                row(12, 1, false, 1),
                row(10, 1, true, 1),
                row(11, 1, true, 1),
                row(12, 1, false, 1),
                row(10, 1, true, 1),
                row(11, 1, true, 1),
                row(12, 1, false, 1),
            ]),
            attempts: [
                attempt(
                    RepresentationKind::Stored,
                    false,
                    Some(RepresentationKind::Stored),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Stored,
                    true,
                    Some(RepresentationKind::Stored),
                    5,
                    9,
                ),
                attempt(
                    RepresentationKind::Compact,
                    false,
                    Some(RepresentationKind::Compact),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Compact,
                    true,
                    Some(RepresentationKind::Compact),
                    5,
                    9,
                ),
            ],
        }),
    },
    Case {
        name: CaseName("a_composite_key_joins_the_flag_and_number_together"),
        word: Word::Succeeds,
        input: || Keying::Composite,
        expect: Expect::Value(Joined {
            rows: Cow::Borrowed(&[
                row(10, 1, true, 1),
                row(11, 1, true, 1),
                row(12, 1, false, 1),
                row(10, 1, true, 1),
                row(11, 1, true, 1),
            ]),
            attempts: [
                attempt(
                    RepresentationKind::Stored,
                    false,
                    Some(RepresentationKind::Stored),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Stored,
                    true,
                    Some(RepresentationKind::Stored),
                    5,
                    5,
                ),
                attempt(
                    RepresentationKind::Compact,
                    false,
                    Some(RepresentationKind::Compact),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Compact,
                    true,
                    Some(RepresentationKind::Compact),
                    5,
                    5,
                ),
            ],
        }),
    },
    Case {
        name: CaseName("a_reordered_composite_key_joins_the_same_rows"),
        word: Word::Succeeds,
        input: || Keying::Reordered,
        expect: Expect::Value(Joined {
            rows: Cow::Borrowed(&[
                row(10, 1, true, 1),
                row(11, 1, true, 1),
                row(12, 1, false, 1),
                row(10, 1, true, 1),
                row(11, 1, true, 1),
            ]),
            attempts: [
                attempt(
                    RepresentationKind::Stored,
                    false,
                    Some(RepresentationKind::Stored),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Stored,
                    true,
                    Some(RepresentationKind::Stored),
                    5,
                    5,
                ),
                attempt(
                    RepresentationKind::Compact,
                    false,
                    Some(RepresentationKind::Compact),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Compact,
                    true,
                    Some(RepresentationKind::Compact),
                    5,
                    5,
                ),
            ],
        }),
    },
    Case {
        name: CaseName("no_keys_join_every_left_row_to_every_right_row"),
        word: Word::Succeeds,
        input: || Keying::Empty,
        expect: Expect::Value(Joined {
            rows: Cow::Borrowed(&[
                row(10, 1, true, 1),
                row(20, 1, false, 2),
                row(11, 1, true, 1),
                row(30, 1, true, 3),
                row(12, 1, false, 1),
                row(10, 1, true, 1),
                row(20, 1, false, 2),
                row(11, 1, true, 1),
                row(30, 1, true, 3),
                row(12, 1, false, 1),
                row(10, 1, true, 1),
                row(20, 1, false, 2),
                row(11, 1, true, 1),
                row(30, 1, true, 3),
                row(12, 1, false, 1),
                row(10, 9, true, 1),
                row(20, 9, false, 2),
                row(11, 9, true, 1),
                row(30, 9, true, 3),
                row(12, 9, false, 1),
            ]),
            attempts: [
                attempt(
                    RepresentationKind::Stored,
                    false,
                    Some(RepresentationKind::Stored),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Stored,
                    true,
                    Some(RepresentationKind::Stored),
                    5,
                    20,
                ),
                attempt(
                    RepresentationKind::Compact,
                    false,
                    Some(RepresentationKind::Compact),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Compact,
                    true,
                    Some(RepresentationKind::Compact),
                    5,
                    20,
                ),
            ],
        }),
    },
    Case {
        name: CaseName("unlike_key_kinds_join_no_rows"),
        word: Word::Succeeds,
        input: || Keying::Unlike,
        expect: Expect::Value(Joined {
            rows: Cow::Borrowed(&[]),
            attempts: [
                attempt(
                    RepresentationKind::Stored,
                    false,
                    Some(RepresentationKind::Stored),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Stored,
                    true,
                    Some(RepresentationKind::Stored),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Compact,
                    false,
                    Some(RepresentationKind::Compact),
                    5,
                    0,
                ),
                attempt(
                    RepresentationKind::Compact,
                    true,
                    Some(RepresentationKind::Compact),
                    5,
                    0,
                ),
            ],
        }),
    },
    Case {
        name: CaseName(
            "invalid_direct_join_keys_keep_exact_error_index_across_computed_stored_and_both_deliveries",
        ),
        word: Word::Refuses,
        input: || Keying::InvalidLeft,
        expect: Expect::Error(Refused {
            error: OperatingError::Table(RowsError::NoSuchColumn { index: 99 }),
            attempts: REFUSED,
        }),
    },
    Case {
        name: CaseName("an_invalid_right_key_keeps_the_exact_error_index"),
        word: Word::Refuses,
        input: || Keying::InvalidRight,
        expect: Expect::Error(Refused {
            error: OperatingError::Table(RowsError::NoSuchColumn { index: 99 }),
            attempts: REFUSED,
        }),
    },
];

/// Joins the fixture tables on the keying.
fn subject(keying: Input) -> Result<Value, Failure> {
    exercise(keying)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
