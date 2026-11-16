//! The one lowering of a worklist, the columns of its roots, edges and lattice; the kinds those columns must have and its rows
//! have; and its declared lattice, tabled once before any run and read by place.
use super::types::{
    Cover, CoverPlace, LatticeSide, LatticeTable, OrderWord, RankVisit, SetWords, StatePlace,
    StateRank, WalkFrame, WorklistParts, WorklistShape,
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
/// least state. One walk ranks the states lower first and gathers each one's set of the states above it, as bits by rank, so
/// a pair's least upper bound is the lowest-ranked state above both, if every state above both is above it: O(rows + S^2)
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
    let words = SetWords(count.div_ceil(64));
    // The least state is the one every state is at or above.
    let held = |at| {
        lattice
            .above
            .get(at * words.0..(at + 1) * words.0)
            .unwrap_or(&[])
            .iter()
            .fold(0, |held, word| held + word.0.count_ones() as usize)
    };
    let mut full = (0..count).filter(|at| held(*at) == count);
    let (Some(bottom), None) = (full.next(), full.next()) else {
        return Err(PortableOperatorFault::NotALattice);
    };
    lattice.least = StatePlace(u32::try_from(bottom).unwrap_or(u32::MAX));
    for one in 0..count {
        for other in 0..count {
            let pair = [one, other].map(|at| StatePlace(u32::try_from(at).unwrap_or(u32::MAX)));
            let join = least_upper_bound(lattice, words, pair)?;
            lattice.joins.push(join);
        }
    }
    Ok(())
}

/// Ranks the states lower first and gives each its set of the states at or above it, walking the rows depth first: a state
/// is ranked when every state above it is, so ranking in reverse finish order puts each before the states above it.
///
/// # Errors
///
/// `NotALattice` when the rows close a cycle.
fn ranked(lattice: &mut LatticeTable) -> Result<(), PortableOperatorFault> {
    let count = lattice.states.len();
    let words = SetWords(count.div_ceil(64));
    lattice.visits.clear();
    lattice.visits.resize(count, RankVisit::Unseen);
    lattice.ranks.clear();
    lattice.ranks.resize(count, StateRank::default());
    lattice.by_rank.clear();
    lattice.by_rank.resize(count, StatePlace::default());
    lattice.above.clear();
    lattice.above.resize(count * words.0, OrderWord::default());
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
                    done(lattice, frame.state, rank, words);
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

/// Ranks `state` at `rank`, every state above it ranked, and gives it the set of them and itself.
fn done(lattice: &mut LatticeTable, state: StatePlace, rank: StateRank, words: SetWords) {
    let (at, rank_at, words) = (
        usize::try_from(state.0).unwrap_or(usize::MAX),
        usize::try_from(rank.0).unwrap_or(usize::MAX),
        words.0,
    );
    if let Some(slot) = lattice.ranks.get_mut(at) {
        *slot = rank;
    }
    if let Some(visit) = lattice.visits.get_mut(at) {
        *visit = RankVisit::Done;
    }
    if let Some(slot) = lattice.by_rank.get_mut(rank_at) {
        *slot = state;
    }
    if let Some(word) = lattice.above.get_mut(at * words + rank_at / 64) {
        word.0 |= 1 << (rank_at % 64);
    }
    let first = lattice.covers.partition_point(|cover| cover.lower < state);
    let end = lattice.covers.partition_point(|cover| cover.lower <= state);
    for cover in first..end {
        let Some(upper) = lattice.covers.get(cover).map(|cover| cover.upper) else {
            continue;
        };
        let upper = usize::try_from(upper.0).unwrap_or(usize::MAX);
        for word in 0..words {
            if let Some(bits) = lattice.above.get(upper * words + word).copied()
                && let Some(own) = lattice.above.get_mut(at * words + word)
            {
                own.0 |= bits.0;
            }
        }
    }
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

/// The one least state above both states of `pair`: the lowest-ranked state above both, when every state above both is
/// above it.
///
/// # Errors
///
/// `NotALattice` when no state is above both, or the lowest-ranked one is not below every other.
fn least_upper_bound(
    lattice: &LatticeTable,
    words: SetWords,
    pair: [StatePlace; 2],
) -> Result<StatePlace, PortableOperatorFault> {
    let row = |state: StatePlace| {
        let at = usize::try_from(state.0).unwrap_or(usize::MAX);
        lattice
            .above
            .get(at * words.0..(at + 1) * words.0)
            .unwrap_or(&[])
    };
    let [one, other] = pair.map(row);
    let rank = one
        .iter()
        .zip(other)
        .enumerate()
        .find_map(|(word, (one, other))| {
            let both = one.0 & other.0;
            (both != 0).then(|| word * 64 + both.trailing_zeros() as usize)
        })
        .ok_or(PortableOperatorFault::NotALattice)?;
    let join = lattice
        .by_rank
        .get(rank)
        .copied()
        .ok_or(PortableOperatorFault::NotALattice)?;
    // Every state above both is above the candidate, or the candidate is only one minimal upper bound among several.
    let above = row(join);
    if one
        .iter()
        .zip(other)
        .zip(above)
        .all(|((one, other), above)| one.0 & other.0 & !above.0 == 0)
    {
        Ok(join)
    } else {
        Err(PortableOperatorFault::NotALattice)
    }
}
