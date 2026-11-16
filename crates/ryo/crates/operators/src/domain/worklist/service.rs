//! The one lowering of a worklist, the columns of its roots, edges and lattice; the kinds those columns must have and its rows
//! have; and its declared lattice, tabled once before any run and read by place.
use super::types::{
    Cover, CoverPlace, LatticeSide, LatticeTable, RankVisit, StatePlace, StateRank, WalkFrame,
    WorklistParts, WorklistShape,
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
/// least state. One walk ranks the states lower first. Then, the lower-ranked state of each pair first, from the highest
/// rank down, a pair's least upper bound is the lowest-ranked of the joins of the other with the lower one's upper rows,
/// when it is below each of them (a state is below another when their join is the other): O(S x rows) for S states
/// words of S/64 bits for S states.
///
/// # Errors
///
/// `NotALattice` when there are states and the order has a cycle, there is not one least state, or two states have no one
/// least upper bound; a lattice of no state is none of these, and holds no root or edge state.
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
    lattice.joins.clear();
    let count = lattice.states.len();
    // No state is no lattice to check: it holds nothing, so any root or edge state is refused where it is looked up.
    if count == 0 {
        lattice.least = StatePlace::default();
        return Ok(());
    }
    lattice.covers.clear();
    for pair in cells.chunks_exact(2) {
        if let [lower, upper] = pair {
            let (lower, upper) = (place(lattice, *lower)?, place(lattice, *upper)?);
            if lower != upper {
                lattice.covers.push(Cover { lower, upper });
            }
        }
    }
    lattice.covers.sort_unstable();
    lattice.covers.dedup();
    ranked(lattice)?;
    lattice.joins.clear();
    lattice.joins.resize(count * count, StatePlace(u32::MAX));
    for low in (0..count).rev() {
        let one = *lattice
            .by_rank
            .get(low)
            .ok_or(PortableOperatorFault::NotALattice)?;
        for high in low..count {
            let other = *lattice
                .by_rank
                .get(high)
                .ok_or(PortableOperatorFault::NotALattice)?;
            let join = least_upper_bound(lattice, [one, other])?;
            for (left, right) in [(one, other), (other, one)] {
                let at = usize::try_from(left.0).unwrap_or(usize::MAX) * count
                    + usize::try_from(right.0).unwrap_or(usize::MAX);
                if let Some(slot) = lattice.joins.get_mut(at) {
                    *slot = join;
                }
            }
        }
    }
    // The least state is the lowest-ranked one, when its join with every state is that state.
    let bottom = *lattice
        .by_rank
        .first()
        .ok_or(PortableOperatorFault::NotALattice)?;
    for at in 0..count {
        let state = StatePlace(u32::try_from(at).unwrap_or(u32::MAX));
        if join_of(lattice, bottom, state)? != state {
            return Err(PortableOperatorFault::NotALattice);
        }
    }
    lattice.least = bottom;
    Ok(())
}

/// Ranks the states lower first, walking the rows depth first: a state is ranked when every state above it is, so ranking in
/// reverse finish order puts each before the states above it.
///
/// # Errors
///
/// `NotALattice` when the rows close a cycle.
fn ranked(lattice: &mut LatticeTable) -> Result<(), PortableOperatorFault> {
    let count = lattice.states.len();
    lattice.visits.clear();
    lattice.visits.resize(count, RankVisit::Unseen);
    lattice.ranks.clear();
    lattice.ranks.resize(count, StateRank::default());
    lattice.by_rank.clear();
    lattice.by_rank.resize(count, StatePlace::default());
    lattice.frames.clear();
    let mut finished = 0;
    for start in 0..count {
        let start = StatePlace(u32::try_from(start).unwrap_or(u32::MAX));
        if visit(lattice, start)? != RankVisit::Unseen {
            continue;
        }
        opened(lattice, start);
        while let Some(frame) = lattice.frames.last().copied() {
            match lattice
                .covers
                .get(frame.next.0)
                .filter(|cover| cover.lower == frame.state)
            {
                Some(cover) => {
                    let upper = cover.upper;
                    if let Some(open) = lattice.frames.last_mut() {
                        open.next = CoverPlace(frame.next.0 + 1);
                    }
                    match visit(lattice, upper)? {
                        RankVisit::Open => return Err(PortableOperatorFault::NotALattice),
                        RankVisit::Unseen => opened(lattice, upper),
                        RankVisit::Done => {}
                    }
                }
                None => {
                    let _done = lattice.frames.pop();
                    let rank = StateRank(u32::try_from(count - 1 - finished).unwrap_or(u32::MAX));
                    finished += 1;
                    done(lattice, frame.state, rank);
                }
            }
        }
    }
    Ok(())
}

/// Where the walk is with `state`.
///
/// # Errors
///
/// `NotALattice` for a place past the states.
fn visit(lattice: &LatticeTable, state: StatePlace) -> Result<RankVisit, PortableOperatorFault> {
    lattice
        .visits
        .get(usize::try_from(state.0).unwrap_or(usize::MAX))
        .copied()
        .ok_or(PortableOperatorFault::NotALattice)
}

/// Opens `state`: the walk follows its rows, from its first.
fn opened(lattice: &mut LatticeTable, state: StatePlace) {
    if let Some(visit) = lattice
        .visits
        .get_mut(usize::try_from(state.0).unwrap_or(usize::MAX))
    {
        *visit = RankVisit::Open;
    }
    let next = CoverPlace(lattice.covers.partition_point(|cover| cover.lower < state));
    lattice.frames.push(WalkFrame { state, next });
}

/// Ranks `state` at `rank`, every state above it ranked.
fn done(lattice: &mut LatticeTable, state: StatePlace, rank: StateRank) {
    let at = usize::try_from(state.0).unwrap_or(usize::MAX);
    if let Some(slot) = lattice.ranks.get_mut(at) {
        *slot = rank;
    }
    if let Some(visit) = lattice.visits.get_mut(at) {
        *visit = RankVisit::Done;
    }
    if let Some(slot) = lattice
        .by_rank
        .get_mut(usize::try_from(rank.0).unwrap_or(usize::MAX))
    {
        *slot = state;
    }
}

/// The join of `one` and `other` the table holds.
///
/// # Errors
///
/// `NotALattice` when the table holds none for them.
fn join_of(
    lattice: &LatticeTable,
    one: StatePlace,
    other: StatePlace,
) -> Result<StatePlace, PortableOperatorFault> {
    let at = usize::try_from(one.0).unwrap_or(usize::MAX) * lattice.states.len()
        + usize::try_from(other.0).unwrap_or(usize::MAX);
    lattice
        .joins
        .get(at)
        .copied()
        .filter(|join| join.0 != u32::MAX)
        .ok_or(PortableOperatorFault::NotALattice)
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

/// The one least state above both states of `pair`, the first the lower-ranked or equal: the state itself for a state and
/// itself; else the lowest-ranked of the joins of the other with the first's upper rows (each above the first, so already
/// tabled), when it is below each of them.
///
/// # Errors
///
/// `NotALattice` when the first has no upper row (nothing is above both), or the lowest-ranked join is not below another: two
/// upper bounds with none below the other.
fn least_upper_bound(
    lattice: &LatticeTable,
    [one, other]: [StatePlace; 2],
) -> Result<StatePlace, PortableOperatorFault> {
    if one == other {
        return Ok(one);
    }
    let first = lattice.covers.partition_point(|cover| cover.lower < one);
    let end = lattice.covers.partition_point(|cover| cover.lower <= one);
    let rank = |state: StatePlace| {
        lattice
            .ranks
            .get(usize::try_from(state.0).unwrap_or(usize::MAX))
            .copied()
            .ok_or(PortableOperatorFault::NotALattice)
    };
    let mut least: Option<StatePlace> = None;
    for cover in lattice.covers.get(first..end).unwrap_or(&[]) {
        let candidate = join_of(lattice, cover.upper, other)?;
        least = match least {
            Some(held) if rank(held)? <= rank(candidate)? => Some(held),
            _ => Some(candidate),
        };
    }
    let join = least.ok_or(PortableOperatorFault::NotALattice)?;
    // The join is below every other candidate, or there are two upper bounds with none below the other.
    for cover in lattice.covers.get(first..end).unwrap_or(&[]) {
        let candidate = join_of(lattice, cover.upper, other)?;
        if join_of(lattice, join, candidate)? != candidate {
            return Err(PortableOperatorFault::NotALattice);
        }
    }
    Ok(join)
}
