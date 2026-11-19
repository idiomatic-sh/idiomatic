//! The `ordered_walk` item of the `kernels` collection: from each distinct partition and root, in sorted order, an ordered
//! depth-first visit of the edges, a source's edges in ascending order of their order column, each edge reached once with
//! its ordinal among the root's visits. The edges are sorted by partition, source and order, so a source's edges are one
//! run, found by binary search.

use super::super::hash_index::item::indexed;
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{canonical_cells, cell_of, count_of, kinds_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::ordered_dfs::types::DfsParts;
use crate::domain::physical_plan::types::InputRows;
use crate::domain::physical_plan::types::Resources;
use crate::domain::physical_plan::types::keyed_rows::value::{IndexedRow, hash_of_cells};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::visited_set::value::VisitedSet;
use crate::domain::physical_plan::types::walk_scratch::value::WalkScratch;
use crate::domain::table_scratch::types::SetMembership;
use crate::domain::table_texts::service::text_of_cell;
use crate::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::row_texts::types::CellText;
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
            | Lowering::TextMap { .. }
            | Lowering::PointerChase { .. }
            | Lowering::OrderedMatch { .. }
            | Lowering::Render { .. }
            | Lowering::ScanBound { .. }
            | Lowering::Worklist { .. }
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
    texts: &Texts<'_>,
    walk: &mut WalkScratch,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    walk.reached.clear();
    shaped(dfs, roots, edges, output)?;
    // Main compares a text by its canonical symbol: the root columns first, then the edge columns, rows last first.
    canonical_cells(
        roots,
        dfs.root_keys.iter().chain([&dfs.root]),
        texts,
        &mut walk.reached,
        &mut walk.entities,
    )?;
    canonical_cells(
        edges,
        dfs.edge_keys.iter().chain([&dfs.source, &dfs.destination]),
        texts,
        &mut walk.reached,
        &mut walk.next,
    )?;
    let canon = Canon {
        roots: &walk.entities,
        edges: &walk.next,
        keys: KeyPlace(dfs.root_keys.len()),
    };
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
        canon
            .edge_order(*one, *other)
            .then_with(|| cell(edges, *one, dfs.order).cmp(&cell(edges, *other, dfs.order)))
    });
    if sorted.windows(2).any(|pair| {
        canon.edge_order(pair[0], pair[1]).is_eq()
            && cell(edges, pair[0], dfs.order) == cell(edges, pair[1], dfs.order)
    }) {
        return Err(PortableOperatorFault::DuplicateOrder);
    }
    // The roots after the edges, sorted by partition and root.
    for at in 0..count_of(roots).0 {
        sorted.push(IndexedRow(u32::try_from(at).unwrap_or(u32::MAX)));
    }
    let (by_edge, by_root) = sorted.split_at_mut(edge_count);
    by_root.sort_unstable_by(|one, other| canon.root_order(*one, *other));
    // Spike (ryo-fmx3): each partition and source's run of `by_edge`, found once instead of by two searches per expansion.
    let width = canon.keys.0 + 2;
    let runs = &mut walk.edge_runs;
    runs.clear();
    let mut first = 0;
    while first < by_edge.len() {
        let mut end = first + 1;
        while end < by_edge.len() && canon.edge_order(by_edge[first], by_edge[end]).is_eq() {
            end += 1;
        }
        let at = indexed(by_edge[first]).0 * width;
        let hash = hash_of_cells(canon.edges[at..at + width - 1].iter().copied()).0;
        let _ = runs.insert_unique(hash, (first as u32, end as u32), |held| {
            let at = indexed(by_edge[held.0 as usize]).0 * width;
            hash_of_cells(canon.edges[at..at + width - 1].iter().copied()).0
        });
        first = end;
    }
    let key = &mut walk.run_key;
    let mut previous: Option<IndexedRow> = None;
    for root in by_root.iter().copied() {
        if previous.is_some_and(|before| canon.root_order(before, root).is_eq()) {
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
                canon,
                runs,
            },
            key,
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
    /// The canonical key cells of both.
    canon: Canon<'a>,
    /// Spike (ryo-fmx3): each partition and source's run of `by_edge`.
    runs: &'a hashbrown::HashTable<(u32, u32)>,
}

/// The visit from one root: each edge leaving an expanded entity in order, its destination expanded after it is yielded.
/// `frames` holds each expanded entity's next and end place in `by_edge`, two entries per entity.
fn visit(
    dfs: &DfsParts,
    sides: Sides<'_>,
    key: &mut Vec<Value>,
    met: &mut VisitedSet,
    frames: &mut Vec<RowIndex>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    met.reset();
    frames.clear();
    key.clear();
    key.extend((0..sides.canon.keys.0).map(|place| sides.canon.root(sides.root, KeyPlace(place))));
    key.push(Value::Number(Integer(0)));
    let start = sides.canon.root(sides.root, sides.canon.keys);
    let _root = met.meet(start);
    framed(sides, key, start, frames);
    let mut ordinal = Integer(0);
    let mut pending: Option<Value> = None;
    loop {
        if let Some(destination) = pending.take() {
            if met.meet(destination) == SetMembership::Out {
                framed(sides, key, destination, frames);
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
        pending = Some(sides.canon.edge(edge, KeyPlace(sides.canon.keys.0 + 1)));
    }
}

/// Pushes onto `frames` the run of `by_edge` of the edges leaving `entity` in the root's partition: its first and end place.
fn framed(sides: Sides<'_>, key: &mut [Value], entity: Value, frames: &mut Vec<RowIndex>) {
    if let Some(last) = key.last_mut() {
        *last = entity;
    }
    let canon = sides.canon;
    let width = canon.keys.0 + 2;
    let (first, end) = sides
        .runs
        .find(hash_of_cells(key.iter().copied()).0, |held| {
            let at = indexed(sides.by_edge[held.0 as usize]).0 * width;
            canon.edges[at..at + width - 1] == *key
        })
        .map_or((0, 0), |held| (held.0 as usize, held.1 as usize));
    frames.extend([RowIndex(first), RowIndex(end)]);
}

/// The canonical key cells of an ordered visit, row by row: each root's partition and root, each edge's partition, source
/// and destination, a text as the symbol main's engine interned it to.
#[derive(Clone, Copy)]
struct Canon<'a> {
    /// The roots' key cells.
    roots: &'a [Value],
    /// The edges' key cells.
    edges: &'a [Value],
    /// How many partition keys each side has: the place of a root's root, and of an edge's source.
    keys: KeyPlace,
}

/// A place among a row's key cells: its partition keys, then its root, or its source and destination.
#[derive(Clone, Copy)]
struct KeyPlace(usize);

#[expect(
    clippy::indexing_slicing,
    reason = "the key cells are made together with the rows they key, row by row: a place past them is a bug, which panics"
)]
impl Canon<'_> {
    /// A root's key cell at `place`.
    fn root(&self, row: IndexedRow, place: KeyPlace) -> Value {
        self.roots[indexed(row).0 * (self.keys.0 + 1) + place.0]
    }

    /// An edge's key cell at `place`.
    fn edge(&self, row: IndexedRow, place: KeyPlace) -> Value {
        self.edges[indexed(row).0 * (self.keys.0 + 2) + place.0]
    }

    /// The order of two edge rows by partition, then source.
    fn edge_order(&self, one: IndexedRow, other: IndexedRow) -> Ordering {
        (0..=self.keys.0)
            .map(KeyPlace)
            .map(|place| self.edge(one, place).cmp(&self.edge(other, place)))
            .find(|order| order.is_ne())
            .unwrap_or(Ordering::Equal)
    }

    /// The order of two root rows by partition, then root.
    fn root_order(&self, one: IndexedRow, other: IndexedRow) -> Ordering {
        (0..=self.keys.0)
            .map(KeyPlace)
            .map(|place| self.root(one, place).cmp(&self.root(other, place)))
            .find(|order| order.is_ne())
            .unwrap_or(Ordering::Equal)
    }
}

/// Interns, in order, each text a walk numbered after the file's symbols, as main's engine interned it while it prepared the
/// walk, so the next walk of the file finds it among them.
pub(crate) fn interned(fresh: &[Value], resources: &mut dyn Resources) {
    for value in fresh {
        let Some(text) = text_of_cell(*value, &resources.texts()).map(|text| text.0.to_owned())
        else {
            continue;
        };
        let _symbol = resources.intern(CellText(&text));
    }
}

/// The cell of an indexed row.
fn cell(rows: InputRows<'_>, row: IndexedRow, column: ColumnIndex) -> Option<Value> {
    cell_of(rows, indexed(row), column)
}
