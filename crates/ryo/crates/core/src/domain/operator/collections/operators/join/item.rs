//! The `join` operator: combine the rows of two inputs whose keys are equal.

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::{first, second};
use crate::domain::operating::types::{
    Emit, Feed, InputPlace, OperatorContext, Room, Supply, TreeShape,
};
use crate::domain::positions::types::KeyColumns;
use crate::domain::replay::service::derive;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_cells::types::{RowIndex, TreeForm, TreeRows};
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{
    cell, column_in, count_of, index, kinds_of, positions, read_row, reset, row_in, table_in,
};
use crate::domain::table_model::types::{On, RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::tree_join::collections::strategies::hash::item::HashItem;
use crate::domain::tree_join::service::{
    give_back, join_tree, probe, source_column, take_state, tree_finding,
};
use crate::domain::tree_join::types::{EdgeHit, TreeState};
use ryo_language::domain::relations::service::{node_file, node_position};
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn, Side};
use ryo_values::domain::plans::types::Operation;

/// `join`, one item of the `operators` collection.
pub(crate) struct JoinItem;

/// The columns of the answer, as they are in the left rows and in the rows looked up: those of a tree through the view of it.
fn answer_kinds(
    left: &[CellKind],
    right: &[CellKind],
    view: Option<&[ColumnIndex]>,
    output: &[JoinColumn],
    out: &mut Table,
) -> Result<(), OperatingError> {
    reset(out, &[]);
    for column in output {
        let (kinds, place) = match (column.side, view) {
            (Side::Left, _) => (left, column.column),
            (Side::Right, Some(view)) => (right, source_column(view, column.column)),
            (Side::Right, None) => (right, column.column),
        };
        out.kinds.push(
            *column_in(kinds, place).ok_or(TableModelError::NoSuchColumn {
                index: column.column.0,
            })?,
        );
    }
    Ok(())
}

/// Writes into `into` the cells of one answer: each column of `output`, from the left row or the right row.
fn joined_row(
    output: &[JoinColumn],
    left_row: &[Value],
    right_row: &[Value],
    into: &mut Vec<Value>,
) -> Result<(), OperatingError> {
    into.clear();
    for column in output {
        let source = match column.side {
            Side::Left => left_row,
            Side::Right => right_row,
        };
        into.push(cell(source, column.column)?);
    }
    Ok(())
}

/// The rows of `looked_up` whose node in the column `right_key` is an element of the tree of `edges` other than its root, each
/// with the edge that leads to it, in the order of the edges.
///
/// # Errors
///
/// `NoSuchColumn` when the rows lack the column.
fn edge_hits(
    edges: &TreeRows<'_>,
    looked_up: &Table,
    right_key: ColumnIndex,
) -> Result<Vec<EdgeHit>, OperatingError> {
    let mut hits = Vec::new();
    for found in 0..looked_up.count.0 {
        let Some(right_row) = row_in(looked_up, RowIndex(found)) else {
            continue;
        };
        let Value::Node(entity) = cell(right_row, right_key)? else {
            continue;
        };
        let place = usize::try_from(node_position(entity).0).unwrap_or(usize::MAX);
        if node_file(entity) == edges.file && place >= 1 && place < edges.elements.kind.len() {
            hits.push(EdgeHit {
                edge: RowIndex(place - 1),
                found: RowIndex(found),
            });
        }
    }
    hits.sort_unstable();
    Ok(hits)
}

/// Passes on the answer of the join of `left` and `right`, both read in place.
///
/// # Errors
///
/// Whatever the join or the one that takes the rows refuse.
pub(crate) fn join_rows(
    operation: &Operation,
    left: &Rows<'_>,
    right: &Rows<'_>,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Join {
        left_on,
        right_on,
        output,
        filter,
        view,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    if let Rows::Tree(tree) = right
        && let Some(finding) = tree_finding(tree, left_on, right_on, view)
    {
        return join_tree(
            tree,
            left,
            finding,
            TreeShape {
                comparisons: filter,
                view,
                output,
            },
            context,
            emit,
        );
    }
    if !filter.is_empty() || !view.is_empty() {
        return Err(OperatingError::WrongOperation);
    }
    // Nothing on either side joins with nothing: the other side is not even read.
    if count_of(left).0 == 0 || count_of(right).0 == 0 {
        return Ok(());
    }
    let mut row = std::mem::take(&mut context.scratch.row);
    let mut other = std::mem::take(&mut context.scratch.other);
    let mut key = std::mem::take(&mut context.scratch.key);
    let mut order = std::mem::take(&mut context.scratch.order);
    let mut built = std::mem::take(&mut context.scratch.right);
    let outcome = (|| {
        let looked_up = table_in(right, &mut built, &mut row);
        // A few rows on the right find their edges of the tree by the child, and no pass over the edges is made.
        if let (
            Rows::Tree(
                edges @ TreeRows {
                    form: TreeForm::Edges,
                    ..
                },
            ),
            [ColumnIndex(1)],
            [right_key],
        ) = (left, left_on.as_slice(), right_on.as_slice())
            && looked_up.count.0 * 16 < count_of(left).0
        {
            let hits = edge_hits(edges, looked_up, *right_key)?;
            for EdgeHit { edge, found } in hits {
                let (Some(left_row), Some(right_row)) =
                    (read_row(left, edge, &mut row), row_in(looked_up, found))
                else {
                    continue;
                };
                joined_row(output, left_row, right_row, &mut other)?;
                emit.row(context, &other)?;
            }
            return Ok(());
        }
        index(looked_up, On::Columns(right_on), &mut order)?;
        for position in positions(count_of(left)) {
            let Some(left_row) = read_row(left, position, &mut row) else {
                continue;
            };
            for hit in HashItem::find(
                looked_up,
                &order,
                KeyColumns {
                    left: left_on,
                    right: right_on,
                },
                left_row,
                &mut key,
            )? {
                let right_row = row_in(looked_up, *hit).unwrap_or_default();
                joined_row(output, left_row, right_row, &mut other)?;
                emit.row(context, &other)?;
            }
        }
        Ok(())
    })();
    context.scratch.row = row;
    context.scratch.other = other;
    context.scratch.key = key;
    context.scratch.order = order;
    context.scratch.right = built;
    outcome
}

impl OperatorsItem for JoinItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Join { output, view, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let (left, right) = (first(inputs)?, second(inputs)?);
        let tree = matches!(right, Rows::Tree(_));
        answer_kinds(
            kinds_of(left),
            kinds_of(right),
            tree.then_some(view.as_slice()),
            output,
            out,
        )?;
        join_rows(operation, left, right, context, &mut Collect(out))
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        let Operation::Join {
            left_on,
            right_on,
            output,
            filter,
            view,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let Room {
            memo,
            scratch,
            held,
            ..
        } = room;
        let [left_held, right_held, _] = held;
        // A side that has to be made is made only when the other has a row: when nothing is on the left nothing joins. A left
        // that is not made as it is read, or a right that has to be made, is held whole first.
        if ins.streams(InputPlace(0)) == Supply::AtHand
            || ins.streams(InputPlace(1)) == Supply::Streamed
        {
            let left = ins.hold(InputPlace(0), context, left_held)?;
            if count_of(&left).0 == 0 {
                return Ok(());
            }
            let right = ins.hold(InputPlace(1), context, right_held)?;
            let mut own = derive(context, memo, scratch);
            return join_rows(operation, &left, &right, &mut own, emit);
        }
        // Rows as they are made on the left, the right at hand: each finds its rows on the right as it comes.
        let right = ins.hold(InputPlace(1), context, right_held)?;
        if let Rows::Tree(tree) = &right
            && let Some(finding) = tree_finding(tree, left_on, right_on, view)
        {
            let mut state: Option<TreeState> = None;
            let mut found = 0_usize;
            let outcome = ins.stream(
                InputPlace(0),
                context,
                &mut Pipe(|each: &mut OperatorContext<'_>, left_row: &[Value]| {
                    let mut counted = Pipe(|inner: &mut OperatorContext<'_>, row: &[Value]| {
                        found += 1;
                        emit.row(inner, row)
                    });
                    let mut own = derive(each, memo, scratch);
                    if state.is_none() {
                        state = Some(take_state(tree, filter, &mut own));
                    }
                    match state.as_mut() {
                        Some(state) => probe(
                            state,
                            tree,
                            finding,
                            TreeShape {
                                comparisons: filter,
                                view,
                                output,
                            },
                            left_row,
                            &mut own,
                            &mut counted,
                        ),
                        None => Ok(()),
                    }
                }),
            );
            if let Some(state) = state {
                let mut own = derive(context, memo, scratch);
                give_back(state, &mut own);
            }
            ins.found(InputPlace(1), RowCount(found));
            return outcome;
        }
        if !filter.is_empty() || !view.is_empty() {
            return Err(OperatingError::WrongOperation);
        }
        stream_lookup(operation, ins, &right, scratch, context, emit)
    }
}

/// A join whose left rows come as they are made and whose right is a table at hand: each row of the left finds its rows on the
/// right as it comes, the right indexed by its key when the first row comes.
///
/// # Errors
///
/// What indexing the right, the lookup or the one that takes the rows refuse.
fn stream_lookup(
    operation: &Operation,
    ins: &mut dyn Feed<'_>,
    right: &Rows<'_>,
    scratch: &mut OperatorScratch,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let Operation::Join {
        left_on,
        right_on,
        output,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let OperatorScratch {
        row,
        other,
        key,
        order,
        right: built,
        ..
    } = scratch;
    let mut indexed = false;
    ins.stream(
        InputPlace(0),
        context,
        &mut Pipe(|each: &mut OperatorContext<'_>, left_row: &[Value]| {
            let looked_up = match right {
                Rows::Computed(table) => {
                    if !indexed {
                        index(table, On::Columns(right_on), order)?;
                        indexed = true;
                    }
                    table
                }
                Rows::Stored(_) | Rows::Tree(_) => {
                    if !indexed {
                        let table = table_in(right, built, row);
                        index(table, On::Columns(right_on), order)?;
                        indexed = true;
                    }
                    &*built
                }
            };
            for hit in HashItem::find(
                looked_up,
                order,
                KeyColumns {
                    left: left_on,
                    right: right_on,
                },
                left_row,
                key,
            )? {
                let right_row = row_in(looked_up, *hit).unwrap_or_default();
                joined_row(output, left_row, right_row, other)?;
                emit.row(each, other)?;
            }
            Ok(())
        }),
    )
}
