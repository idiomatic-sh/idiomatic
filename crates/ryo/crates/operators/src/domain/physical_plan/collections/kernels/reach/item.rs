//! The `reach` item of the `kernels` collection: the entities a closure's edges lead to from a start, by one or more edges,
//! each once, ascending, the edges keyed by partition and source through `hash_index`'s table. From every source of every
//! partition (`traverse::All`), or from only the partition and source each left row binds (`traverse::Seeds`), each start
//! walked once per file and its run kept for the rows that bind it again.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use super::super::hash_index::item::{cells, find_cells, hash_of, indexed, next_of};
use super::super::probe::item::ProbeScratch;
use super::super::scan_facts::item::{cell_of, count_of, kinds_of};
use crate::domain::closure::types::ReachParts;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::SeededJoin;
use crate::domain::physical_plan::types::file_buffers::value::{
    IndexKeyHash, IndexedRow, KeyedRows, NO_ROW, ReachedCount, ReachedPlace, ReachedRun, WalkScratch,
};
use crate::domain::physical_plan::types::{InputRows, KernelRun, KeyedRowAt, KeyedSide};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// `reach`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct ReachItem;

#[cfg(test)]
impl KernelsItem for ReachItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = match lowering {
            Lowering::Reach { .. } | Lowering::SeededReach { .. } => true,
            Lowering::Filter { .. }
            | Lowering::Lookup { .. }
            | Lowering::Project { .. }
            | Lowering::Extend { .. }
            | Lowering::Probe { .. }
            | Lowering::Distinct { .. }
            | Lowering::Group { .. }
            | Lowering::Axis { .. }
            | Lowering::OrderedWalk { .. }
            | Lowering::Components { .. }
            | Lowering::Unlowered => false,
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// The rows a reach walks from: every source of its edges, or the left rows of the join it was fused into.
#[derive(Clone, Copy, Debug)]
#[expect(dead_code, reason = "red step: F3 commit 3 runs it")]
pub(crate) enum Starts<'a> {
    /// Every source of every partition, in partition order then ascending.
    Every,
    /// The left rows, read in order, each probing the run of the start it binds.
    Seeded(InputRows<'a>, &'a SeededJoin),
}

/// What a reach borrows besides its rows: the run, for a seeded closure's probe, and the worker's two keyed-row tables.
pub(crate) struct ReachScratch<'s> {
    /// The edges, keyed by partition and source.
    pub(crate) index: &'s mut KeyedRows,
    /// The walks' work.
    pub(crate) walk: &'s mut WalkScratch,
    /// The probe's own scratch, over the second keyed-row table, for a seeded closure.
    pub(crate) probe: ProbeScratch<'s>,
}

/// Writes into `output` the rows of the closure of `edges` from `starts`, and answers the fault of the first row that raised
/// one.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows.
pub(crate) fn reached(
    graph: &ReachParts,
    edges: InputRows<'_>,
    starts: Starts<'_>,
    run: &KernelRun<'_>,
    scratch: ReachScratch<'_>,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let ReachScratch { index, walk, probe } = scratch;
    let columns = &mut walk.columns;
    columns.near.clear();
    columns.near.extend(graph.keys.iter().copied().chain([graph.source]));
    columns.far.clear();
    columns.far.extend(graph.keys.iter().copied().chain([graph.destination]));
    cells(KeyedSide { rows: edges, on: &columns.near }, index)?;
    walk.reached.clear();
    walk.runs.clear();
    // Red step: the walks land in F3 commit 3; until then a reach answers no row.
    let _ = (starts, run, probe);
    output.count = RowCount(0);
    Ok(None)
}

#[expect(dead_code, reason = "red step: F3 commit 3 runs it")]
/// The closure from every start, partition by partition and source by source ascending, each start's entities ascending.
fn every(
    graph: &ReachParts,
    edges: InputRows<'_>,
    index: &KeyedRows,
    walk: &mut WalkScratch,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let kinds = kinds_of(edges);
    output.kinds.clear();
    for column in graph.keys.iter().chain([&graph.source, &graph.destination]) {
        output.kinds.push(
            kinds
                .get(usize::try_from(column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?,
        );
    }
    walk.starts.clear();
    walk.starts.extend(index.keys.iter().map(|run| run.first));
    let near = &walk.columns.near;
    walk.starts.sort_unstable_by(|left, right| {
        near.iter()
            .map(|column| cell_of(edges, indexed(*left), *column).cmp(&cell_of(edges, indexed(*right), *column)))
            .find(|order| order.is_ne())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for at in 0..walk.starts.len() {
        let start = walk.starts[at];
        walk.reached.clear();
        let side = KeyedSide { rows: edges, on: &walk.columns.near };
        let first = find_cells(index, side, KeyedRowAt { side, row: indexed(start) })?.map_or(NO_ROW, |run| run.first);
        walked(edges, index, &walk.columns.far, &walk.columns.near, first, WalkState {
            visited: &mut walk.visited,
            frontier: &mut walk.frontier,
            reached: &mut walk.reached,
        }, graph.destination)?;
        walk.reached.sort_unstable();
        for entity in &walk.reached {
            for column in &walk.columns.near {
                output.cells.push(cell_of(edges, indexed(start), *column).ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?);
            }
            output.cells.push(*entity);
            output.count = RowCount(output.count.0 + 1);
        }
    }
    Ok(())
}

#[expect(dead_code, reason = "red step: F3 commit 3 runs it")]
/// The work of one walk, borrowed from the scratch.
struct WalkState<'w> {
    /// The entities met.
    visited: &'w mut hashbrown::HashTable<Value>,
    /// The edge rows whose far ends are left to expand.
    frontier: &'w mut Vec<RowIndex>,
    /// Where the entities met are appended.
    reached: &'w mut Vec<Value>,
}

#[expect(dead_code, reason = "red step: F3 commit 3 runs it")]
/// Appends to `state.reached` each entity the edges lead to from the start whose first edge row is `first`, once.
fn walked(
    edges: InputRows<'_>,
    index: &KeyedRows,
    far: &[ColumnIndex],
    near: &[ColumnIndex],
    first: IndexedRow,
    state: WalkState<'_>,
    destination: ColumnIndex,
) -> Result<(), PortableOperatorFault> {
    state.visited.clear();
    state.frontier.clear();
    let indexed_side = KeyedSide { rows: edges, on: near };
    let mut row = first;
    loop {
        while row != NO_ROW {
            let at = indexed(row);
            let entity = cell_of(edges, at, destination).ok_or(PortableOperatorFault::NoSuchColumn { index: destination })?;
            let hash = value_hash(entity).0;
            if state.visited.find(hash, |held| *held == entity).is_none() {
                let _met = state.visited.insert_unique(hash, entity, |held| value_hash(*held).0);
                state.reached.push(entity);
                state.frontier.push(at);
            }
            row = next_of(index, row);
        }
        let Some(edge) = state.frontier.pop() else {
            return Ok(());
        };
        let probe = KeyedRowAt { side: KeyedSide { rows: edges, on: far }, row: edge };
        row = find_cells(index, indexed_side, probe)?.map_or(NO_ROW, |run| run.first);
    }
}

#[expect(dead_code, reason = "red step: F3 commit 3 runs it")]
/// The rows each start a left row binds reaches, as the join's logical right rows, walking each start once.
fn seeded_rows(
    graph: &ReachParts,
    edges: InputRows<'_>,
    left: InputRows<'_>,
    join: &SeededJoin,
    index: &KeyedRows,
    walk: &mut WalkScratch,
) -> Result<(), PortableOperatorFault> {
    let width = graph.keys.len() + 1;
    let seed = &mut walk.columns.seed;
    seed.clear();
    seed.resize(width, ColumnIndex(u32::MAX));
    let crate::domain::equality_join::types::ProbeKey::Columns { left: left_on, right: right_on } = &join.probe.key else {
        return Err(PortableOperatorFault::WrongOperation);
    };
    for (left_column, right_column) in left_on.iter().zip(right_on) {
        let bound = join
            .columns
            .get(usize::try_from(right_column.0).unwrap_or(usize::MAX))
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: *right_column })?;
        if let Some(slot) = seed.get_mut(usize::try_from(bound.0).unwrap_or(usize::MAX))
            && slot.0 == u32::MAX
        {
            *slot = *left_column;
        }
    }
    let rows = &mut walk.rows;
    rows.cells.clear();
    rows.count = RowCount(0);
    rows.kinds.clear();
    let kinds = kinds_of(edges);
    for column in &join.columns {
        let edge_column = graph
            .keys
            .iter()
            .chain([&graph.source, &graph.destination])
            .nth(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?;
        rows.kinds.push(
            kinds
                .get(usize::try_from(edge_column.0).unwrap_or(usize::MAX))
                .copied()
                .ok_or(PortableOperatorFault::NoSuchColumn { index: edge_column })?,
        );
    }
    let seed_side = KeyedSide { rows: left, on: &walk.columns.seed };
    for at in 0..count_of(left).0 {
        let probe = KeyedRowAt { side: seed_side, row: RowIndex(at) };
        let hash = hash_of(probe)?;
        let known = walk.runs.find(hash.0, |run| {
            run.hash == hash
                && walk.columns.seed.iter().all(|column| {
                    cell_of(left, indexed(run.first), *column) == cell_of(left, RowIndex(at), *column)
                })
        });
        if known.is_some() {
            continue;
        }
        let start = ReachedPlace(u32::try_from(walk.reached.len()).unwrap_or(u32::MAX));
        let indexed_side = KeyedSide { rows: edges, on: &walk.columns.near };
        let first = find_cells(index, indexed_side, probe)?.map_or(NO_ROW, |run| run.first);
        walked(edges, index, &walk.columns.far, &walk.columns.near, first, WalkState {
            visited: &mut walk.visited,
            frontier: &mut walk.frontier,
            reached: &mut walk.reached,
        }, graph.destination)?;
        let from = usize::try_from(start.0).unwrap_or(usize::MAX);
        walk.reached[from..].sort_unstable();
        let count = ReachedCount(u32::try_from(walk.reached.len() - from).unwrap_or(u32::MAX));
        let _run = walk.runs.insert_unique(
            hash.0,
            ReachedRun { hash, first: IndexedRow(u32::try_from(at).unwrap_or(u32::MAX)), start, count },
            |run| run.hash.0,
        );
        for entity in &walk.reached[from..] {
            for column in &join.columns {
                let place = usize::try_from(column.0).unwrap_or(usize::MAX);
                let cell = if place < width {
                    let bound = walk.columns.seed[place];
                    cell_of(left, RowIndex(at), bound).ok_or(PortableOperatorFault::NoSuchColumn { index: bound })?
                } else {
                    *entity
                };
                walk.rows.cells.push(cell);
            }
            walk.rows.count = RowCount(walk.rows.count.0 + 1);
        }
    }
    Ok(())
}

#[expect(dead_code, reason = "red step: F3 commit 3 runs it")]
/// The hash a walk keeps an entity it met under.
fn value_hash(value: Value) -> IndexKeyHash {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = rustc_hash::FxHasher::default();
    value.hash(&mut hasher);
    IndexKeyHash(hasher.finish())
}
