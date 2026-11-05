//! Tarjan cycle semantics over immutable engine-prepared adjacency.
use super::super::interface::OperatorsItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::graph::CycleMarks;
use crate::domain::operator::contract::types::{
    OperatorContract, PreparedInputs, PrimitiveLease, RowSink,
};
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_scratch::types::{EdgePlace, EntityNumber, WalkStep};
use ryo_derived::domain::contract::types::CycleGraph;
use ryo_values::domain::plans::types::{Operation, OperationKind};
/// Walks every node once, and marks in `room.found` those of a component of more than one or with an edge to themselves.
#[expect(
    clippy::indexing_slicing,
    reason = "every index is in bounds by construction: a node is a number below the length of `values`, to which `clear_marks` sized `order`, `low`, `open` and `found` and `lay_out_edges` sized `starts` one past, and an edge place is below the next node's start, inside `leaving`"
)]
fn mark_components(graph: &CycleGraph, room: &mut CycleMarks) {
    let values = graph.values();
    let starts = graph.starts();
    let leaving = graph.leaving();
    let CycleMarks {
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
        if order[root] != usize::MAX {
            continue;
        }
        walk.push(WalkStep {
            entity: EntityNumber(root),
            edge: EdgePlace(starts[root]),
        });
        order[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        open[root] = true;
        while let Some(&WalkStep {
            entity: EntityNumber(node),
            edge: EdgePlace(at),
        }) = walk.last()
        {
            if at < starts[node + 1] {
                if let Some(top) = walk.last_mut() {
                    top.edge.0 += 1;
                }
                let to = leaving[at];
                if to == node {
                    found[node] = true;
                }
                // A node not yet walked is not open, so the two cases are apart.
                if open[to] {
                    low[node] = low[node].min(order[to]);
                }
                if order[to] == usize::MAX {
                    order[to] = next;
                    low[to] = next;
                    next += 1;
                    stack.push(to);
                    open[to] = true;
                    walk.push(WalkStep {
                        entity: EntityNumber(to),
                        edge: EdgePlace(starts[to]),
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
                low[above] = low[above].min(low[node]);
            }
            if low[node] == order[node] {
                component.clear();
                while let Some(member) = stack.pop() {
                    open[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                if component.len() > 1 {
                    for member in component.iter() {
                        found[*member] = true;
                    }
                }
            }
        }
    }
}

pub(crate) struct CyclesItem;
impl OperatorsItem for CyclesItem {
    fn prepare(
        &self,
        operation: &Operation,
        inputs: &[crate::domain::operation_schema::types::InputShape<'_>],
        bindings: &[ParameterBinding],
    ) -> Result<OperatorContract, OperatingError> {
        crate::domain::operator::contract::service::prepare_for(
            OperationKind::Cycles,
            operation,
            inputs,
            bindings,
        )
    }
    fn evaluate(
        &self,
        operation: &Operation,
        inputs: PreparedInputs<'_>,
        lease: &mut PrimitiveLease<'_>,
        emit: &mut dyn RowSink,
    ) -> Result<(), OperatingError> {
        if !matches!(operation, Operation::Cycles { .. })
            || !matches!(inputs, PreparedInputs::Views(_))
        {
            return Err(OperatingError::WrongOperation);
        }
        let graph = lease
            .views
            .resources
            .cycles()?
            .ok_or(OperatingError::WrongBinding)?;
        let marks = lease
            .workspace
            .cycles
            .as_mut()
            .ok_or(OperatingError::WrongBinding)?;
        let nodes = graph.values().len();
        marks.order.clear();
        marks.order.resize(nodes, usize::MAX);
        marks.low.clear();
        marks.low.resize(nodes, 0);
        marks.open.clear();
        marks.open.resize(nodes, false);
        marks.found.clear();
        marks.found.resize(nodes, false);
        marks.stack.clear();
        marks.walk.clear();
        mark_components(graph, marks);
        marks.ordered.clear();
        marks.ordered.extend(
            graph
                .values()
                .iter()
                .zip(&marks.found)
                .filter(|(_, on)| **on)
                .map(|(value, _)| *value),
        );
        marks.ordered.sort_unstable();
        // The marks remain engine-owned; copy one scalar before the downstream reborrow.
        let count = marks.ordered.len();
        for index in 0..count {
            let value = lease
                .workspace
                .cycles
                .as_ref()
                .and_then(|work| work.ordered.get(index))
                .copied()
                .ok_or(OperatingError::ShapeMismatch)?;
            emit.row(lease, &[value, value])?;
        }
        Ok(())
    }
}
