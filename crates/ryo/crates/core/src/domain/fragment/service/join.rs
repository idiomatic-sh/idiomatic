//! The `joins` service: the lowered steps that combine the rows of two steps, with no IO and no ports.

use ryo_language::domain::elements::service::children;
use ryo_language::domain::symbols::service::name_count;
use ryo_relations::domain::relations::types::{CellKind, Position, Relation, Value};

use crate::domain::comparing::service::holds_all;
use crate::domain::lowering::constants::EDGE;
use crate::domain::lowering::service::{kind_of, shape_at, step_of, viewed};
use crate::domain::lowering::types::{LoweredColumn, Lowering};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Memo, NameCount, OperatorContext};
use crate::domain::positions::service::{count, followed, in_tree, key_into, made_of, row, value};
use crate::domain::positions::types::{
    KeyColumns, LoweredRoom, Positions, Sides, Slots, Stride, Work,
};
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_model::service::positions;
use crate::domain::table_model::types::Verdict;
use crate::domain::walks::service::take_filter;
use crate::domain::walks::service::{allows, give};
use crate::domain::walks::types::Admit;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Comparison;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::Operation;

use crate::domain::operator::contract::types::{PrimitiveSemantics, join_result};
use ryo_operators::domain::contract::types::{
    JoinInput, JoinInterrupted, OperatorCall, PositionJoinDriver,
};
mod drivers;
/// Whether the edge to the element at `child` is one the comparisons admit.
fn edge_admitted(
    tree: &TreeRows<'_>,
    comparisons: &[Comparison],
    child: Position,
    cells: &mut Vec<Value>,
    context: &mut OperatorContext<'_>,
) -> Result<Verdict, OperatingError> {
    if comparisons.is_empty() {
        return Ok(Verdict::Holds);
    }
    cells.clear();
    for field in EDGE {
        cells.push(value(tree, field, child));
    }
    let kinds: Vec<CellKind> = EDGE.iter().map(|field| kind_of(*field)).collect();
    let names = NameCount(name_count(context.symbols.as_ref()).0);
    holds_all(comparisons, cells, &kinds, names, context)
}

/// The rows of the join of the left step with the elements of the tree: each left row whose key column leads to an element the
/// filter admits, with that element.
///
/// # Errors
///
/// What a comparison of the filter refuses.
fn join_tree(
    semantics: &dyn PrimitiveSemantics,
    tree: &TreeRows<'_>,
    filter: &[Comparison],
    left_rows: &Positions,
    key_column: LoweredColumn,
    context: &mut OperatorContext<'_>,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let mut admit = if filter.is_empty() {
        Admit::All
    } else {
        Admit::Filter(take_filter(tree, filter, context), filter)
    };
    let mut outcome = Ok(());
    for at in positions(count(left_rows)) {
        let each = row(left_rows, at);
        let Some(hit) = followed(tree, key_column, each) else {
            continue;
        };
        let hit = in_tree(tree, hit);
        let mut driver = drivers::Tree {
            tree,
            admit: &mut admit,
            context,
            hit: [hit.0],
            pending: true,
            ready: false,
            error: None,
        };
        let answer = semantics.operators(OperatorCall::EqualityJoin {
            input: JoinInput::Positions {
                left: each.0,
                driver: &mut driver,
                target: &mut out.0,
            },
        });
        if let Err(error) = join_result(answer, driver.error.take()) {
            outcome = Err(error);
            break;
        }
    }
    give(admit, context);
    outcome
}

/// The rows of the join of the left step with the right step: each pair of rows whose keys are equal.
///
/// # Errors
///
/// `NotRun` when a step was not lowered.
fn join_steps(
    semantics: &dyn PrimitiveSemantics,
    derived: &mut dyn crate::domain::operator::contract::derived::ScopedDerived,
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    sides: Sides,
    keys_of: KeyColumns<'_>,
    room: &mut LoweredRoom,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let left_shape = shape_at(lowering, sides.left)?;
    let right_shape = shape_at(lowering, sides.right)?;
    let LoweredRoom {
        made, keys, probe, ..
    } = room;
    let left_rows = made_of(made, sides.left).ok_or(OperatingError::NotRun)?;
    let right_rows = made_of(made, sides.right).ok_or(OperatingError::NotRun)?;
    let stride = Stride(keys_of.right.len());
    keys.0.clear();
    for at in positions(count(right_rows)) {
        key_into(
            tree,
            &right_shape.columns,
            row(right_rows, at),
            keys_of.right,
            keys,
        );
    }
    derived.with_raw_order(&keys.0, stride.0, &mut |order| {
        for at in positions(count(left_rows)) {
            let each = row(left_rows, at);
            probe.0.clear();
            key_into(tree, &left_shape.columns, each, keys_of.left, probe);
            let mut driver = drivers::Steps {
                right: right_rows,
                order,
                places: order.hits(&probe.0),
                current: None,
            };
            join_result(
                semantics.operators(OperatorCall::EqualityJoin {
                    input: JoinInput::Positions {
                        left: each.0,
                        driver: &mut driver,
                        target: &mut out.0,
                    },
                }),
                None,
            )?;
        }
        Ok(())
    })
}

/// The rows of the join of the left step with a side of the tree, or with another step.
///
/// # Errors
///
/// What a comparison refuses, or `NotRun` when the step is not a join the tree can run.
pub(super) fn run(
    semantics: &dyn PrimitiveSemantics,
    tree: &TreeRows<'_>,
    lowering: &Lowering,
    operation: &Operation,
    work: Work<'_, '_>,
    out: &mut Slots,
) -> Result<(), OperatingError> {
    let Work {
        room,
        context,
        derived,
    } = work;
    let Operation::Join {
        left,
        right,
        left_on,
        right_on,
        filter,
        view,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let left_id = step_of(*left)?;
    let left_shape = shape_at(lowering, left_id)?;
    let key_column = left_shape
        .columns
        .get(
            left_on
                .first()
                .map_or(usize::MAX, |on| usize::try_from(on.0).unwrap_or(usize::MAX)),
        )
        .copied()
        .ok_or(OperatingError::NotRun)?;
    let left_place = usize::try_from(left_id.0).unwrap_or(usize::MAX);
    match right {
        Input::Relation(Relation::SyntaxTree) => join_tree(
            semantics,
            tree,
            filter,
            room.made.get(left_place).ok_or(OperatingError::NotRun)?,
            key_column,
            context,
            out,
        ),
        Input::Relation(Relation::SyntaxChild) => {
            let LoweredRoom {
                made, found, cells, ..
            } = room;
            let left_rows = made.get(left_place).ok_or(OperatingError::NotRun)?;
            let by_parent = right_on
                .first()
                .is_some_and(|on| viewed(view, *on) == ColumnIndex(0));
            let names = NameCount(name_count(context.symbols.as_ref()).0);
            context.memo.clear();
            context.memo.resize(filter.len() * names.0, Memo::Unknown);
            for at in positions(count(left_rows)) {
                let each = row(left_rows, at);
                let Some(start) = followed(tree, key_column, each) else {
                    continue;
                };
                found.0.clear();
                let start = in_tree(tree, start);
                // The root, at position 0, is no element's child: by the parent it has its children, by itself none.
                if by_parent {
                    found
                        .0
                        .extend(children(tree.elements, start).map(|child| child.0));
                } else if start.0 >= 1 {
                    found.0.push(start.0);
                } else {
                    // The root by itself: no row.
                }
                let mut driver = drivers::Edges {
                    tree,
                    filter,
                    cells,
                    context,
                    found: &found.0,
                    at: 0,
                    current: None,
                    error: None,
                };
                let answer = semantics.operators(OperatorCall::EqualityJoin {
                    input: JoinInput::Positions {
                        left: each.0,
                        driver: &mut driver,
                        target: &mut out.0,
                    },
                });
                join_result(answer, driver.error.take())?;
            }
            Ok(())
        }
        Input::Step(right_id) => {
            let sides = Sides {
                left: left_id,
                right: *right_id,
            };
            let keys_of = KeyColumns {
                left: left_on,
                right: right_on,
            };
            join_steps(
                semantics, derived, tree, lowering, sides, keys_of, room, out,
            )
        }
        Input::Relation(_) => Err(OperatingError::NotAStep),
    }
}

use ryo_values::domain::rows::types::RowIndex;
