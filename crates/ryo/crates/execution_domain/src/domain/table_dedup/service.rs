//! The `table_dedup` service: keeping a row once among the rows of a table, with no IO and no ports.

use hashbrown::HashTable;
use hashbrown::hash_table::OccupiedEntry;
use ryo_relations::domain::relations::types::{CellKind, Value};

use super::types::RowFound;

use crate::domain::table_model::types::Addition;

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
        ryo_derived::domain::key::service::hash_row(
            row_at(cells, &kept.kinds, index),
            ryo_derived::domain::key::types::On::Whole,
        )
        .0
    };
    let hash = ryo_derived::domain::key::service::hash_row(
        row,
        ryo_derived::domain::key::types::On::Whole,
    )
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

use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
