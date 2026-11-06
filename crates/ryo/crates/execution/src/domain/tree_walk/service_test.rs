//! The `tree_walk` service's cases, on the columns a walk makes.

use ryo_relations::domain::relations::types::{CellKind, Position};
use ryo_values::selects;

use super::service::walk_kinds;
use crate::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

use crate::domain::table_scratch::types::ElementPlaces;
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};

/// What a row of the cases asks: the column the output names, if any.
#[derive(Clone, Copy)]
enum Scenario {
    /// No output is named: the start row with the node found.
    Whole,
    /// The output names the first column of the right side.
    Right,
    /// The output names a column the right side does not have.
    Missing,
    /// Reads the nearest ancestor remembered for an element the remembered answers hold no slot for.
    NearestPastTheTree,
}

/// What a row of the cases asks.
type Input = Scenario;
/// How many columns the walk makes, or for a read of the nearest ancestors, what it crashed with as its length.
type Value = usize;
type Failure = OperatingError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_output_makes_the_start_row_and_the_node_found"),
        word: Word::Succeeds,
        input: || Scenario::Whole,
        expect: Expect::Value(3),
    },
    Case {
        name: CaseName("an_output_column_of_the_right_makes_one_column"),
        word: Word::Succeeds,
        input: || Scenario::Right,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_output_column_the_side_lacks_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Missing,
        expect: Expect::Refusal(|error| selects!(matches!(error, OperatingError::Table(..)))),
    },
    Case {
        name: CaseName("a_nearest_ancestor_with_no_slot_crashes_instead_of_finding_nothing"),
        word: Word::Succeeds,
        input: || Scenario::NearestPastTheTree,
        expect: Expect::Value(1),
    },
];

/// Historical oracle for the retired nearest-cache slot contract.
const SLOT_OF_ELEMENT: &str = "a slot for each element of the tree the walk reads";
#[expect(
    clippy::expect_used,
    reason = "the remembered answers hold one slot per element; a missing slot must crash"
)]
fn nearest_at(nearest: &ElementPlaces, element: Position) -> Position {
    nearest
        .0
        .get(usize::try_from(element.0).unwrap_or(usize::MAX))
        .map(|known| Position(*known))
        .expect(SLOT_OF_ELEMENT)
}

/// Whether `work` crashed with the message a missing slot of the nearest ancestors gives: 1 when it did, 0 otherwise.
fn crashed_on_missing_slot(work: impl FnOnce()) -> usize {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).map_or_else(
        |payload| {
            let message = payload
                .downcast_ref::<&str>()
                .map(|message| (*message).to_owned())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            usize::from(message == SLOT_OF_ELEMENT)
        },
        |()| 0,
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let from = [CellKind::Node, CellKind::Number];
    let reached = [CellKind::Node];
    if matches!(scenario, Scenario::NearestPastTheTree) {
        let nearest = ElementPlaces(vec![0, 0]);
        return Ok(crashed_on_missing_slot(|| {
            let _found = nearest_at(&nearest, Position(5));
        }));
    }
    let output = match scenario {
        Scenario::Whole | Scenario::NearestPastTheTree => Vec::new(),
        Scenario::Right => vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(0),
        }],
        Scenario::Missing => vec![JoinColumn {
            side: Side::Right,
            column: ColumnIndex(4),
        }],
    };
    let mut out = Table::default();
    walk_kinds(&from, &reached, &output, &mut out)?;
    Ok(out.kinds.len())
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
