//! Reentrant delivery and retry use one genuine prepared program and file owner.
use super::fixtures::exercise_nested;
use super::fixtures::{Attempt, Observed};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use std::borrow::Cow;

const fn n(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

/// The refused first attempt and the replay, delivering the outer rows and publishing
/// `refused_nested` then `replayed_nested` from the nested join.
const fn nested(
    refused_delivered: &'static [Cell],
    replayed_delivered: &'static [Cell],
    refused_nested: &'static [Cell],
    replayed_nested: &'static [Cell],
) -> Observed {
    Observed {
        attempts: [
            Attempt {
                refused: Some(OperatingError::NotRead),
                delivered: Cow::Borrowed(refused_delivered),
                frames: 2,
                nested: Cow::Borrowed(refused_nested),
                frames_on_the_lookup: 2,
                frames_holding_their_memo: 2,
                memo_restored: true,
                providers_read: true,
                reread: false,
            },
            Attempt {
                refused: None,
                delivered: Cow::Borrowed(replayed_delivered),
                frames: 3,
                nested: Cow::Borrowed(replayed_nested),
                frames_on_the_lookup: 3,
                frames_holding_their_memo: 3,
                memo_restored: true,
                providers_read: true,
                reread: false,
            },
        ],
        lookup_calls: 5,
        reads_unchanged_after_release: true,
        builds: 1,
        index_shared: true,
        working_bytes: 0,
    }
}

/// Whether the nested join ends in a select, extend and project to one result.
type Input = bool;
type Value = Observed;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "same_owner_nested_join_preserves_shared_index_bag_order_and_delivery_retry",
        ),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(nested(
            &[n(2)],
            &[n(2), n(2), n(2)],
            &[n(20), n(21), n(22), n(20), n(21), n(22)],
            &[
                n(20),
                n(21),
                n(22),
                n(20),
                n(21),
                n(22),
                n(20),
                n(21),
                n(22),
            ],
        )),
    },
    Case {
        name: CaseName("nested_output_keeps_consumer_memo_scratch_prefix_and_resume"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(nested(
            &[n(7)],
            &[n(7), n(7), n(7)],
            &[n(42), n(42)],
            &[n(42), n(42), n(42)],
        )),
    },
];

/// Runs the nested join through one owner, refused once and replayed.
fn subject(single_result: Input) -> Result<Value, Failure> {
    exercise_nested(single_result)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
