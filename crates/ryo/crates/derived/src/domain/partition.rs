//! Typed partition order; clustered order is deliberately distinct from RawKeyOrder.
use super::contract::types::{DerivedFault, RowCells};
use ryo_values::domain::plan_parts::types::ColumnIndex;
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
fn hash_at(input: &dyn RowCells, row: usize, keys: &[ColumnIndex]) -> Result<u64, DerivedFault> {
    keys.iter().try_fold(0_u64, |hash, column| {
        let value = input
            .cell(row, *column)
            .ok_or(DerivedFault::ShapeMismatch)?;
        Ok(
            (hash.rotate_left(5) ^ super::equality::word(value))
                .wrapping_mul(0x517c_c1b7_2722_0a95),
        )
    })
}

fn same_key(input: &dyn RowCells, left: usize, right: usize, keys: &[ColumnIndex]) -> bool {
    keys.iter()
        .all(|column| input.cell(left, *column) == input.cell(right, *column))
}

/// Match `sort_rows` exactly: a clustered stream keeps its original order;
/// returning key runs sort by hash, then typed key values, then physical row ID.
pub(crate) fn ordered_positions(
    input: &dyn RowCells,
    keys: &[ColumnIndex],
) -> Result<Vec<usize>, DerivedFault> {
    input.validate()?;
    check_columns(input.kinds(), keys)?;
    let mut ordered = Vec::with_capacity(input.count());
    let mut runs = hashbrown::HashSet::new();
    let mut head = None;
    let mut clustered = true;
    for at in 0..input.count() {
        let row = at;
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
            let row = at;
            Ok((hash_at(input, row, keys)?, row))
        })
        .collect::<Result<Vec<_>, DerivedFault>>()?;
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
