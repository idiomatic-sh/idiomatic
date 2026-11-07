//! Existing typed hash-chain semantics, with reusable owned buffers.
use super::types::{KeyIndex, KeyIndexCapacities};
use crate::domain::allocation::types::ElementCapacity;
use crate::domain::fault::types::DerivedFault;
use crate::domain::key::service::word_of;
use crate::domain::key::types::{ChainLink, KeyCellsHash};
use crate::domain::rows::types::{BulkRows, RowCells};
use ryo_relations::domain::relations::types::{ColumnsView, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// Indexes the keys of the rows on the columns, reusing the buffers of a recycled index.
pub(crate) fn build(
    rows: &dyn RowCells,
    columns: &[ColumnIndex],
    recycled: Option<KeyIndex>,
) -> Result<KeyIndex, DerivedFault> {
    for column in columns {
        if usize::try_from(column.0).map_or(true, |index| index >= rows.kinds().len()) {
            return Err(DerivedFault::NoSuchColumn { column: *column });
        }
    }
    let mut out = recycled.unwrap_or_default();
    let count = rows.count();
    match rows.bulk() {
        Some(BulkRows::Stored(view)) => fill(count, columns, &mut out, |row, column| {
            stored_cell(view, row, column)
        }),
        Some(BulkRows::Selected {
            source,
            positions,
            columns: projection,
        }) => fill(count, columns, &mut out, |row, column| {
            let physical = RowIndex(positions.get(row.0)?.0 as usize);
            stored_cell(source, physical, *projection.get(column.0 as usize)?)
        }),
        Some(BulkRows::Computed(table)) => fill(count, columns, &mut out, |row, column| {
            let width = table.kinds.len();
            let cells = table
                .cells
                .get(row.0.checked_mul(width)?..row.0.checked_add(1)?.checked_mul(width)?)?;
            cells.get(column.0 as usize).copied()
        }),
        None => fill(count, columns, &mut out, |row, column| rows.cell(row, column)),
    }
    let buckets = count.0.next_power_of_two().max(1);
    out.heads.clear();
    out.heads.resize(buckets, ChainLink(0));
    out.next.clear();
    out.next.resize(count.0, ChainLink(0));
    for (place, (hash, link)) in out.hashes.iter().zip(out.next.iter_mut()).enumerate().rev() {
        let bucket = usize::try_from(hash.0).unwrap_or(0) & (buckets - 1);
        if let Some(head) = out.heads.get_mut(bucket) {
            *link = *head;
            *head = ChainLink(u32::try_from(place + 1).unwrap_or(u32::MAX));
        }
    }
    Ok(out)
}

/// The capacity of each buffer of the index.
#[must_use]
pub fn capacities(index: &KeyIndex) -> KeyIndexCapacities {
    KeyIndexCapacities {
        rows: ElementCapacity(index.rows.capacity()),
        hashes: ElementCapacity(index.hashes.capacity()),
        heads: ElementCapacity(index.heads.capacity()),
        links: ElementCapacity(index.next.capacity()),
        sorted: ElementCapacity(index.sorted.capacity()),
        pairs: ElementCapacity(index.pairs.capacity()),
        runs: ElementCapacity(index.runs.capacity()),
    }
}

fn stored_cell(view: ColumnsView<'_>, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    let at = column.0 as usize;
    let kind = *view.kinds.get(at)?;
    let bits = *view.columns.get(at)?.0.get(row.0)?;
    Some(ryo_relations::domain::relations::service::from_bits(
        kind,
        ryo_relations::domain::relations::types::Bits(bits),
    ))
}

fn fill(
    count: RowCount,
    columns: &[ColumnIndex],
    out: &mut KeyIndex,
    cell: impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
) {
    out.rows.clear();
    out.rows.extend((0..count.0).map(RowIndex));
    out.hashes.clear();
    out.hashes.extend(out.rows.iter().map(|row| {
        KeyCellsHash(columns.iter().fold(0_u64, |hash, column| {
            (hash.rotate_left(5) ^ cell(*row, *column).map_or(0, |value| word_of(&value).0))
                .wrapping_mul(0x517c_c1b7_2722_0a95)
        }))
    }));
}
