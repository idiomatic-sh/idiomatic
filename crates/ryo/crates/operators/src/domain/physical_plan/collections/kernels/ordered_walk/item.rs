//! The `ordered_walk` item of the `kernels` collection: from each distinct partition and root, in sorted order, an ordered
//! depth-first visit of the edges, a source's edges in ascending order of their order column, each edge reached once with
//! its ordinal among the root's visits. The edges are sorted by partition, source and order, so a source's edges are one
//! run, found by binary search.

use super::super::hash_index::item::indexed;
use super::super::hash_index::item::value_hash;
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{cell_of, count_of, kinds_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::ordered_dfs::types::DfsParts;
use crate::domain::physical_plan::types::InputRows;
use crate::domain::physical_plan::types::file_buffers::value::{IndexedRow, WalkScratch};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::cmp::Ordering;

/// `ordered_walk`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct OrderedWalkItem;

#[cfg(test)]
impl KernelsItem for OrderedWalkItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = match lowering {
            Lowering::OrderedWalk { .. } => true,
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
            | Lowering::Components { .. }
            | Lowering::Unlowered => false,
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Writes into `output` each visit's row: the root's keys and root, the edge's source, destination and edge, and the ordinal
/// of the visit among its root's, from 0. A root is visited once per distinct partition and root, in their sorted order; a
/// visit yields every edge leaving an entity it expands, then expands the edge's destination if it was not met yet.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows, a root, source and destination of different kinds,
/// partitions of different kinds or widths, or an order cell that is not a number; `DuplicateOrder` for two edges of one
/// partition and source of equal order.
pub(crate) fn visited(
    dfs: &DfsParts,
    roots: InputRows<'_>,
    edges: InputRows<'_>,
    walk: &mut WalkScratch,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    shaped(dfs, roots, edges, output)?;
    let edge_count = count_of(edges).0;
    let sorted = &mut walk.starts;
    sorted.clear();
    for at in 0..edge_count {
        if !matches!(
            cell_of(edges, RowIndex(at), dfs.order),
            Some(Value::Number(_))
        ) {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
        sorted.push(IndexedRow(u32::try_from(at).unwrap_or(u32::MAX)));
    }
    sorted.sort_unstable_by(|one, other| {
        edge_order(edges, dfs, *one, *other)
            .then_with(|| cell(edges, *one, dfs.order).cmp(&cell(edges, *other, dfs.order)))
    });
    if sorted.windows(2).any(|pair| {
        edge_order(edges, dfs, pair[0], pair[1]).is_eq()
            && cell(edges, pair[0], dfs.order) == cell(edges, pair[1], dfs.order)
    }) {
        return Err(PortableOperatorFault::DuplicateOrder);
    }
    // The roots after the edges, sorted by partition and root.
    for at in 0..count_of(roots).0 {
        sorted.push(IndexedRow(u32::try_from(at).unwrap_or(u32::MAX)));
    }
    let (by_edge, by_root) = sorted.split_at_mut(edge_count);
    by_root.sort_unstable_by(|one, other| root_order(roots, dfs, *one, *other));
    let mut previous: Option<IndexedRow> = None;
    for root in by_root.iter().copied() {
        if previous.is_some_and(|before| root_order(roots, dfs, before, root).is_eq()) {
            continue;
        }
        previous = Some(root);
        visit(
            dfs,
            Sides {
                roots,
                edges,
                root,
                by_edge,
            },
            &mut walk.visited,
            &mut walk.frontier,
            output,
        )?;
    }
    Ok(None)
}

/// Checks the columns' kinds, and gives `output` the kinds of its row.
fn shaped(
    dfs: &DfsParts,
    roots: InputRows<'_>,
    edges: InputRows<'_>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let kind = |kinds: &[CellKind], column: ColumnIndex| {
        kinds
            .get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
    };
    let (root_kinds, edge_kinds) = (kinds_of(roots), kinds_of(edges));
    let root = kind(root_kinds, dfs.root)?;
    if kind(edge_kinds, dfs.source)? != root
        || kind(edge_kinds, dfs.destination)? != root
        || kind(edge_kinds, dfs.order)? != CellKind::Number
        || dfs.root_keys.len() != dfs.edge_keys.len()
    {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    output.kinds.clear();
    for (left, right) in dfs.root_keys.iter().zip(&dfs.edge_keys) {
        let left = kind(root_kinds, *left)?;
        if left != kind(edge_kinds, *right)? {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
        output.kinds.push(left);
    }
    output.kinds.push(root);
    for column in [dfs.source, dfs.destination, dfs.edge] {
        output.kinds.push(kind(edge_kinds, column)?);
    }
    output.kinds.push(CellKind::Number);
    Ok(())
}

/// The rows one root's visit reads: the root row, the roots and edges, and the edges sorted by partition, source and order.
#[derive(Clone, Copy)]
struct Sides<'a> {
    /// The root rows.
    roots: InputRows<'a>,
    /// The edge rows.
    edges: InputRows<'a>,
    /// The root row visited from.
    root: IndexedRow,
    /// The edge rows, sorted.
    by_edge: &'a [IndexedRow],
}

/// The visit from one root: each edge leaving an expanded entity in order, its destination expanded after it is yielded.
/// `frames` holds each expanded entity's next and end place in `by_edge`, two entries per entity.
fn visit(
    dfs: &DfsParts,
    sides: Sides<'_>,
    met: &mut hashbrown::HashTable<Value>,
    frames: &mut Vec<RowIndex>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    met.clear();
    frames.clear();
    let start = cell(sides.roots, sides.root, dfs.root)
        .ok_or(PortableOperatorFault::NoSuchColumn { index: dfs.root })?;
    let _root = met.insert_unique(value_hash(start).0, start, |held| value_hash(*held).0);
    framed(dfs, sides, start, frames);
    let mut ordinal = Integer(0);
    let mut pending: Option<Value> = None;
    loop {
        if let Some(destination) = pending.take() {
            let hash = value_hash(destination).0;
            if met.find(hash, |held| *held == destination).is_none() {
                let _met = met.insert_unique(hash, destination, |held| value_hash(*held).0);
                framed(dfs, sides, destination, frames);
            }
        }
        let at = loop {
            let [.., next, end] = frames.as_mut_slice() else {
                return Ok(());
            };
            if next.0 < end.0 {
                let at = next.0;
                next.0 += 1;
                break at;
            }
            frames.truncate(frames.len() - 2);
        };
        let edge = *sides
            .by_edge
            .get(at)
            .ok_or(PortableOperatorFault::ShapeMismatch)?;
        for column in dfs.root_keys.iter().chain([&dfs.root]) {
            output.cells.push(
                cell(sides.roots, sides.root, *column)
                    .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?,
            );
        }
        for column in [dfs.source, dfs.destination, dfs.edge] {
            output.cells.push(
                cell(sides.edges, edge, column)
                    .ok_or(PortableOperatorFault::NoSuchColumn { index: column })?,
            );
        }
        output.cells.push(Value::Number(ordinal));
        output.count = RowCount(output.count.0 + 1);
        ordinal = Integer(
            ordinal
                .0
                .checked_add(1)
                .ok_or(PortableOperatorFault::ShapeMismatch)?,
        );
        pending = cell(sides.edges, edge, dfs.destination);
    }
}

/// Pushes onto `frames` the run of `by_edge` of the edges leaving `entity` in the root's partition: its first and end place.
fn framed(dfs: &DfsParts, sides: Sides<'_>, entity: Value, frames: &mut Vec<RowIndex>) {
    let against = |edge: &IndexedRow| {
        dfs.edge_keys
            .iter()
            .zip(&dfs.root_keys)
            .map(|(edge_key, root_key)| {
                cell(sides.edges, *edge, *edge_key).cmp(&cell(sides.roots, sides.root, *root_key))
            })
            .find(|order| order.is_ne())
            .unwrap_or_else(|| cell(sides.edges, *edge, dfs.source).cmp(&Some(entity)))
    };
    let first = sides
        .by_edge
        .partition_point(|edge| against(edge) == Ordering::Less);
    let end = sides
        .by_edge
        .partition_point(|edge| against(edge) != Ordering::Greater);
    frames.extend([RowIndex(first), RowIndex(end)]);
}

/// The order of two edge rows by partition, then source.
fn edge_order(
    edges: InputRows<'_>,
    dfs: &DfsParts,
    one: IndexedRow,
    other: IndexedRow,
) -> Ordering {
    dfs.edge_keys
        .iter()
        .chain([&dfs.source])
        .map(|column| cell(edges, one, *column).cmp(&cell(edges, other, *column)))
        .find(|order| order.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// The order of two root rows by partition, then root.
fn root_order(
    roots: InputRows<'_>,
    dfs: &DfsParts,
    one: IndexedRow,
    other: IndexedRow,
) -> Ordering {
    dfs.root_keys
        .iter()
        .chain([&dfs.root])
        .map(|column| cell(roots, one, *column).cmp(&cell(roots, other, *column)))
        .find(|order| order.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// The cell of an indexed row.
fn cell(rows: InputRows<'_>, row: IndexedRow, column: ColumnIndex) -> Option<Value> {
    cell_of(rows, indexed(row), column)
}
