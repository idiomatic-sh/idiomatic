//! The `cycles` operator: the entities that can reach themselves, one row each with the entity in both columns.

use ryo_language::domain::relations::types::Value;

use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::service::first;
use crate::domain::operating::types::{Emit, Feed, OperatorContext, Room};
use crate::domain::replay::service::replay_computed;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{
    cell, check_columns, column_in, count_of, kinds_of, positions, push_row, read_row, reset,
};
use crate::domain::table_model::types::{Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_scratch::types::{CycleRoom, EdgeEnds, EdgePlace, EntityNumber, WalkStep};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::Operation;

/// `cycles`, one item of the `operators` collection.
pub(crate) struct CyclesItem;

/// Marks in `room.found` the nodes on cycles: those in a component of more than one, or with an edge to themselves.
/// The components are found in one pass over the nodes and edges (Tarjan's), in the room the caller keeps.
fn on_cycles(room: &mut CycleRoom) {
    lay_out_edges(room);
    clear_marks(room);
    mark_components(room);
}

/// Lays out the edges leaving each node in one array: where each node's edges start in `room.leaving`, and the node each leads to.
#[expect(
    clippy::indexing_slicing,
    reason = "every index is in bounds by construction: an edge's ends are numbers of `values`, below its length, `starts` and `filled` are sized one past it, and each edge fills one place of `leaving`, sized to the edges"
)]
fn lay_out_edges(room: &mut CycleRoom) {
    let CycleRoom {
        values,
        pairs,
        starts,
        filled,
        leaving,
        ..
    } = room;
    let nodes = values.len();
    starts.0.clear();
    starts.0.resize(nodes + 1, 0);
    for pair in pairs.iter() {
        starts.0[pair.from.0 + 1] += 1;
    }
    for place in 0..nodes {
        starts.0[place + 1] += starts.0[place];
    }
    filled.0.clone_from(&starts.0);
    leaving.0.clear();
    leaving.0.resize(pairs.len(), 0);
    for pair in pairs.iter() {
        leaving.0[filled.0[pair.from.0]] = pair.to.0;
        filled.0[pair.from.0] += 1;
    }
}

/// Clears what a walk marks of each node, sized for the nodes of `room`.
fn clear_marks(room: &mut CycleRoom) {
    let CycleRoom {
        values,
        order,
        low,
        open,
        stack,
        found,
        walk,
        ..
    } = room;
    let nodes = values.len();
    order.0.clear();
    order.0.resize(nodes, usize::MAX);
    low.0.clear();
    low.0.resize(nodes, 0);
    open.0.clear();
    open.0.resize(nodes, false);
    found.0.clear();
    found.0.resize(nodes, false);
    stack.0.clear();
    walk.clear();
}

/// Walks every node once, and marks in `room.found` those of a component of more than one or with an edge to themselves.
#[expect(
    clippy::indexing_slicing,
    reason = "every index is in bounds by construction: a node is a number below the length of `values`, to which `clear_marks` sized `order`, `low`, `open` and `found` and `lay_out_edges` sized `starts` one past, and an edge place is below the next node's start, inside `leaving`"
)]
fn mark_components(room: &mut CycleRoom) {
    let CycleRoom {
        values,
        starts,
        leaving,
        order,
        low,
        open,
        stack,
        found,
        walk,
        component,
        ..
    } = room;
    let nodes = values.len();
    let mut next = 0_usize;
    for root in 0..nodes {
        if order.0[root] != usize::MAX {
            continue;
        }
        walk.push(WalkStep {
            entity: EntityNumber(root),
            edge: EdgePlace(starts.0[root]),
        });
        order.0[root] = next;
        low.0[root] = next;
        next += 1;
        stack.0.push(root);
        open.0[root] = true;
        while let Some(&WalkStep {
            entity: EntityNumber(node),
            edge: EdgePlace(at),
        }) = walk.last()
        {
            if at < starts.0[node + 1] {
                if let Some(top) = walk.last_mut() {
                    top.edge.0 += 1;
                }
                let to = leaving.0[at];
                if to == node {
                    found.0[node] = true;
                }
                // A node not yet walked is not open, so the two cases are apart.
                if open.0[to] {
                    low.0[node] = low.0[node].min(order.0[to]);
                }
                if order.0[to] == usize::MAX {
                    order.0[to] = next;
                    low.0[to] = next;
                    next += 1;
                    stack.0.push(to);
                    open.0[to] = true;
                    walk.push(WalkStep {
                        entity: EntityNumber(to),
                        edge: EdgePlace(starts.0[to]),
                    });
                }
                continue;
            }
            let _: Option<WalkStep> = walk.pop();
            if let Some(&WalkStep {
                entity: EntityNumber(above),
                ..
            }) = walk.last()
            {
                low.0[above] = low.0[above].min(low.0[node]);
            }
            if low.0[node] == order.0[node] {
                component.0.clear();
                while let Some(member) = stack.0.pop() {
                    open.0[member] = false;
                    component.0.push(member);
                    if member == node {
                        break;
                    }
                }
                if component.0.len() > 1 {
                    for member in &component.0 {
                        found.0[*member] = true;
                    }
                }
            }
        }
    }
}

impl OperatorsItem for CyclesItem {
    fn compute(
        &self,
        operation: &Operation,
        inputs: &[Rows<'_>],
        context: &mut OperatorContext<'_>,
        out: &mut Table,
    ) -> Result<(), OperatingError> {
        let Operation::Cycles {
            source,
            destination,
            ..
        } = operation
        else {
            return Err(OperatingError::WrongOperation);
        };
        let edges = first(inputs)?;
        check_columns(kinds_of(edges), &[*source, *destination])?;
        let kinds = kinds_of(edges);
        let kind = |column: ColumnIndex| {
            column_in(kinds, column)
                .copied()
                .ok_or(TableModelError::NoSuchColumn { index: column.0 })
        };
        let wanted = [kind(*source)?, kind(*destination)?];
        let OperatorScratch {
            row: buffer,
            cycles: room,
            ..
        } = &mut *context.scratch;
        room.numbers.clear();
        room.values.clear();
        room.pairs.clear();
        for position in positions(count_of(edges)) {
            let Some(edge) = read_row(edges, position, buffer) else {
                continue;
            };
            let (from, to) = (cell(edge, *source)?, cell(edge, *destination)?);
            let CycleRoom {
                numbers,
                values,
                pairs,
                ..
            } = &mut *room;
            let mut number = |value: Value| {
                *numbers.entry(value).or_insert_with(|| {
                    values.push(value);
                    EntityNumber(values.len() - 1)
                })
            };
            let pair = EdgeEnds {
                from: number(from),
                to: number(to),
            };
            pairs.push(pair);
        }
        on_cycles(room);
        room.ordered.clear();
        let CycleRoom {
            values,
            found,
            ordered,
            ..
        } = &mut *room;
        ordered.extend(
            values
                .iter()
                .zip(found.0.iter())
                .filter(|(_, on)| **on)
                .map(|(value, _)| *value),
        );
        ordered.sort_unstable();
        reset(out, &wanted);
        for value in ordered.iter() {
            push_row(out, &[*value, *value]);
        }
        Ok(())
    }

    fn stream(
        &self,
        operation: &Operation,
        ins: &mut dyn Feed<'_>,
        context: &mut OperatorContext<'_>,
        room: &mut Room,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        replay_computed(
            operation,
            ins,
            context,
            room,
            emit,
            |operation, inputs, context, out| CyclesItem.compute(operation, inputs, context, out),
        )
    }
}
