//! The `worklist` item of the `kernels` collection: from each distinct partition and root, in sorted order, a
//! first-in-first-out worklist fixpoint over a declared lattice. An entity leaving the queue follows, in ascending order, its
//! edges whose before-state is the state it holds then, each joining its after-state into the destination, which joins the
//! queue again when its state changes; every edge followed is one row, with the destination's state after the join.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{canonical_cells, cell_of, count_of, kinds_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::InputRows;
use crate::domain::physical_plan::types::keyed_rows::value::KeyedRows;
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::table_texts::types::Texts;
use crate::domain::worklist::types::{StatePlace, WorklistParts, WorklistScratch};
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowIndex, Table};

/// `worklist`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct WorklistItem;

#[cfg(test)]
impl KernelsItem for WorklistItem {
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
    let _ = repeats;
    scratch.held.clear();
    scratch.queue.clear();
    shaped(worklist, inputs, output)?;
    scratch.fresh.clear();
    tabled(worklist, inputs.lattice, texts, scratch)?;
    sorted_edges(worklist, inputs.edges, texts, scratch)?;
    sorted_roots(worklist, inputs.roots, texts, scratch)?;
    Ok(None)
}

/// Checks the columns' kinds, and gives `output` the kinds of its row.
fn shaped(
    worklist: &WorklistParts,
    inputs: WorklistInputs<'_>,
    output: &mut Table,
) -> Result<(), PortableOperatorFault> {
    let kind = |kinds: &[CellKind], column: ColumnIndex| {
        kinds
            .get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
    };
    let (roots, edges, lattice) = (
        kinds_of(inputs.roots),
        kinds_of(inputs.edges),
        kinds_of(inputs.lattice),
    );
    let root = kind(roots, worklist.root)?;
    let state = kind(roots, worklist.state)?;
    if kind(edges, worklist.source)? != root
        || kind(edges, worklist.destination)? != root
        || kind(edges, worklist.order)? != CellKind::Number
        || kind(edges, worklist.before)? != state
        || kind(edges, worklist.after)? != state
        || kind(lattice, worklist.lower)? != state
        || kind(lattice, worklist.upper)? != state
        || worklist.root_keys.len() != worklist.edge_keys.len()
    {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    output.kinds.clear();
    for (left, right) in worklist.root_keys.iter().zip(&worklist.edge_keys) {
        let left = kind(roots, *left)?;
        if left != kind(edges, *right)? {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
        output.kinds.push(left);
    }
    output
        .kinds
        .extend([root, root, root, CellKind::Number, state, state]);
    Ok(())
}

/// Tables the lattice: its states, ascending, each with the cell it is written as; the order between them, the reflexive and
/// transitive closure of the rows; and the least upper bound of every pair, each checked before any run.
///
/// # Errors
///
/// `NotALattice` when the order has a cycle, there is not one least state, or a pair has no one least upper bound.
fn tabled(
    worklist: &WorklistParts,
    lattice: InputRows<'_>,
    texts: &Texts<'_>,
    scratch: &mut WorklistScratch,
) -> Result<(), PortableOperatorFault> {
    let columns = [worklist.lower, worklist.upper];
    canonical_cells(
        lattice,
        columns.iter(),
        texts,
        &mut scratch.fresh,
        &mut scratch.cells,
    )?;
    scratch.states.clear();
    scratch.states.extend_from_slice(&scratch.cells);
    scratch.states.sort_unstable();
    scratch.states.dedup();
    let count = scratch.states.len();
    scratch.named.clear();
    for state in &scratch.states {
        let first = scratch
            .cells
            .iter()
            .position(|cell| cell == state)
            .ok_or(PortableOperatorFault::NotALattice)?;
        let column = columns
            .get(first % 2)
            .copied()
            .ok_or(PortableOperatorFault::NotALattice)?;
        scratch.named.push(
            cell_of(lattice, RowIndex(first / 2), column)
                .ok_or(PortableOperatorFault::NoSuchColumn { index: column })?,
        );
    }
    scratch.below.clear();
    scratch.below.resize(count * count, false);
    for at in 0..count {
        set(&mut scratch.below, count, at, at);
    }
    for pair in scratch.cells.chunks_exact(2) {
        let (lower, upper) = (
            place(&scratch.states, pair[0])?,
            place(&scratch.states, pair[1])?,
        );
        set(&mut scratch.below, count, slot(lower), slot(upper));
    }
    // Warshall's closure: a state below one below another is below it.
    for middle in 0..count {
        for lower in 0..count {
            if !is_below(&scratch.below, count, lower, middle) {
                continue;
            }
            for upper in 0..count {
                if is_below(&scratch.below, count, middle, upper) {
                    set(&mut scratch.below, count, lower, upper);
                }
            }
        }
    }
    let cyclic = (0..count).any(|one| {
        (0..count).any(|other| {
            one != other
                && is_below(&scratch.below, count, one, other)
                && is_below(&scratch.below, count, other, one)
        })
    });
    let least = (0..count)
        .filter(|one| (0..count).all(|other| is_below(&scratch.below, count, *one, other)))
        .count();
    if cyclic || least != 1 {
        return Err(PortableOperatorFault::NotALattice);
    }
    scratch.joins.clear();
    for one in 0..count {
        for other in 0..count {
            let join = least_upper_bound(&scratch.below, count, one, other)?;
            scratch.joins.push(join);
        }
    }
    Ok(())
}

/// The one least state above both `one` and `other`.
///
/// # Errors
///
/// `NotALattice` when there is none, or more than one.
fn least_upper_bound(
    below: &[bool],
    count: usize,
    one: usize,
    other: usize,
) -> Result<StatePlace, PortableOperatorFault> {
    let above =
        |state: usize| is_below(below, count, one, state) && is_below(below, count, other, state);
    let mut least = (0..count).filter(|state| above(*state)).filter(|state| {
        (0..count).all(|bound| !above(bound) || is_below(below, count, *state, bound))
    });
    match (least.next(), least.next()) {
        (Some(join), None) => Ok(StatePlace(u32::try_from(join).unwrap_or(u32::MAX))),
        _ => Err(PortableOperatorFault::NotALattice),
    }
}

/// The place of a state as an index.
fn slot(state: StatePlace) -> usize {
    usize::try_from(state.0).unwrap_or(usize::MAX)
}

/// Whether state `lower` is at or below state `upper`.
fn is_below(below: &[bool], count: usize, lower: usize, upper: usize) -> bool {
    below.get(lower * count + upper).copied().unwrap_or(false)
}

/// Records that state `lower` is at or below state `upper`.
fn set(below: &mut [bool], count: usize, lower: usize, upper: usize) {
    if let Some(cell) = below.get_mut(lower * count + upper) {
        *cell = true;
    }
}

/// The place of the canonical cell `state` among the lattice's states.
///
/// # Errors
///
/// `NotALattice` when the lattice does not hold it.
fn place(states: &[Value], state: Value) -> Result<StatePlace, PortableOperatorFault> {
    states
        .binary_search(&state)
        .map(|at| StatePlace(u32::try_from(at).unwrap_or(u32::MAX)))
        .map_err(|_| PortableOperatorFault::NotALattice)
}

/// The width of an edge's canonical cells: its partition, source, destination, before-state and after-state.
fn edge_width(worklist: &WorklistParts) -> usize {
    worklist.edge_keys.len() + 4
}

/// The width of a root's canonical cells: its partition, root and state.
fn root_width(worklist: &WorklistParts) -> usize {
    worklist.root_keys.len() + 2
}

/// Sorts the edge rows by partition, source, before-state and order into `scratch.edges`, their canonical cells in
/// `scratch.cells`.
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
    canonical_cells(
        edges,
        worklist.edge_keys.iter().chain([
            &worklist.source,
            &worklist.destination,
            &worklist.before,
            &worklist.after,
        ]),
        texts,
        &mut scratch.fresh,
        &mut scratch.cells,
    )?;
    let width = edge_width(worklist);
    let count = count_of(edges).0;
    scratch.edges.clear();
    for at in 0..count {
        if !matches!(
            cell_of(edges, RowIndex(at), worklist.order),
            Some(Value::Number(_))
        ) {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
        for state in scratch
            .cells
            .get(at * width + width - 2..at * width + width)
            .unwrap_or(&[])
        {
            let _known = place(&scratch.states, *state)?;
        }
        scratch.edges.push(RowIndex(at));
    }
    // A run is keyed by partition, source and before-state: every canonical cell but the destination and after-state.
    let run = |row: RowIndex| {
        let cells = scratch
            .cells
            .get(row.0 * width..row.0 * width + width)
            .unwrap_or(&[]);
        cells
            .iter()
            .take(width - 3)
            .chain(cells.get(width - 2))
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
        .any(|pair| run(pair[0]).eq(run(pair[1])) && order(pair[0]) == order(pair[1]))
    {
        return Err(PortableOperatorFault::DuplicateOrder);
    }
    Ok(())
}

/// Sorts the root rows by partition and root into `scratch.roots`, their canonical cells in `scratch.root_cells`.
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
    canonical_cells(
        roots,
        worklist
            .root_keys
            .iter()
            .chain([&worklist.root, &worklist.state]),
        texts,
        &mut scratch.fresh,
        &mut scratch.root_cells,
    )?;
    let width = root_width(worklist);
    let count = count_of(roots).0;
    scratch.roots.clear();
    for at in 0..count {
        if let Some(state) = scratch.root_cells.get(at * width + width - 1) {
            let _known = place(&scratch.states, *state)?;
        }
        scratch.roots.push(RowIndex(at));
    }
    let cells = &scratch.root_cells;
    let key = |row: RowIndex| {
        cells
            .get(row.0 * width..row.0 * width + width - 1)
            .unwrap_or(&[])
    };
    scratch
        .roots
        .sort_by(|one, other| key(*one).cmp(key(*other)));
    Ok(())
}
