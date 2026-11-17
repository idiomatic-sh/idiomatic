//! The `queued_walk` item of the `kernels` collection: from each distinct partition and root, in sorted order, a
//! first-in-first-out worklist fixpoint over a declared lattice. An entity leaving the queue follows, in ascending order, its
//! edges whose before-state is the state it holds then, each joining its after-state into the destination, which joins the
//! queue again when its state changes; every edge followed is one row, with the destination's state after the join.

use super::super::distinct::item::counted;
use super::super::hash_index::item::value_hash;
#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{canonical_cells, cell_of, count_of, kinds_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{CellStart, InputRows};
use crate::domain::table_texts::types::Texts;
use crate::domain::worklist::service::{joined, output_kinds, place, state_of, tabled};
use crate::domain::worklist::types::{
    HeldState, LatticeSide, StatePlace, WorklistParts, WorklistScratch,
};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::{ColumnIndex, RepeatedRows};
use ryo_values::domain::rows::types::{RowIndex, Table};
use std::cmp::Ordering;

/// `queued_walk`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct QueuedWalkItem;

#[cfg(test)]
impl KernelsItem for QueuedWalkItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = match lowering {
            Lowering::Worklist { .. } => true,
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
            | Lowering::Components { .. }
            | Lowering::TextMap { .. }
            | Lowering::OrderedMatch { .. }
            | Lowering::Render { .. }
            | Lowering::ScanBound { .. }
            | Lowering::Unlowered => false,
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// The three inputs of a worklist: its root rows, its edge rows and its lattice rows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WorklistInputs<'a> {
    /// The root rows.
    pub(crate) roots: InputRows<'a>,
    /// The edge rows.
    pub(crate) edges: InputRows<'a>,
    /// The lattice rows.
    pub(crate) lattice: InputRows<'a>,
}

/// Writes into `output` each distinct row of the runs' traces, in the order first written: the root's keys and root, the
/// followed edge's source, destination, order and before-state, and the destination's state after the edge's join.
/// `repeats` keys the rows written so far.
///
/// # Errors
///
/// A fault of the node's shape: a column past its input's rows, a root, source and destination of different kinds, states of
/// different kinds, an order cell that is not a number; `DuplicateOrder` for two edges of one partition, source and
/// before-state of equal order; `NotALattice` for lattice rows whose order has a cycle, no one least state, or a pair of
/// states with no one least upper bound, or for a root or edge state the lattice rows do not hold.
pub(crate) fn worklisted(
    worklist: &WorklistParts,
    inputs: WorklistInputs<'_>,
    texts: &Texts<'_>,
    scratch: &mut WorklistScratch,
    repeats: &mut KeyedRows,
    output: &mut Table,
) -> Result<Option<PortableOperatorFault>, PortableOperatorFault> {
    let kinds = [inputs.roots, inputs.edges, inputs.lattice].map(kinds_of);
    output_kinds(worklist, kinds, &mut output.kinds)?;
    scratch.fresh.clear();
    let columns = [worklist.lower, worklist.upper];
    let lattice = inputs.lattice;
    canonical_cells(
        lattice,
        columns.iter(),
        texts,
        &mut scratch.fresh,
        &mut scratch.cells,
    )?;
    tabled(&mut scratch.lattice, &scratch.cells, |row, side| {
        let column = match side {
            LatticeSide::Lower => worklist.lower,
            LatticeSide::Upper => worklist.upper,
        };
        cell_of(lattice, row, column)
    })?;
    sorted_edges(worklist, inputs.edges, texts, scratch)?;
    sorted_roots(worklist, inputs.roots, texts, scratch)?;
    repeats.cleared();
    let width = worklist.root_keys.len() + 2;
    let mut at = 0;
    while let Some(first) = scratch.roots.get(at).copied() {
        let key = |row: RowIndex| {
            scratch
                .root_cells
                .get(row.0 * width..row.0 * width + width - 1)
        };
        // A root's rows are one run: it starts at the join of the least state and every state they give it.
        let mut start = scratch.lattice.least;
        while let Some(row) = scratch
            .roots
            .get(at)
            .copied()
            .filter(|row| key(*row) == key(first))
        {
            let state = cell(&scratch.root_cells, CellPlace(row.0 * width + width - 1))?;
            start = joined(&scratch.lattice, start, place(&scratch.lattice, state)?);
            at += 1;
        }
        let run = Run {
            worklist,
            inputs,
            root: first,
        };
        ran(run, start, scratch, repeats, output)?;
    }
    Ok(None)
}

/// One run of a worklist: its columns, its rows, and the first row of its root.
#[derive(Clone, Copy)]
struct Run<'a> {
    /// The worklist's columns.
    worklist: &'a WorklistParts,
    /// Its rows.
    inputs: WorklistInputs<'a>,
    /// The first row of the run's root.
    root: RowIndex,
}

/// A place among canonical cells: a row's place times the cells of a row, plus a cell's place in its row.
#[derive(Clone, Copy)]
struct CellPlace(usize);

/// The canonical cell at `at` of `cells`.
fn cell(cells: &[Value], at: CellPlace) -> Result<Value, PortableOperatorFault> {
    cells
        .get(at.0)
        .copied()
        .ok_or(PortableOperatorFault::ShapeMismatch)
}

/// The state `entity` holds in the open run: the least state until a join raises it.
fn held(scratch: &WorklistScratch, entity: Value) -> StatePlace {
    scratch
        .held
        .find(value_hash(entity).0, |held| held.entity == entity)
        .map_or(scratch.lattice.least, |held| held.state)
}

/// Makes `entity` hold `state` in the open run.
fn hold(scratch: &mut WorklistScratch, entity: Value, state: StatePlace) {
    let hash = value_hash(entity).0;
    match scratch.held.find_mut(hash, |held| held.entity == entity) {
        Some(held) => held.state = state,
        None => {
            let _held = scratch
                .held
                .insert_unique(hash, HeldState { entity, state }, |held| {
                    value_hash(held.entity).0
                });
        }
    }
}

/// Runs one root: the queue starts as the root, holding `start`; an entity leaving it follows, in ascending order, its edges
/// of the state it holds then, each joining its after-state into the destination, which joins the queue again when its state
/// changes; each edge followed writes one row, unless an equal row was written.
fn ran(
    run: Run<'_>,
    start: StatePlace,
    scratch: &mut WorklistScratch,
    repeats: &mut KeyedRows,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let keys = run.worklist.root_keys.len();
    let (root_width, edge_width) = (keys + 2, keys + 4);
    let partition = run.root.0 * root_width..run.root.0 * root_width + keys;
    scratch.held.clear();
    scratch.queue.clear();
    let entity = cell(&scratch.root_cells, CellPlace(partition.end))?;
    hold(scratch, entity, start);
    scratch.queue.push_back(entity);
    while let Some(popped) = scratch.queue.pop_front() {
        let state = held(scratch, popped);
        let [before, _] = state_of(&scratch.lattice, state)?;
        // The edges leaving `popped` in the root's partition and in the state it holds: one run of the sorted edges.
        let target = || {
            scratch
                .root_cells
                .get(partition.clone())
                .unwrap_or(&[])
                .iter()
                .copied()
                .chain([popped, before])
        };
        let edge_run = |row: &RowIndex| {
            let cells = scratch
                .cells
                .get(row.0 * edge_width..(row.0 + 1) * edge_width)
                .unwrap_or(&[]);
            cells
                .iter()
                .take(keys + 1)
                .chain(cells.get(keys + 2))
                .copied()
        };
        let first = scratch
            .edges
            .partition_point(|row| edge_run(row).cmp(target()) == Ordering::Less);
        let end = scratch
            .edges
            .partition_point(|row| edge_run(row).cmp(target()) != Ordering::Greater);
        for at in first..end {
            let edge = *scratch
                .edges
                .get(at)
                .ok_or(PortableOperatorFault::ShapeMismatch)?;
            let destination = cell(&scratch.cells, CellPlace(edge.0 * edge_width + keys + 1))?;
            let after = place(
                &scratch.lattice,
                cell(&scratch.cells, CellPlace(edge.0 * edge_width + keys + 3))?,
            )?;
            let was = held(scratch, destination);
            let reached = joined(&scratch.lattice, was, after);
            if reached != was {
                hold(scratch, destination, reached);
                scratch.queue.push_back(destination);
            }
            let row = CellStart(output.cells.len());
            written(run, edge, [state, reached], scratch, output)?;
            counted(output, row, RepeatedRows::Dropped, repeats);
        }
    }
    Ok(())
}

/// Writes the row of an edge followed: the root's keys and root, the edge's source, destination and order, then the state the
/// source was popped in and the destination's state after the join, each state as the cell it is written as.
fn written(
    run: Run<'_>,
    edge: RowIndex,
    states: [StatePlace; 2],
    scratch: &WorklistScratch,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let worklist = run.worklist;
    let read = |rows: InputRows<'_>, row: RowIndex, column: ColumnIndex| {
        cell_of(rows, row, column).ok_or(PortableOperatorFault::NoSuchColumn { index: column })
    };
    for column in worklist.root_keys.iter().chain([&worklist.root]) {
        output
            .cells
            .push(read(run.inputs.roots, run.root, *column)?);
    }
    for column in [worklist.source, worklist.destination, worklist.order] {
        output.cells.push(read(run.inputs.edges, edge, column)?);
    }
    for state in states {
        let [_, named] = state_of(&scratch.lattice, state)?;
        output.cells.push(named);
    }
    Ok(())
}

/// Sorts the edge rows by partition, source, before-state and order into `scratch.edges`, their canonical cells (each edge's
/// partition, source, destination, before-state and after-state) in `scratch.cells`.
///
/// # Errors
///
/// `ShapeMismatch` for an order cell that is not a number, `NotALattice` for a state the lattice does not hold, and
/// `DuplicateOrder` for two edges of one partition, source and before-state of equal order.
fn sorted_edges(
    worklist: &WorklistParts,
    edges: InputRows<'_>,
    texts: &Texts<'_>,
    scratch: &mut WorklistScratch,
) -> Result<(), PortableOperatorFault> {
    let states = [
        &worklist.source,
        &worklist.destination,
        &worklist.before,
        &worklist.after,
    ];
    let columns = worklist.edge_keys.iter().chain(states);
    canonical_cells(
        edges,
        columns,
        texts,
        &mut scratch.fresh,
        &mut scratch.cells,
    )?;
    let (keys, width) = (worklist.edge_keys.len(), worklist.edge_keys.len() + 4);
    scratch.edges.clear();
    for at in 0..count_of(edges).0 {
        if !matches!(
            cell_of(edges, RowIndex(at), worklist.order),
            Some(Value::Number(_))
        ) {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
        for state in [keys + 2, keys + 3] {
            let _known = place(
                &scratch.lattice,
                cell(&scratch.cells, CellPlace(at * width + state))?,
            )?;
        }
        scratch.edges.push(RowIndex(at));
    }
    // A run is keyed by partition, source and before-state: every canonical cell but the destination and the after-state.
    let cells = &scratch.cells;
    let run = |row: RowIndex| {
        let held = cells.get(row.0 * width..(row.0 + 1) * width).unwrap_or(&[]);
        held.iter()
            .take(keys + 1)
            .chain(held.get(keys + 2))
            .copied()
    };
    let order = |row: RowIndex| cell_of(edges, row, worklist.order);
    scratch.edges.sort_unstable_by(|one, other| {
        run(*one)
            .cmp(run(*other))
            .then_with(|| order(*one).cmp(&order(*other)))
    });
    if scratch
        .edges
        .windows(2)
        .any(|pair| matches!(pair, [one, other] if run(*one).eq(run(*other)) && order(*one) == order(*other)))
    {
        return Err(PortableOperatorFault::DuplicateOrder);
    }
    Ok(())
}

/// Sorts the root rows by partition and root into `scratch.roots`, each root's rows in their order, their canonical
/// cells (each root's partition, root and state) in `scratch.root_cells`.
///
/// # Errors
///
/// `NotALattice` for a root state the lattice does not hold.
fn sorted_roots(
    worklist: &WorklistParts,
    roots: InputRows<'_>,
    texts: &Texts<'_>,
    scratch: &mut WorklistScratch,
) -> Result<(), PortableOperatorFault> {
    let columns = worklist
        .root_keys
        .iter()
        .chain([&worklist.root, &worklist.state]);
    canonical_cells(
        roots,
        columns,
        texts,
        &mut scratch.fresh,
        &mut scratch.root_cells,
    )?;
    let width = worklist.root_keys.len() + 2;
    scratch.roots.clear();
    for at in 0..count_of(roots).0 {
        let _known = place(
            &scratch.lattice,
            cell(&scratch.root_cells, CellPlace(at * width + width - 1))?,
        )?;
        scratch.roots.push(RowIndex(at));
    }
    let cells = &scratch.root_cells;
    let key = |row: RowIndex| cells.get(row.0 * width..row.0 * width + width - 1);
    // A root's rows keep their order, the first giving its cells: ties go by row, with no buffer a stable sort would take.
    scratch
        .roots
        .sort_unstable_by(|one, other| key(*one).cmp(&key(*other)).then(one.0.cmp(&other.0)));
    Ok(())
}
