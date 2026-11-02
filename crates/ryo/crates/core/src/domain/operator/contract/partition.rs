//! Exact row-ID order for a borrowed partitioned population.
//! No source payload or per-partition adjacency is retained by the ordering pass.

use super::views::BorrowedRows;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::{
    service::{check_columns, mix, word_of},
    types::KeyCellsHash,
};
use ryo_values::domain::plan_parts::types::ColumnIndex;

fn hash_at(
    input: BorrowedRows<'_>,
    row: RowIndex,
    keys: &[ColumnIndex],
) -> Result<KeyCellsHash, OperatingError> {
    keys.iter().try_fold(KeyCellsHash(0), |hash, column| {
        let value = input
            .cell(row, *column)
            .ok_or(OperatingError::ShapeMismatch)?;
        Ok(mix(hash, word_of(&value)))
    })
}

fn same_key(
    input: BorrowedRows<'_>,
    left: RowIndex,
    right: RowIndex,
    keys: &[ColumnIndex],
) -> bool {
    keys.iter()
        .all(|column| input.cell(left, *column) == input.cell(right, *column))
}

/// Match `sort_rows` exactly: a clustered stream keeps its original order;
/// returning key runs sort by hash, then typed key values, then physical row ID.
pub(crate) fn ordered_positions(
    input: BorrowedRows<'_>,
    keys: &[ColumnIndex],
) -> Result<Vec<RowIndex>, OperatingError> {
    input.validate()?;
    check_columns(input.kinds(), keys)?;
    let mut ordered = Vec::with_capacity(input.count());
    let mut runs = hashbrown::HashSet::new();
    let mut head = None;
    let mut clustered = true;
    for at in 0..input.count() {
        let row = RowIndex(at);
        if head.is_none_or(|previous| !same_key(input, previous, row, keys)) {
            if !runs.insert(hash_at(input, row, keys)?) {
                clustered = false;
                break;
            }
            head = Some(row);
        }
        ordered.push(row);
    }
    if clustered {
        return Ok(ordered);
    }
    let mut pairs = (0..input.count())
        .map(|at| {
            let row = RowIndex(at);
            Ok((hash_at(input, row, keys)?, row))
        })
        .collect::<Result<Vec<_>, OperatingError>>()?;
    pairs.sort_unstable();
    ordered.clear();
    ordered.extend(pairs.iter().map(|(_, row)| *row));
    let mut start = 0;
    for tied in pairs.chunk_by(|left, right| left.0 == right.0) {
        let end = start + tied.len();
        if let Some(rows) = ordered.get_mut(start..end) {
            rows.sort_by(|left, right| {
                keys.iter()
                    .map(|column| input.cell(*left, *column).cmp(&input.cell(*right, *column)))
                    .find(|ordering| ordering.is_ne())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(left.cmp(right))
            });
        }
        start = end;
    }
    Ok(ordered)
}

#[cfg(test)]
mod tests;
