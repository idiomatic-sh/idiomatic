//! The `axis` operator: walk a parent and child hierarchy from each row's entity and append the related entity.

use ryo_language::domain::relations::types::{CellKind, Value};

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::{first, second, third};
use crate::domain::operating::types::{
    Emit, Feed, HeldRoom, InputPlace, OperatorContext, Room, StartCount, StreamRoom, Supply,
};
use crate::domain::replay::service::derive;
use crate::domain::replay::types::{Collect, Pipe};
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::service::{
    cell, check_columns, count_of, index, kinds_of, matches_empty, matching, positions, read_row,
    reset, row_in, table_in,
};
use crate::domain::table_model::types::{Emptiness, On, RowCount, Rows, Table};
use crate::domain::table_reach::service::{follow, reach_breadth};
use crate::domain::table_reach::types::{Edges, Walk};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::tree_walk::service::{entity_of, give_walk, start_walk, walk_kinds, walk_row};
use crate::domain::tree_walk::types::TreeWalk;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};
use ryo_values::domain::plans::types::Operation;

/// `axis`, one item of the `operators` collection.
pub(crate) struct AxisItem;

impl OperatorsItem for AxisItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Axis { id, output, .. } = operation else {
            return Err(OperatingError::WrongOperation);
        };
        let (from, edges, domain) = (first(inputs)?, second(inputs)?, third(inputs)?);
        check_columns(kinds_of(from), &[*id])?;
        // No row to start from reaches nothing: the edges and the domain are not read.
        if count_of(from).0 == 0 {
            walk_kinds(kinds_of(from), kinds_of(domain), output, out)?;
            return Ok(());
        }
        // Nothing is an answer when the domain lists nothing: the walk is not made.
        if matches!(domain, Rows::Computed(_)) && count_of(domain).0 == 0 && output.is_empty() {
            reset(out, kinds_of(from));
            out.kinds.push(CellKind::Node);
            return Ok(());
        }
        if let Some(tree) = tree_to_walk(operation, edges, domain) {
            let reached = match domain {
                Rows::Tree(each) if each.form == TreeForm::Elements => each,
                Rows::Stored(_) | Rows::Selected(_) | Rows::Tree(_) | Rows::Computed(_) => tree,
            };
            walk_kinds(kinds_of(from), kinds_of(&Rows::Tree(*reached)), output, out)?;
            let mut walk = start_walk(
                operation,
                tree,
                domain,
                Some(StartCount(count_of(from).0)),
                context,
            )?;
            let mut row = std::mem::take(&mut context.scratch.row);
            let mut answer = std::mem::take(&mut context.scratch.other);
            let outcome = (|| {
                let mut emit = Collect(out);
                for position in positions(count_of(from)) {
                    let Some(from_row) = read_row(from, position, &mut row) else {
                        continue;
                    };
                    walk_row(&mut walk, from_row, context, &mut emit, &mut answer)?;
                }
                Ok(())
            })();
            context.scratch.row = row;
            context.scratch.other = answer;
            give_walk(walk, context);
            return outcome;
        }
        walk_table(operation, [from, edges, domain], context, out)
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        if !matches!(operation, Operation::Axis { .. }) {
            return Err(OperatingError::WrongOperation);
        }
        let Room {
            memo,
            scratch,
            held,
            out,
            ..
        } = room;
        let [from_held, edges_held, domain_held] = held;
        // Edges and domain that are made are made once there is a start; those at hand are read in place.
        let streaming = ins.streams(InputPlace(0)) == Supply::Streamed
            && ins.streams(InputPlace(1)) == Supply::AtHand
            && ins.streams(InputPlace(2)) == Supply::AtHand;
        if !streaming {
            let from = ins.hold(InputPlace(0), context, from_held)?;
            if count_of(&from).0 == 0 {
                return Ok(());
            }
            let edges = ins.hold(InputPlace(1), context, edges_held)?;
            let domain = ins.hold(InputPlace(2), context, domain_held)?;
            return compute_held(
                operation,
                [&from, &edges, &domain],
                context,
                HeldRoom { memo, scratch, out },
                emit,
            );
        }
        let edges = ins.hold(InputPlace(1), context, edges_held)?;
        let domain = ins.hold(InputPlace(2), context, domain_held)?;
        let Some(tree) = tree_to_walk(operation, &edges, &domain) else {
            return stream_held(
                operation,
                ins,
                [&edges, &domain],
                context,
                StreamRoom {
                    held: HeldRoom { memo, scratch, out },
                    from: from_held,
                },
                emit,
            );
        };
        let mut walk: Option<TreeWalk<'_, '_>> = None;
        let mut reached = 0_usize;
        let mut answer = std::mem::take(&mut scratch.other);
        let outcome = ins.stream(
            InputPlace(0),
            context,
            &mut Pipe(|each: &mut OperatorContext<'_>, from_row: &[Value]| {
                let mut own = derive(each, memo, scratch);
                if walk.is_none() {
                    walk = Some(start_walk(operation, tree, &domain, None, &mut own)?);
                }
                match walk.as_mut() {
                    Some(walk) => walk_row(
                        walk,
                        from_row,
                        &mut own,
                        &mut Pipe(|inner: &mut OperatorContext<'_>, row: &[Value]| {
                            reached += 1;
                            emit.row(inner, row)
                        }),
                        &mut answer,
                    ),
                    None => Ok(()),
                }
            }),
        );
        scratch.other = answer;
        if let Some(walk) = walk {
            let mut own = derive(context, memo, scratch);
            give_walk(walk, &mut own);
        }
        ins.found(InputPlace(1), RowCount(reached));
        outcome
    }
}

/// The tree whose edges the axis walks in place, when it can: its edges are the tree's, parent then child, and its domain is
/// every element or the elements a filter of the tree admits.
fn tree_to_walk<'r, 't>(
    operation: &Operation,
    edges: &'r Rows<'t>,
    domain: &Rows<'_>,
) -> Option<&'r TreeRows<'t>> {
    let Operation::Axis {
        parent,
        child,
        domain_id,
        domain_filter,
        domain_view,
        ..
    } = operation
    else {
        return None;
    };
    let Rows::Tree(
        tree @ TreeRows {
            form: TreeForm::Edges,
            ..
        },
    ) = edges
    else {
        return None;
    };
    let domain_of_tree = matches!(
        domain,
        Rows::Tree(TreeRows {
            form: TreeForm::Elements,
            ..
        })
    ) && entity_of(domain_view, *domain_id) == ColumnIndex(0);
    ((*parent, *child) == (ColumnIndex(0), ColumnIndex(1))
        && (domain_filter.is_empty() && domain_view.is_empty() || domain_of_tree))
        .then_some(tree)
}

/// The rows of an axis whose edges are a table, walked by looking up each step: the walks that are not over the tree.
///
/// # Errors
///
/// `WrongOperation` for a walk only the tree can run, or a column the rows lack.
fn walk_table(
    operation: &Operation,
    [from, edges, domain]: [&Rows<'_>; 3],
    context: &mut OperatorContext<'_>,
    out: &mut Table,
) -> Result<(), OperatingError> {
    let Operation::Axis {
        direction,
        id,
        parent,
        child,
        domain_id,
        domain_filter,
        domain_view,
        output,
        ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    if !domain_filter.is_empty()
        || !domain_view.is_empty()
        || !output.is_empty()
        || matches!(
            direction,
            Direction::Deep | Direction::PreviousSibling | Direction::NextSibling
        )
    {
        // Below the children and the siblings are walks of the tree, whose order no table of edges keeps.
        return Err(OperatingError::WrongOperation);
    }
    let OperatorScratch {
        row,
        order,
        right,
        domain: domain_table,
        domain_order,
        walk,
        ..
    } = &mut *context.scratch;
    let right = table_in(edges, right, row);
    let domain_table = table_in(domain, domain_table, row);
    // Walking up follows an edge from its child to its parent; walking down, the other way.
    let (starts, ends) = match direction {
        Direction::Parent | Direction::Ancestor | Direction::NearestAncestor => ([*child], *parent),
        Direction::Child
        | Direction::Descendant
        | Direction::Deep
        | Direction::PreviousSibling
        | Direction::NextSibling => ([*parent], *child),
    };
    check_columns(&right.kinds, &[*parent, *child])?;
    index(right, On::Columns(&starts), order)?;
    let domain_on = [*domain_id];
    index(domain_table, On::Columns(&domain_on), domain_order)?;
    let walked = Edges {
        table: right,
        order,
        from: &starts,
        to: ends,
    };
    reset(out, kinds_of(from));
    out.kinds.push(CellKind::Node);
    for position in positions(count_of(from)) {
        let Some(from_row) = read_row(from, position, row) else {
            continue;
        };
        let start = cell(from_row, *id)?;
        walk.reached.clear();
        match direction {
            Direction::Parent | Direction::Child => {
                let Walk { key, reached, .. } = &mut *walk;
                follow(&walked, start, key, reached);
            }
            Direction::Ancestor
            | Direction::Descendant
            | Direction::Deep
            | Direction::NearestAncestor
            | Direction::PreviousSibling
            | Direction::NextSibling => reach_breadth(&walked, start, walk),
        }
        let valid = |entity: &Value| {
            matches_empty(&matching(
                domain_table,
                On::Columns(&domain_on),
                domain_order,
                &[*entity],
            )) == Emptiness::Filled
        };
        let nearest = matches!(direction, Direction::NearestAncestor);
        for entity in walk
            .reached
            .iter()
            .filter(|entity| valid(entity))
            .take(if nearest { 1 } else { usize::MAX })
        {
            out.cells.extend_from_slice(from_row);
            out.cells.push(*entity);
            out.count = RowCount(out.count.0 + 1);
        }
    }
    Ok(())
}

/// An axis whose starts come as they are made and whose edges are not a tree to walk: the starts are held whole, then computed.
fn stream_held(
    operation: &Operation,
    ins: &mut dyn Feed<'_>,
    held: [&Rows<'_>; 2],
    context: &mut OperatorContext<'_>,
    room: StreamRoom<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let StreamRoom {
        held: HeldRoom { memo, scratch, out },
        from: from_held,
    } = room;
    let from = ins.hold(InputPlace(0), context, from_held)?;
    if count_of(&from).0 == 0 {
        return Ok(());
    }
    compute_held(
        operation,
        [&from, held[0], held[1]],
        context,
        HeldRoom { memo, scratch, out },
        emit,
    )
}

/// The rows an axis makes of rows it holds whole, computed and passed on one by one: the walks that are not over the tree.
fn compute_held(
    operation: &Operation,
    held: [&Rows<'_>; 3],
    context: &mut OperatorContext<'_>,
    room: HeldRoom<'_>,
    emit: &mut dyn Emit,
) -> Result<(), OperatingError> {
    let HeldRoom { memo, scratch, out } = room;
    {
        let mut own = derive(context, memo, scratch);
        AxisItem.compute(
            operation,
            &[held[0].clone(), held[1].clone(), held[2].clone()],
            &mut own,
            out,
        )?;
    }
    let mut own = derive(context, memo, scratch);
    for index in positions(out.count) {
        if let Some(row) = row_in(out, index) {
            emit.row(&mut own, row)?;
        }
    }
    Ok(())
}
