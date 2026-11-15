//! The `hash_index` item of the `kernels` collection: rows keyed into the worker's one keyed-row table, with no allocation per
//! row and no key copied out, by one name column's text or by the cells of key columns.

#[cfg(test)]
use super::super::interface::{KernelsItem, Reach};
use super::super::scan_facts::item::{cell_of, count_of, rows_of};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::contract::types::PredicateOutcome;
#[cfg(test)]
use crate::domain::physical_plan::types::Lowering;
use crate::domain::physical_plan::types::file_buffers::value::{
    IndexKeyHash, IndexedRow, KeyedRows, KeyedRun, NO_ROW,
};
use crate::domain::physical_plan::types::{CellStart, KeyedRowAt, KeyedSide};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::service::hash_in;
use ryo_relations::domain::relations::service::from_bits;
use ryo_relations::domain::relations::types::{Bits, CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
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
                Lowering::Lookup { .. } | Lowering::Probe { .. } | Lowering::Group { .. }
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

/// Empties `rows` of every key and row, keeping its capacity.
pub(crate) fn emptied(rows: &mut KeyedRows) {
    cleared(rows, RowCount(0));
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
fn hash_of(at: KeyedRowAt<'_>) -> Result<IndexKeyHash, PortableOperatorFault> {
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

/// Keys the row `table` holds from cell `start`, by all its cells, into `rows`, which keys the table's rows before it in
/// order: `Holds` when no row before it holds the same cells, which then keeps it; `Fails` for a repeat, which it does not
/// key. Rows of equal cells are compared from the table's own cells, with no row copied out.
pub(crate) fn first_of_row(
    rows: &mut KeyedRows,
    table: &Table,
    start: CellStart,
) -> PredicateOutcome {
    let width = table.kinds.len();
    let held = &table.cells;
    let new = held.get(start.0..).unwrap_or_default();
    let mut hasher = rustc_hash::FxHasher::default();
    for cell in new {
        cell.hash(&mut hasher);
    }
    let hash = IndexKeyHash(hasher.finish());
    let row_at = |row: IndexedRow| {
        let at = indexed(row).0.saturating_mul(width);
        held.get(at..at.saturating_add(width)).unwrap_or_default()
    };
    if rows
        .keys
        .find(hash.0, |run| run.hash == hash && row_at(run.first) == new)
        .is_some()
    {
        return PredicateOutcome::Fails;
    }
    let row = IndexedRow(u32::try_from(rows.next.len()).unwrap_or(u32::MAX));
    rows.next.push(NO_ROW);
    insert(rows, hash, row, |_| PredicateOutcome::Fails);
    PredicateOutcome::Holds
}

/// The row after `row` of the same key, or `NO_ROW` after its last.
#[expect(
    clippy::indexing_slicing,
    reason = "the chain is made together with the rows it indexes: a row past it is a bug, which panics"
)]
pub(crate) fn next_of(rows: &KeyedRows, row: IndexedRow) -> IndexedRow {
    rows.next[usize::try_from(row.0).unwrap_or(usize::MAX)]
}
