//! Existing typed hash-chain semantics, with reusable owned buffers.
use super::contract::types::{BulkRows, ChainLink, DerivedFault, KeyCellsHash, KeyIndex, RowCells};
use ryo_relations::domain::relations::types::{Flag, Integer, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;
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
    match rows.bulk() {
        Some(BulkRows::Stored(view)) => fill(rows.count(), columns, &mut out, |row, column| {
            stored_cell(view, row, column)
        }),
        Some(BulkRows::Selected {
            source,
            positions,
            columns: projection,
        }) => fill(rows.count(), columns, &mut out, |row, column| {
            let physical = usize::try_from(*positions.get(row)?).ok()?;
            stored_cell(source, physical, *projection.get(column.0 as usize)?)
        }),
        Some(BulkRows::Computed(table)) => fill(rows.count(), columns, &mut out, |row, column| {
            let width = table.kinds.len();
            let cells = table
                .cells
                .get(row.checked_mul(width)?..row.checked_add(1)?.checked_mul(width)?)?;
            cells.get(column.0 as usize).copied()
        }),
        None => fill(rows.count(), columns, &mut out, |row, column| {
            rows.cell(row, column)
        }),
    }
    let buckets = rows.count().next_power_of_two().max(1);
    out.heads.clear();
    out.heads.resize(buckets, ChainLink(0));
    out.next.clear();
    out.next.resize(rows.count(), ChainLink(0));
    for (place, (hash, link)) in out.hashes.iter().zip(out.next.iter_mut()).enumerate().rev() {
        let bucket = usize::try_from(hash.0).unwrap_or(0) & (buckets - 1);
        if let Some(head) = out.heads.get_mut(bucket) {
            *link = *head;
            *head = ChainLink(u32::try_from(place + 1).unwrap_or(u32::MAX));
        }
    }
    Ok(out)
}

fn stored_cell(
    view: ryo_relations::domain::relations::types::ColumnsView<'_>,
    row: usize,
    column: ColumnIndex,
) -> Option<Value> {
    let at = usize::try_from(column.0).ok()?;
    let kind = *view.kinds.get(at)?;
    let bits = *view.columns.get(at)?.0.get(row)?;
    Some(ryo_relations::domain::relations::service::from_bits(
        kind,
        ryo_relations::domain::relations::types::Bits(bits),
    ))
}
fn fill(
    count: usize,
    columns: &[ColumnIndex],
    out: &mut KeyIndex,
    cell: impl Fn(usize, ColumnIndex) -> Option<Value>,
) {
    out.rows.clear();
    out.rows.extend((0..count).map(RowIndex));
    out.hashes.clear();
    out.hashes.extend(out.rows.iter().map(|row| {
        KeyCellsHash(columns.iter().fold(0_u64, |hash, column| {
            (hash.rotate_left(5)
                ^ cell(row.0, *column).map_or(0, |value| super::contract::key::word_of(&value).0))
            .wrapping_mul(0x517c_c1b7_2722_0a95)
        }))
    }));
}
