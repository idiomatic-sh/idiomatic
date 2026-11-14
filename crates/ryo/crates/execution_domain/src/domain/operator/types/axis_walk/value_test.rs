//! Proves the `AxisWalk` value's behaviour: a walk over an empty axis finds nothing, and a walk or an admission
//! with no axis bound keeps the engine's refusal.

use super::super::row_delivery::value::RowDelivery;
use super::value::AxisWalk;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::primitive::types::PreparedInputs;
use crate::domain::operator::contract::types::{AxisWorkspace, PrimitiveWorkspace};
use crate::domain::operator::contract::views::types::PreparedAxis;
use crate::domain::operator::fixtures::{Kept, Resources, extend, with_lease};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::axis::types::AxisCandidates;
use ryo_operators::domain::contract::types::VertexPlace;
use ryo_relations::domain::relations::types::{Count, Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};

/// What a case asks of the walk.
type Ask = for<'f> fn(&mut AxisWalk<'f>, &mut RowDelivery<'_, 'f>) -> Result<AxisCandidates, ()>;
/// The axis the invocation acquired, and what the case asks.
type Input = (Option<PreparedAxis<'static>>, Ask);
/// The candidates the walk found.
type Value = AxisCandidates;
/// The engine error the delivery kept.
type Failure = Option<OperatingError>;

fn subject((axis, case): Input) -> Result<Value, Failure> {
    let resources = Resources { axis };
    let mut workspace = PrimitiveWorkspace {
        axis: Some(AxisWorkspace::default()),
        ..PrimitiveWorkspace::default()
    };
    let operation = extend();
    let mut sink = Kept::default();
    with_lease(&resources, PredicateOutcome::Holds, &mut workspace, |lease| {
        let mut walk = AxisWalk::default();
        let mut rows = RowDelivery::new(lease, &mut sink, &operation, PreparedInputs::Views(&[]));
        case(&mut walk, &mut rows).map_err(|()| rows.take_failure())
    })
}

fn walk_children<'f>(
    walk: &mut AxisWalk<'f>,
    rows: &mut RowDelivery<'_, 'f>,
) -> Result<AxisCandidates, ()> {
    walk.walk(
        rows,
        Direction::Child,
        &[Cell::Number(Integer(1))],
        ColumnIndex(0),
    )
    .map_err(|_| ())
}

/// The rows of this value's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_axis_has_no_candidate"),
        word: Word::Succeeds,
        input: || (Some(PreparedAxis::Empty), walk_children),
        expect: Expect::Value(AxisCandidates::Entities(Count(0))),
    },
    Case {
        name: CaseName("a_walk_needs_an_axis"),
        word: Word::Refuses,
        input: || (None, walk_children),
        expect: Expect::Error(Some(OperatingError::WrongBinding)),
    },
    Case {
        name: CaseName("an_admission_needs_a_walk"),
        word: Word::Refuses,
        input: || {
            (Some(PreparedAxis::Empty), |walk, rows| {
                let _admission = walk.admits(rows, VertexPlace(0)).map_err(|_| ())?;
                Ok(AxisCandidates::Entities(Count(0)))
            })
        },
        expect: Expect::Error(Some(OperatingError::WrongBinding)),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
