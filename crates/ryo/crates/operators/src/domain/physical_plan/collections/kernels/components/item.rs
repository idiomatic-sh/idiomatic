//! The `components` item of the `kernels` collection: Tarjan's strongly connected components (the cycles concept's one
//! search) over the edges, numbered in ascending entity order, and each entity of a component of more than one, or with an edge to itself, once, ascending. The
//! edges are keyed by source through `hash_index`'s table.

use super::super::hash_index::item::{cells, indexed, next_of};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::cycles::types::{
    CycleMark, CycleParts, CycleScratch, EdgePlace, EdgeStart, MeetOrder, NodeCount, NodeNumber,
    WalkStep,
};
use crate::domain::physical_plan::types::keyed_rows::value::{KeyedRows, NO_ROW};
use crate::domain::physical_plan::types::{InputRows, KeyedSide};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};



/// Writes into `output` one row per entity on a cycle, ascending, the entity as both its source and its destination, and
/// answers the fault of the first row that raised one.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows.
pub(crate) fn cyclic(
    cycles: &CycleParts,
    edges: InputRows<'_>,
    index: &mut KeyedRows,
    scratch: &mut CycleScratch,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let kinds = kinds_of(edges);
    output.kinds.clear();
    for column in [cycles.source, cycles.destination] {
        output.kinds.push(
            kinds
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn { index: column })?,
        );
    }
    // The entities, numbered by their place in ascending order.
    let nodes = &mut scratch.ordered;
    nodes.clear();
    for at in 0..count_of(edges).0 {
        nodes.push(end_of(edges, RowIndex(at), cycles.source)?);
        nodes.push(end_of(edges, RowIndex(at), cycles.destination)?);
    }
    nodes.sort_unstable();
    nodes.dedup();
    // The adjacency: the edges keyed by source, each source's edges in edge-row order, laid out by node number.
    cells(
        KeyedSide {
            rows: edges,
            on: std::slice::from_ref(&cycles.source),
        },
        index,
    )?;
    let count = nodes.len();
    scratch.starts.clear();
    scratch.starts.resize(count + 1, EdgeStart(0));
    for run in index.every_run() {
        let node = number_of(nodes, end_of(edges, indexed(run.first), cycles.source)?)?;
        if let Some(slot) = scratch.starts.get_mut(node.0 + 1) {
            *slot = EdgeStart(run.rows.0);
        }
    }
    for at in 1..=count {
        let before = scratch.starts.get(at - 1).copied().unwrap_or(EdgeStart(0));
        if let Some(slot) = scratch.starts.get_mut(at) {
            slot.0 += before.0;
        }
    }
    scratch.leaving.clear();
    scratch
        .leaving
        .resize(scratch.starts.last().map_or(0, |end| end.0), NodeNumber(0));
    for run in index.every_run() {
        let node = number_of(nodes, end_of(edges, indexed(run.first), cycles.source)?)?;
        let mut place = scratch.starts.get(node.0).map_or(0, |start| start.0);
        let mut row = run.first;
        while row != NO_ROW {
            let to = number_of(nodes, end_of(edges, indexed(row), cycles.destination)?)?;
            if let Some(slot) = scratch.leaving.get_mut(place) {
                *slot = to;
            }
            place += 1;
            row = next_of(index, row);
        }
    }
    // Lent to the walk and given back, so their capacity stays with the scratch.
    let (starts, leaving) = (
        std::mem::take(&mut scratch.starts),
        std::mem::take(&mut scratch.leaving),
    );
    components(
        NodeCount(count),
        |node, place| edge_at(&starts, &leaving, node, place),
        scratch,
    );
    scratch.starts = starts;
    scratch.leaving = leaving;
    for (value, on) in scratch.ordered.iter().zip(&scratch.found) {
        if *on == CycleMark::Raised {
            output.cells.extend([*value, *value]);
            output.count = RowCount(output.count.0 + 1);
        }
    }
    Ok(None)
}

/// The cell of an edge's end, or the fault of a column past the row.
fn end_of(
    edges: InputRows<'_>,
    row: RowIndex,
    column: ColumnIndex,
) -> Result<Value, PortableOperatorFault> {
    cell_of(edges, row, column).ok_or(PortableOperatorFault::NoSuchColumn { index: column })
}

/// The number of an entity among the ascending entities.
fn number_of(nodes: &[Value], entity: Value) -> Result<NodeNumber, PortableOperatorFault> {
    nodes
        .binary_search(&entity)
        .map(NodeNumber)
        .map_err(|_| PortableOperatorFault::ShapeMismatch)
}

/// The node the edge at `place` leaving `node` leads to, in the adjacency laid out by node number.
fn edge_at(
    starts: &[EdgeStart],
    leaving: &[NodeNumber],
    node: NodeNumber,
    place: EdgePlace,
) -> Option<NodeNumber> {
    let at = starts.get(node.0)?.0.checked_add(place.0)?;
    (at < starts.get(node.0 + 1)?.0)
        .then(|| leaving.get(at).copied())
        .flatten()
}

/// Marks in `scratch.found`, by node number, each node of `nodes` in a component of more than one or with an edge to itself,
/// `edge` answering the node the edge at a place leaving a node leads to. Tarjan's strongly connected components, one pass.
pub fn components(
    nodes: NodeCount,
    edge: impl Fn(NodeNumber, EdgePlace) -> Option<NodeNumber>,
    scratch: &mut CycleScratch,
) {
    let count = nodes.0;
    scratch.order.clear();
    scratch.order.resize(count, UNMET);
    scratch.low.clear();
    scratch.low.resize(count, MeetOrder(0));
    scratch.open.clear();
    scratch.open.resize(count, CycleMark::Clear);
    scratch.found.clear();
    scratch.found.resize(count, CycleMark::Clear);
    scratch.stack.clear();
    scratch.walk.clear();
    mark_components(nodes, edge, scratch);
}

const UNMET: MeetOrder = MeetOrder(usize::MAX);

/// Tarjan's walk: every node once, marking in `found` those, and marks in `found` those of a component of more than one or with an edge to themselves.
#[expect(
    clippy::indexing_slicing,
    reason = "every index is a node number below the node count, to which `order`, `low`, `open` and `found` were sized before the walk"
)]
fn mark_components(
    nodes: NodeCount,
    lead: impl Fn(NodeNumber, EdgePlace) -> Option<NodeNumber>,
    room: &mut CycleScratch,
) {
    let CycleScratch {
        order,
        low,
        open,
        stack,
        found,
        walk,
        component,
        ..
    } = room;
    let nodes = nodes.0;
    let mut next = 0_usize;
    for root in 0..nodes {
        if order[root] != UNMET {
            continue;
        }
        walk.push(WalkStep {
            node: NodeNumber(root),
            edge: EdgePlace(0),
        });
        order[root] = MeetOrder(next);
        low[root] = MeetOrder(next);
        next += 1;
        stack.push(NodeNumber(root));
        open[root] = CycleMark::Raised;
        while let Some(&WalkStep {
            node: NodeNumber(node),
            edge,
        }) = walk.last()
        {
            if let Some(NodeNumber(to)) = lead(NodeNumber(node), edge) {
                if let Some(top) = walk.last_mut() {
                    top.edge.0 += 1;
                }
                if to == node {
                    found[node] = CycleMark::Raised;
                }
                // A node not yet walked is not open, so the two cases are apart.
                if open[to] == CycleMark::Raised {
                    low[node] = low[node].min(order[to]);
                }
                if order[to] == UNMET {
                    order[to] = MeetOrder(next);
                    low[to] = MeetOrder(next);
                    next += 1;
                    stack.push(NodeNumber(to));
                    open[to] = CycleMark::Raised;
                    walk.push(WalkStep {
                        node: NodeNumber(to),
                        edge: EdgePlace(0),
                    });
                }
                continue;
            }
            let _: Option<WalkStep> = walk.pop();
            if let Some(&WalkStep {
                node: NodeNumber(above),
                ..
            }) = walk.last()
            {
                low[above] = low[above].min(low[node]);
            }
            if low[node] == order[node] {
                component.clear();
                while let Some(member) = stack.pop() {
                    open[member.0] = CycleMark::Clear;
                    component.push(member);
                    if member.0 == node {
                        break;
                    }
                }
                if component.len() > 1 {
                    for member in component.iter() {
                        found[member.0] = CycleMark::Raised;
                    }
                }
            }
        }
    }
}
