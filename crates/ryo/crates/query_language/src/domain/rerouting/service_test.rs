//! An operation that read one step reads another in its place, each column it names there renamed; a column the renaming has no
//! place for refuses it.

use super::service::reading_through;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as Reads, JoinColumn, KeyFan, Side, StepId,
};
use ryo_values::domain::plans::types::Operation;

/// Which side of a join reads the step that is replaced.
#[derive(Clone, Copy, Debug)]
enum Read {
    /// The left side.
    Left,
    /// The right side.
    Right,
}

/// The renaming a row gives: each column `c` placed at `c + 10`, or only column 0 placed.
#[derive(Clone, Copy, Debug)]
enum Renaming {
    /// Every column has a place.
    Shifted,
    /// Only column 0 has one.
    OnlyTheFirst,
}

/// What a row asks of the service.
#[derive(Clone, Copy, Debug)]
struct Given {
    /// The side that reads step 0.
    read: Read,
    /// How its columns are renamed.
    renaming: Renaming,
}

/// The join after it reads step 9 in place of step 0: its inputs, its keys and its answer's columns.
#[derive(Debug, PartialEq, Eq)]
struct Rerouted {
    /// Its left input.
    left: Reads,
    /// Its right input.
    right: Reads,
    /// Its left keys.
    left_on: std::borrow::Cow<'static, [ColumnIndex]>,
    /// Its right keys.
    right_on: std::borrow::Cow<'static, [ColumnIndex]>,
    /// Its answer's columns.
    output: std::borrow::Cow<'static, [JoinColumn]>,
}
type Value = Rerouted;
/// A column of the replaced step had no place.
#[derive(Debug, PartialEq, Eq)]
struct Unplaced;
type Input = Given;
type Failure = Unplaced;

/// A join of `left` with `right` on the columns `on`, answering the columns `answered`.
fn join(left: u32, right: u32, on: [u32; 2], answered: [u32; 2]) -> Operation {
    Operation::Join {
        left: Reads::Step(StepId(left)),
        right: Reads::Step(StepId(right)),
        left_on: vec![ColumnIndex(on[0])],
        right_on: vec![ColumnIndex(on[1])],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(answered[0]),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(answered[1]),
            },
        ],
        filter: Vec::new(),
        view: Vec::new(),
        fan: KeyFan::Shared,
        reach: None,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_join_reading_the_step_on_its_left_reads_the_other_with_its_left_columns_renamed",
        ),
        word: Word::Succeeds,
        input: || Given {
            read: Read::Left,
            renaming: Renaming::Shifted,
        },
        expect: Expect::Value(Rerouted {
            left: Reads::Step(StepId(9)),
            right: Reads::Step(StepId(1)),
            left_on: std::borrow::Cow::Borrowed(&[ColumnIndex(11)]),
            right_on: std::borrow::Cow::Borrowed(&[ColumnIndex(1)]),
            output: std::borrow::Cow::Borrowed(&[
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(10),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(0),
                },
            ]),
        }),
    },
    Case {
        name: CaseName(
            "a_join_reading_the_step_on_its_right_reads_the_other_with_its_right_columns_renamed",
        ),
        word: Word::Succeeds,
        input: || Given {
            read: Read::Right,
            renaming: Renaming::Shifted,
        },
        expect: Expect::Value(Rerouted {
            left: Reads::Step(StepId(1)),
            right: Reads::Step(StepId(9)),
            left_on: std::borrow::Cow::Borrowed(&[ColumnIndex(1)]),
            right_on: std::borrow::Cow::Borrowed(&[ColumnIndex(11)]),
            output: std::borrow::Cow::Borrowed(&[
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(10),
                },
            ]),
        }),
    },
    Case {
        name: CaseName("a_column_the_renaming_has_no_place_for_refuses_the_rerouting"),
        word: Word::Refuses,
        input: || Given {
            read: Read::Left,
            renaming: Renaming::OnlyTheFirst,
        },
        expect: Expect::Error(Unplaced),
    },
];

fn subject(given: Input) -> Result<Value, Failure> {
    let mut operation = match given.read {
        Read::Left => join(0, 1, [1, 1], [0, 0]),
        Read::Right => join(1, 0, [1, 1], [0, 0]),
    };
    let renaming = given.renaming;
    reading_through(
        &mut operation,
        Reads::Step(StepId(0)),
        Reads::Step(StepId(9)),
        &|column| match renaming {
            Renaming::Shifted => Some(ColumnIndex(column.0 + 10)),
            Renaming::OnlyTheFirst => (column == ColumnIndex(0)).then_some(column),
        },
    )
    .ok_or(Unplaced)?;
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        output,
        ..
    } = operation
    else {
        return Err(Unplaced);
    };
    Ok(Rerouted {
        left,
        right,
        left_on: left_on.into(),
        right_on: right_on.into(),
        output: output.into(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
