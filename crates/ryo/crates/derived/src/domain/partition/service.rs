//! Typed partition order; clustered order is deliberately distinct from RawKeyOrder.
use super::types::KeyRelation;
use crate::domain::fault::types::DerivedFault;
use crate::domain::key::service::{mix, word_of};
use crate::domain::key::types::{HashedRow, KeyCellsHash};
use crate::domain::row_cells::types::RowCells;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

fn check_columns(
    kinds: &[ryo_relations::domain::relations::types::CellKind],
    columns: &[ColumnIndex],
) -> Result<(), DerivedFault> {
    for column in columns {
        if usize::try_from(column.0).map_or(true, |at| at >= kinds.len()) {
            return Err(DerivedFault::NoSuchColumn { column: *column });
        }
    }
    Ok(())
}

fn hash_at(
    input: &dyn RowCells,
    row: RowIndex,
    keys: &[ColumnIndex],
) -> Result<KeyCellsHash, DerivedFault> {
    keys.iter().try_fold(KeyCellsHash(0), |hash, column| {
        let value = input
            .cell(row, *column)
            .ok_or(DerivedFault::ShapeMismatch)?;
        Ok(mix(hash, word_of(&value)))
    })
}

fn same_key(
    input: &dyn RowCells,
    left: RowIndex,
    right: RowIndex,
    keys: &[ColumnIndex],
) -> KeyRelation {
    if keys
        .iter()
        .all(|column| input.cell(left, *column) == input.cell(right, *column))
    {
        KeyRelation::Same
    } else {
        KeyRelation::Different
    }
}

/// Match `sort_rows` exactly: a clustered stream keeps its original order;
/// returning key runs sort by hash, then typed key values, then physical row ID.
pub(crate) fn ordered_positions(
    input: &dyn RowCells,
    keys: &[ColumnIndex],
) -> Result<Vec<RowIndex>, DerivedFault> {
    input.validate()?;
    check_columns(input.kinds(), keys)?;
    let count = input.count().0;
    let mut ordered = Vec::with_capacity(count);
    let mut runs = hashbrown::HashSet::new();
    let mut head = None;
    let mut clustered = true;
    for at in 0..count {
        let row = RowIndex(at);
        if head.is_none_or(|previous| same_key(input, previous, row, keys) == KeyRelation::Different)
        {
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
    let mut pairs = (0..count)
        .map(|at| {
            let row = RowIndex(at);
            Ok(HashedRow {
                hash: hash_at(input, row, keys)?,
                row,
            })
        })
        .collect::<Result<Vec<_>, DerivedFault>>()?;
    pairs.sort_unstable();
    ordered.clear();
    ordered.extend(pairs.iter().map(|pair| pair.row));
    let mut start = 0;
    for tied in pairs.chunk_by(|left, right| left.hash == right.hash) {
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
