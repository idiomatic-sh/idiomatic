//! The one lowering of a worklist, the columns of its roots, edges and lattice; the kinds those columns must have and its rows
//! have; and its declared lattice, tabled once before any run and read by place.
use super::types::{
    LatticeSide, LatticeTable, OrderPlace, StateOrder, StatePlace, WorklistParts, WorklistShape,
};
use crate::domain::contract::types::PortableOperatorFault;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// The one lowering of a worklist: its root, edge and lattice columns.
pub(crate) fn lower(shape: WorklistShape<'_>) -> WorklistParts {
    let [source, destination, order, before, after] = shape.edge_columns;
    let [lower, upper] = shape.lattice_columns;
    WorklistParts {
        root: shape.root,
        state: shape.state,
        source,
        destination,
        order,
        before,
        after,
        lower,
        upper,
        root_keys: shape.root_keys.to_vec(),
        edge_keys: shape.edge_keys.to_vec(),
    }
}

/// Checks the kinds of a worklist's columns against `kinds`, the cell kinds of its roots, edges and lattice, and writes into
/// `output` the kinds of its row: the roots' keys and root, the edges' source, destination and order, the before-state and the
/// state reached.
///
/// # Errors
///
/// `NoSuchColumn` for a column past its input's kinds; `ShapeMismatch` for a root, source and destination of different
/// kinds, an order that is not a number, states of different kinds, or partitions of different widths or kinds.
pub(crate) fn output_kinds(
    worklist: &WorklistParts,
    [roots, edges, lattice]: [&[CellKind]; 3],
    output: &mut Vec<CellKind>,
) -> Result<(), PortableOperatorFault> {
    let kind = |kinds: &[CellKind], column: ColumnIndex| {
        kinds
            .get(usize::try_from(column.0).unwrap_or(usize::MAX))
            .copied()
            .ok_or(PortableOperatorFault::NoSuchColumn { index: column })
    };
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
    output.clear();
    for (left, right) in worklist.root_keys.iter().zip(&worklist.edge_keys) {
        let left = kind(roots, *left)?;
        if left != kind(edges, *right)? {
            return Err(PortableOperatorFault::ShapeMismatch);
        }
        output.push(left);
    }
    output.extend([root, root, root, CellKind::Number, state, state]);
    Ok(())
}

/// Tables `lattice` from the canonical cells of its rows, `cells` (each row's lower state, then its upper): its states,
/// ascending, each written as `original` gives the first cell of its text, by its row and side; the order between them, the
/// reflexive and transitive closure of the rows; the least upper bound of every two, each checked before any run; and the
/// least state.
///
/// # Errors
///
/// `NotALattice` when the order has a cycle, there is not one least state, or two states have no one least upper bound.
pub(crate) fn tabled(
    lattice: &mut LatticeTable,
    cells: &[Value],
    original: impl Fn(RowIndex, LatticeSide) -> Option<Value>,
) -> Result<(), PortableOperatorFault> {
    lattice.states.clear();
    lattice.states.extend_from_slice(cells);
    lattice.states.sort_unstable();
    lattice.states.dedup();
    lattice.named.clear();
    for state in &lattice.states {
        let first = cells.iter().position(|cell| cell == state);
        let named = first.and_then(|at| {
            let side = if at % 2 == 0 {
                LatticeSide::Lower
            } else {
                LatticeSide::Upper
            };
            original(RowIndex(at / 2), side)
        });
        lattice
            .named
            .push(named.ok_or(PortableOperatorFault::NotALattice)?);
    }
    let count = lattice.states.len();
    lattice.below.clear();
    lattice.below.resize(count * count, StateOrder::Unordered);
    for state in places(lattice) {
        ordered(lattice, state, state);
    }
    for pair in cells.chunks_exact(2) {
        if let [lower, upper] = pair {
            let (lower, upper) = (place(lattice, *lower)?, place(lattice, *upper)?);
            ordered(lattice, lower, upper);
        }
    }
    // Warshall's closure: a state below one below another is below it.
    for middle in places(lattice) {
        for lower in places(lattice) {
            if order(lattice, lower, middle) == StateOrder::Unordered {
                continue;
            }
            for upper in places(lattice) {
                if order(lattice, middle, upper) == StateOrder::AtOrBelow {
                    ordered(lattice, lower, upper);
                }
            }
        }
    }
    let cyclic = places(lattice).any(|one| {
        places(lattice).any(|other| {
            one != other
                && order(lattice, one, other) == StateOrder::AtOrBelow
                && order(lattice, other, one) == StateOrder::AtOrBelow
        })
    });
    let mut least = places(lattice).filter(|one| {
        places(lattice).all(|other| order(lattice, *one, other) == StateOrder::AtOrBelow)
    });
    let (Some(bottom), None, false) = (least.next(), least.next(), cyclic) else {
        return Err(PortableOperatorFault::NotALattice);
    };
    lattice.least = bottom;
    lattice.joins.clear();
    for one in places(lattice) {
        for other in places(lattice) {
            let join = least_upper_bound(lattice, one, other)?;
            lattice.joins.push(join);
        }
    }
    Ok(())
}

/// The least upper bound of `one` and `other`, two states of the tabled `lattice`.
pub(crate) fn joined(lattice: &LatticeTable, one: StatePlace, other: StatePlace) -> StatePlace {
    let at = usize::try_from(one.0).unwrap_or(usize::MAX) * lattice.states.len()
        + usize::try_from(other.0).unwrap_or(usize::MAX);
    lattice.joins.get(at).copied().unwrap_or(one)
}

/// The place of the canonical cell `state` among the states of `lattice`.
///
/// # Errors
///
/// `NotALattice` when the lattice does not hold it.
pub(crate) fn place(
    lattice: &LatticeTable,
    state: Value,
) -> Result<StatePlace, PortableOperatorFault> {
    lattice
        .states
        .binary_search(&state)
        .map(|at| StatePlace(u32::try_from(at).unwrap_or(u32::MAX)))
        .map_err(|_| PortableOperatorFault::NotALattice)
}

/// The canonical cell of the state at `state`, and the cell it is written as.
///
/// # Errors
///
/// `NotALattice` for a place past the lattice's states.
pub(crate) fn state_of(
    lattice: &LatticeTable,
    state: StatePlace,
) -> Result<[Value; 2], PortableOperatorFault> {
    let at = usize::try_from(state.0).unwrap_or(usize::MAX);
    match (lattice.states.get(at), lattice.named.get(at)) {
        (Some(canonical), Some(named)) => Ok([*canonical, *named]),
        _ => Err(PortableOperatorFault::NotALattice),
    }
}

/// The one least state above both `one` and `other`.
///
/// # Errors
///
/// `NotALattice` when there is none, or more than one.
fn least_upper_bound(
    lattice: &LatticeTable,
    one: StatePlace,
    other: StatePlace,
) -> Result<StatePlace, PortableOperatorFault> {
    let above = |state: StatePlace| {
        order(lattice, one, state) == StateOrder::AtOrBelow
            && order(lattice, other, state) == StateOrder::AtOrBelow
    };
    let mut least = places(lattice)
        .filter(|state| above(*state))
        .filter(|state| {
            places(lattice).all(|bound| {
                !above(bound) || order(lattice, *state, bound) == StateOrder::AtOrBelow
            })
        });
    match (least.next(), least.next()) {
        (Some(join), None) => Ok(join),
        _ => Err(PortableOperatorFault::NotALattice),
    }
}

/// Every state of `lattice`, by its place, ascending.
fn places(lattice: &LatticeTable) -> impl Iterator<Item = StatePlace> + use<> {
    (0..u32::try_from(lattice.states.len()).unwrap_or(u32::MAX)).map(StatePlace)
}

/// Where `lattice` records whether `lower` is at or below `upper`.
fn order_at(lattice: &LatticeTable, lower: StatePlace, upper: StatePlace) -> Option<OrderPlace> {
    let count = lattice.states.len();
    let lower = usize::try_from(lower.0).unwrap_or(usize::MAX);
    let upper = usize::try_from(upper.0).unwrap_or(usize::MAX);
    (lower < count && upper < count).then_some(OrderPlace(lower * count + upper))
}

/// Whether `lower` is at or below `upper` in `lattice`.
fn order(lattice: &LatticeTable, lower: StatePlace, upper: StatePlace) -> StateOrder {
    order_at(lattice, lower, upper)
        .and_then(|at| lattice.below.get(at.0).copied())
        .unwrap_or(StateOrder::Unordered)
}

/// Records in `lattice` that `lower` is at or below `upper`.
fn ordered(lattice: &mut LatticeTable, lower: StatePlace, upper: StatePlace) {
    if let Some(at) = order_at(lattice, lower, upper)
        && let Some(cell) = lattice.below.get_mut(at.0)
    {
        *cell = StateOrder::AtOrBelow;
    }
}
