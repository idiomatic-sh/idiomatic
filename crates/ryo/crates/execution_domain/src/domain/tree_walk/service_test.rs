//! The `tree_walk` service's cases, on the columns a walk makes.

use std::borrow::Cow;

use ryo_relations::domain::relations::types::{CellKind, Position};
use ryo_values::selects;

use super::service::walk_kinds;
use crate::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

/// What a row asks.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The kinds of the columns a walk from a node and a number to a node makes, answering the right side's columns at these
    /// places (none: the start row with the node found).
    Walk(&'static [u32]),
    /// Whether reading the nearest ancestor remembered for this element, of remembered answers with this many slots, crashes with
    /// the message of a missing slot.
    NearestPastTheTree(u32, usize),
}

/// What a row answers.
#[derive(Debug, PartialEq)]
enum Answer {
    Kinds(Cow<'static, [CellKind]>),
    Crashed(bool),
}

type Input = Ask;
type Value = Answer;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_output_makes_the_start_row_and_the_node_found"),
        word: Word::Succeeds,
        input: || Ask::Walk(&[]),
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(&[
            CellKind::Node,
            CellKind::Number,
            CellKind::Node,
        ]))),
    },
    Case {
        name: CaseName("an_output_column_of_the_right_makes_one_column"),
        word: Word::Succeeds,
        input: || Ask::Walk(&[0]),
        expect: Expect::Value(Answer::Kinds(Cow::Borrowed(&[CellKind::Node]))),
    },
    Case {
        name: CaseName("an_output_column_the_side_lacks_is_refused"),
        word: Word::Refuses,
        input: || Ask::Walk(&[4]),
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::Table(..)))),
    },
    Case {
        name: CaseName("a_nearest_ancestor_with_no_slot_crashes_instead_of_finding_nothing"),
        word: Word::Succeeds,
        input: || Ask::NearestPastTheTree(5, 2),
        expect: Expect::Value(Answer::Crashed(true)),
    },
];

/// Historical oracle for the retired nearest-cache slot contract.
const SLOT_OF_ELEMENT: &str = "a slot for each element of the tree the walk reads";
fn nearest_at(nearest: &[u32], element: Position) -> Position {
    nearest
        .get(usize::try_from(element.0).unwrap_or(usize::MAX))
        .map(|known| Position(*known))
        .expect(SLOT_OF_ELEMENT)
}

/// Whether `work` crashed with the message a missing slot of the nearest ancestors gives.
fn crashed_on_missing_slot(work: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).map_or_else(
        |payload| {
            let message = payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            message == SLOT_OF_ELEMENT
        },
        |()| false,
    )
}

fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::NearestPastTheTree(element, slots) => {
            let nearest = vec![0; slots];
            Ok(Answer::Crashed(crashed_on_missing_slot(|| {
                let _found = nearest_at(&nearest, Position(element));
            })))
        }
        Ask::Walk(columns) => {
            let output: Vec<JoinColumn> = columns
                .iter()
                .map(|column| JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(*column),
                })
                .collect();
            let mut out = Table::default();
            walk_kinds(
                &[CellKind::Node, CellKind::Number],
                &[CellKind::Node],
                &output,
                &mut out,
            )?;
            Ok(Answer::Kinds(Cow::Owned(out.kinds)))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;
