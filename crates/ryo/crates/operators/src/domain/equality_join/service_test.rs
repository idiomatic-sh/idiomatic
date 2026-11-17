//! The `equality_join` service's cases, on values alone.

use super::service::{lower, seeded};
use super::types::{EmptyRight, Existence, JoinShape, ProbeKey, ProbeMode};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, RepeatedRows, Side};

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// A join through a view keys and answers each right column where the right rows hold it.
fn places_right_columns_through_the_view() -> bool {
    let output = [
        JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        },
        JoinColumn {
            side: Side::Right,
            column: ColumnIndex(1),
        },
    ];
    let probe = lower(
        JoinShape {
            left_on: &[ColumnIndex(0)],
            right_on: &[ColumnIndex(0)],
            output: &output,
            view: &[ColumnIndex(2), ColumnIndex(5)],
            reach: None,
            fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
        },
        &[],
        &[],
    );
    matches!(probe.mode, ProbeMode::Inner(_))
        && probe.key
            == ProbeKey::Columns {
                left: vec![ColumnIndex(0)],
                right: vec![ColumnIndex(2)],
            }
        && probe.output
            == [
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(5),
                },
            ]
        && probe.existence == Existence::Every
        && probe.repeated == RepeatedRows::Kept
        && probe.empty_right == EmptyRight::KeyRead
}

/// A column past the view is where it is.
fn a_column_past_the_view_stays() -> bool {
    let probe = lower(
        JoinShape {
            left_on: &[ColumnIndex(1)],
            right_on: &[ColumnIndex(3)],
            output: &[],
            view: &[ColumnIndex(2)],
            reach: None,
            fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
        },
        &[],
        &[],
    );
    probe.key
        == ProbeKey::Columns {
            left: vec![ColumnIndex(1)],
            right: vec![ColumnIndex(3)],
        }
}

/// A join over no closure seeds no reached columns, and probes as the plain join does.
fn unseeded_join_probes_as_the_join() -> bool {
    let shape = JoinShape {
        left_on: &[ColumnIndex(0)],
        right_on: &[ColumnIndex(1)],
        output: &[],
        view: &[],
        reach: None,
        fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
    };
    let join = seeded(shape, &[], &[]);
    join.columns.is_empty() && join.probe == lower(shape, &[], &[])
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_join_places_right_columns_through_its_view"),
        word: Word::Succeeds,
        input: || places_right_columns_through_the_view,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_column_past_the_view_is_where_it_is"),
        word: Word::Succeeds,
        input: || a_column_past_the_view_stays,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_join_over_no_closure_probes_as_the_join"),
        word: Word::Succeeds,
        input: || unseeded_join_probes_as_the_join,
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

#[cfg(test)]
mod band_test;
