//! The `components` item of the `kernels` collection: Tarjan's strongly connected components (the cycles concept's one
//! search) over the edges, numbered in ascending entity order, and each entity of a component of more than one, or with an edge to itself, once, ascending. The
//! edges are keyed by source through `hash_index`'s table.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use super::super::hash_index::item::{cells, indexed, next_of};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::cycles::service::components;
use crate::domain::cycles::types::{CycleMark, CycleParts, CycleScratch, EdgePlace, EdgeStart, NodeCount, NodeNumber};
use crate::domain::physical_plan::types::file_buffers::value::{KeyedRows, NO_ROW};
use crate::domain::physical_plan::types::{InputRows, KeyedSide};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `components`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ComponentsItem;

#[cfg(test)]
impl KernelsItem for ComponentsItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = match lowering {
            Lowering::Components { .. } => true,
            Lowering::Filter { .. }
            | Lowering::Lookup { .. }
            | Lowering::Project { .. }
            | Lowering::Extend { .. }
            | Lowering::Probe { .. }
            | Lowering::Distinct { .. }
            | Lowering::Group { .. }
            | Lowering::Reach { .. }
            | Lowering::SeededReach { .. }
            | Lowering::Axis { .. }
            | Lowering::OrderedWalk { .. }
            | Lowering::Unlowered => false,
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

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
    cells(KeyedSide { rows: edges, on: std::slice::from_ref(&cycles.source) }, index)?;
    let count = nodes.len();
    scratch.starts.clear();
    scratch.starts.resize(count + 1, EdgeStart(0));
    for run in &index.keys {
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
    scratch.leaving.resize(scratch.starts.last().map_or(0, |end| end.0), NodeNumber(0));
    for run in &index.keys {
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
    let (starts, leaving) = (std::mem::take(&mut scratch.starts), std::mem::take(&mut scratch.leaving));
    components(NodeCount(count), |node, place| edge_at(&starts, &leaving, node, place), scratch);
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
fn end_of(edges: InputRows<'_>, row: RowIndex, column: ColumnIndex) -> Result<Value, PortableOperatorFault> {
    cell_of(edges, row, column).ok_or(PortableOperatorFault::NoSuchColumn { index: column })
}

/// The number of an entity among the ascending entities.
fn number_of(nodes: &[Value], entity: Value) -> Result<NodeNumber, PortableOperatorFault> {
    nodes.binary_search(&entity).map(NodeNumber).map_err(|_| PortableOperatorFault::ShapeMismatch)
}

/// The node the edge at `place` leaving `node` leads to, in the adjacency laid out by node number.
fn edge_at(starts: &[EdgeStart], leaving: &[NodeNumber], node: NodeNumber, place: EdgePlace) -> Option<NodeNumber> {
    let at = starts.get(node.0)?.0.checked_add(place.0)?;
    (at < starts.get(node.0 + 1)?.0).then(|| leaving.get(at).copied()).flatten()
}
