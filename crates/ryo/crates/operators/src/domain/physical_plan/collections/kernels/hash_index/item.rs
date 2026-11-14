//! The `hash_index` item of the `kernels` collection: rows keyed into the worker's one keyed-row table, with no allocation per
//! row and no key copied out, by one name column's text or by the cells of key columns.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{cell_of, count_of, rows_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::physical_plan::types::file_buffers::value::WalkScratch;
use crate::domain::physical_plan::types::file_buffers::value::{
    IndexKeyHash, IndexedRow, KeyedRows, KeyedRun, NO_ROW,
};
#[cfg(test)]
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::{KeyedRowAt, KeyedSide};
use crate::domain::table_scratch::types::SetMembership;
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::service::hash_in;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{Bits, CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction};
use ryo_values::domain::rows::types::{RowCount, RowIndex};
use std::hash::{Hash as _, Hasher as _};

/// `hash_index`, one item of the `kernels` collection.
#[cfg(test)]
pub(crate) struct HashIndexItem;

#[cfg(test)]
impl KernelsItem for HashIndexItem {
    fn reached_by(&self, lowering: &Lowering) -> Reach {
        let runs = {
            matches!(
                lowering,
                Lowering::Lookup { .. }
                    | Lowering::Probe { .. }
                    | Lowering::Group { .. }
                    | Lowering::Reach { .. }
                    | Lowering::SeededReach { .. }
                    | Lowering::Axis { .. }
                    | Lowering::OrderedWalk { .. }
                    | Lowering::Components { .. }
            )
        };
        if runs { Reach::Runs } else { Reach::Skips }
    }
}

/// Empties `rows` for the rows of `count`, keeping its capacity.
fn cleared(rows: &mut KeyedRows, count: RowCount) {
    rows.keys.clear();
    rows.next.clear();
    rows.next.resize(count.0, NO_ROW);
}

/// Adds `row` to the key `same` finds under `hash`, or as a key of its own.
fn insert(
    rows: &mut KeyedRows,
    hash: IndexKeyHash,
    row: IndexedRow,
    same: impl Fn(&KeyedRun) -> PredicateOutcome,
) {
    match rows
        .keys
        .find_mut(hash.0, |run| same(run) == PredicateOutcome::Holds)
    {
        Some(run) => {
            if let Some(link) = rows
                .next
                .get_mut(usize::try_from(run.last.0).unwrap_or(usize::MAX))
            {
                *link = row;
            }
            run.last = row;
            run.rows = RowCount(run.rows.0.saturating_add(1));
        }
        None => {
            let _: hashbrown::hash_table::OccupiedEntry<'_, KeyedRun> = rows.keys.insert_unique(
                hash.0,
                KeyedRun {
                    hash,
                    first: row,
                    last: row,
                    rows: RowCount(1),
                },
                |run| run.hash.0,
            );
        }
    }
}

/// Whether a key's hash is `hash`.
fn hashed(run: &KeyedRun, hash: IndexKeyHash) -> PredicateOutcome {
    if run.hash == hash {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

/// Keys the relation's rows by the hash of `column`'s text into `rows`: each key's first and last row, and each row's next
/// of the same key, ascending. A cell with no text is no key.
pub(crate) fn texts(
    view: &ColumnsView<'_>,
    column: ColumnIndex,
    texts: &Texts<'_>,
    rows: &mut KeyedRows,
) {
    cleared(rows, rows_of(view));
    let Some(cells) = view
        .columns
        .get(usize::try_from(column.0).unwrap_or(usize::MAX))
    else {
        return;
    };
    for (bits, row) in cells.0.iter().zip((0..).map(IndexedRow)) {
        let Value::Symbol(symbol) = from_bits(CellKind::Symbol, Bits(*bits)) else {
            continue;
        };
        let Some(hash) = hash_in(texts.symbols, symbol) else {
            continue;
        };
        let hash = IndexKeyHash(hash.0);
        insert(rows, hash, row, |run| hashed(run, hash));
    }
}

/// The hash of a row's cells in its key columns.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past the row.
pub(crate) fn hash_of(at: KeyedRowAt<'_>) -> Result<IndexKeyHash, PortableOperatorFault> {
    let mut hasher = rustc_hash::FxHasher::default();
    for column in at.side.on {
        cell_of(at.side.rows, at.row, *column)
            .ok_or(PortableOperatorFault::NoSuchColumn { index: *column })?
            .hash(&mut hasher);
    }
    Ok(IndexKeyHash(hasher.finish()))
}

/// Whether two rows hold the same cells in their key columns, read in place.
fn same_key(left: KeyedRowAt<'_>, right: KeyedRowAt<'_>) -> PredicateOutcome {
    let same = left.side.on.len() == right.side.on.len()
        && left.side.on.iter().zip(right.side.on).all(|(one, other)| {
            cell_of(left.side.rows, left.row, *one)
                .is_some_and(|cell| cell_of(right.side.rows, right.row, *other) == Some(cell))
        });
    if same {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

/// The row an index row is.
pub(crate) fn indexed(row: IndexedRow) -> RowIndex {
    RowIndex(usize::try_from(row.0).unwrap_or(usize::MAX))
}

/// Keys every row of `side` by its cells in its key columns into `rows`: rows of equal cells share a key, compared from the
/// rows' own cells, each key's rows ascending.
///
/// # Errors
///
/// `NoSuchColumn` for a key column past a row.
pub(crate) fn cells(
    side: KeyedSide<'_>,
    rows: &mut KeyedRows,
) -> Result<(), PortableOperatorFault> {
    let count = count_of(side.rows);
    cleared(rows, count);
    for at in 0..count.0 {
        let probe = KeyedRowAt {
            side,
            row: RowIndex(at),
        };
        let hash = hash_of(probe)?;
        let row = IndexedRow(u32::try_from(at).unwrap_or(u32::MAX));
        insert(rows, hash, row, |run| {
            if run.hash == hash {
                same_key(
                    KeyedRowAt {
                        side,
                        row: indexed(run.first),
                    },
                    probe,
                )
            } else {
                PredicateOutcome::Fails
            }
        });
    }
    Ok(())
}

/// The key of `rows` whose text hashes to `hash`.
pub(crate) fn find_text(rows: &KeyedRows, hash: IndexKeyHash) -> Option<KeyedRun> {
    rows.keys
        .find(hash.0, |run| hashed(run, hash) == PredicateOutcome::Holds)
        .copied()
}

/// The key of `rows`, indexed from `indexed_side`, that `probe` holds in its key columns.
///
/// # Errors
///
/// `NoSuchColumn` for a probe key column past the row.
pub(crate) fn find_cells(
    rows: &KeyedRows,
    indexed_side: KeyedSide<'_>,
    probe: KeyedRowAt<'_>,
) -> Result<Option<KeyedRun>, PortableOperatorFault> {
    let hash = hash_of(probe)?;
    Ok(rows
        .keys
        .find(hash.0, |run| {
            run.hash == hash
                && same_key(
                    KeyedRowAt {
                        side: indexed_side,
                        row: indexed(run.first),
                    },
                    probe,
                ) == PredicateOutcome::Holds
        })
        .copied())
}

/// The key of `rows`, indexed from `indexed_side` on one column, whose cell is `key`.
///
/// # Errors
///
/// `ShapeMismatch` when the side is keyed on more or fewer columns than one.
pub(crate) fn find_value(
    rows: &KeyedRows,
    indexed_side: KeyedSide<'_>,
    key: Value,
) -> Result<Option<KeyedRun>, PortableOperatorFault> {
    let [column] = indexed_side.on else {
        return Err(PortableOperatorFault::ShapeMismatch);
    };
    let mut hasher = rustc_hash::FxHasher::default();
    key.hash(&mut hasher);
    let hash = IndexKeyHash(hasher.finish());
    Ok(rows
        .keys
        .find(hash.0, |run| {
            run.hash == hash && cell_of(indexed_side.rows, indexed(run.first), *column) == Some(key)
        })
        .copied())
}

/// The row after `row` of the same key, or `NO_ROW` after its last.
#[expect(
    clippy::indexing_slicing,
    reason = "the chain is made together with the rows it indexes: a row past it is a bug, which panics"
)]
pub(crate) fn next_of(rows: &KeyedRows, row: IndexedRow) -> IndexedRow {
    rows.next[usize::try_from(row.0).unwrap_or(usize::MAX)]
}

/// Appends to `walk.reached` what the keyed edges lead to from `start`: the edges' far ends in edge order for a parent or a
/// child; every entity met breadth first, each once (the start not met before), for an ancestor or a descendant; the first
/// one `admitted` holds for the nearest ancestor.
pub(crate) fn keyed_walk(
    edges: KeyedSide<'_>,
    index: &KeyedRows,
    to: ColumnIndex,
    direction: Direction,
    start: Value,
    walk: &mut WalkScratch,
    admitted: impl Fn(Value) -> Result<SetMembership, PortableOperatorFault>,
) -> Result<(), PortableOperatorFault> {
    if matches!(direction, Direction::Parent | Direction::Child) {
        return follow(edges, index, to, start, &mut walk.reached);
    }
    let all = direction != Direction::NearestAncestor;
    walk.visited.clear();
    walk.entities.clear();
    follow(edges, index, to, start, &mut walk.entities)?;
    while !walk.entities.is_empty() {
        walk.next.clear();
        for place in 0..walk.entities.len() {
            let entity = walk.entities[place];
            let hash = value_hash(entity).0;
            if walk.visited.find(hash, |held| *held == entity).is_some() {
                continue;
            }
            let _met = walk
                .visited
                .insert_unique(hash, entity, |held| value_hash(*held).0);
            if !all && admitted(entity)? == SetMembership::In {
                walk.reached.push(entity);
                return Ok(());
            }
            follow(edges, index, to, entity, &mut walk.next)?;
            if all {
                walk.reached.push(entity);
            }
        }
        std::mem::swap(&mut walk.entities, &mut walk.next);
    }
    Ok(())
}

/// Appends to `into` the far end of each edge keyed by `start`, in edge order.
fn follow(
    edges: KeyedSide<'_>,
    index: &KeyedRows,
    to: ColumnIndex,
    start: Value,
    into: &mut Vec<Value>,
) -> Result<(), PortableOperatorFault> {
    let mut row = find_value(index, edges, start)?.map_or(NO_ROW, |run| run.first);
    while row != NO_ROW {
        into.push(
            cell_of(edges.rows, indexed(row), to)
                .ok_or(PortableOperatorFault::NoSuchColumn { index: to })?,
        );
        row = next_of(index, row);
    }
    Ok(())
}

/// The hash a walk keeps an entity it met under.
pub(crate) fn value_hash(value: Value) -> IndexKeyHash {
    let mut hasher = rustc_hash::FxHasher::default();
    value.hash(&mut hasher);
    IndexKeyHash(hasher.finish())
}
