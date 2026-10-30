//! The `table_dedup` service: keeping a row once among the rows of a table, with no IO and no ports.

use hashbrown::HashTable;
use hashbrown::hash_table::OccupiedEntry;
use ryo_language::domain::relations::types::{CellKind, Value};

use super::types::RowFound;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::service::{mix, word_of};
use crate::domain::table_model::types::{Addition, KeyCellsHash, RowCount, Table};

/// The row `index` of `cells`, rows of as many cells as `kinds` has columns; empty past the last row.
fn row_at<'c>(cells: &'c [Value], kinds: &[CellKind], index: RowIndex) -> &'c [Value] {
    let width = kinds.len();
    cells
        .get(index.0 * width..(index.0 + 1) * width)
        .unwrap_or_default()
}

/// Finds `row` among the rows of `kept`, which has the columns of the rows, and adds it when it is not there: where it is, and
/// whether it was added.
pub(crate) fn find_or_add(
    kept: &mut Table,
    seen: &mut HashTable<RowIndex>,
    row: &[Value],
) -> RowFound {
    let hash_of = |cells: &[Value], index: RowIndex| {
        row_at(cells, &kept.kinds, index)
            .iter()
            .fold(KeyCellsHash(0), |hash, value| mix(hash, word_of(value)))
            .0
    };
    let hash = row
        .iter()
        .fold(KeyCellsHash(0), |hash, value| mix(hash, word_of(value)))
        .0;
    if let Some(found) = seen.find(hash, |each| row_at(&kept.cells, &kept.kinds, *each) == row) {
        return RowFound {
            at: *found,
            added: Addition::Existing,
        };
    }
    let at = RowIndex(kept.count.0);
    kept.cells.extend_from_slice(row);
    kept.count = RowCount(kept.count.0 + 1);
    let _: OccupiedEntry<'_, RowIndex> =
        seen.insert_unique(hash, at, |each| hash_of(&kept.cells, *each));
    RowFound {
        at,
        added: Addition::Added,
    }
}

/// Whether `row` is new among the rows of `kept`: if it is, it is added to `kept`.
pub(crate) fn first_time(
    kept: &mut Table,
    seen: &mut HashTable<RowIndex>,
    row: &[Value],
) -> Addition {
    find_or_add(kept, seen, row).added
}

/// Drops each repeated row of the table, the first of its kind kept, in order, in place.
pub(crate) fn distinct(table: &mut Table, seen: &mut HashTable<RowIndex>) {
    let width = table.kinds.len();
    let hash_of = |cells: &[Value], index: RowIndex| {
        row_at(cells, &table.kinds, index)
            .iter()
            .fold(KeyCellsHash(0), |hash, value| mix(hash, word_of(value)))
            .0
    };
    seen.clear();
    let mut kept = 0;
    for index in 0..table.count.0 {
        let hash = hash_of(&table.cells, RowIndex(index));
        let row = row_at(&table.cells, &table.kinds, RowIndex(index));
        if seen
            .find(hash, |each| {
                row_at(&table.cells, &table.kinds, *each) == row
            })
            .is_some()
        {
            continue;
        }
        table
            .cells
            .copy_within(index * width..(index + 1) * width, kept * width);
        let _: OccupiedEntry<'_, RowIndex> =
            seen.insert_unique(hash, RowIndex(kept), |each| hash_of(&table.cells, *each));
        kept += 1;
    }
    table.cells.truncate(kept * width);
    table.count = RowCount(kept);
}
