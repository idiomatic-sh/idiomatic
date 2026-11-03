//! Pure stable projection hashing and first-representative selection; no cache or scheduler.
use crate::domain::table_model::service::{check_columns, rectangular};
use ryo_derived::domain::contract::key::{mix, word_of};
use ryo_derived::domain::contract::types::KeyCellsHash;
use ryo_relations::domain::relations::{
    service::from_bits,
    types::{Bits, ColumnsView, Value},
};
use ryo_values::domain::plan_parts::types::ColumnIndex;
/// Decode one projected cell without treating missing storage as a value.
fn projected_cell(source: ColumnsView<'_>, row: u32, column: ColumnIndex) -> Option<Value> {
    let place = usize::try_from(column.0).ok()?;
    Some(from_bits(
        *source.kinds.get(place)?,
        Bits(*source.columns.get(place)?.0.get(row as usize)?),
    ))
}

/// Hash only projected typed values, preserving repeated columns and their order.
pub(crate) fn projection_hash(
    source: ColumnsView<'_>,
    columns: &[ColumnIndex],
    row: u32,
) -> Option<u64> {
    columns
        .iter()
        .try_fold(KeyCellsHash(0), |hash, column| {
            Some(mix(hash, word_of(&projected_cell(source, row, *column)?)))
        })
        .map(|hash| hash.0)
}

/// Keep the first physical representative of each tuple; the equality index dies with this call.
/// Cached hashes allow growth without decoding previously admitted rows again.
pub(crate) fn representatives(
    source: ColumnsView<'_>,
    columns: &[ColumnIndex],
    rows: &[u32],
    hash_of: impl Fn(ColumnsView<'_>, &[ColumnIndex], u32) -> Option<u64>,
) -> Option<Vec<u32>> {
    if !rectangular(source) || check_columns(source.kinds, columns).is_err() {
        return None;
    }
    let mut seen = hashbrown::HashTable::<(u32, u64)>::new();
    let mut kept = Vec::new();
    for &row in rows {
        if source.columns.first()?.0.get(row as usize).is_none() {
            return None;
        }
        let hash = hash_of(source, columns, row)?;
        if seen
            .find(hash, |(previous, _)| {
                columns.iter().all(|column| {
                    projected_cell(source, *previous, *column)
                        .zip(projected_cell(source, row, *column))
                        .is_some_and(|(left, right)| left == right)
                })
            })
            .is_some()
        {
            continue;
        }
        let _ = seen.insert_unique(hash, (row, hash), |(_, hash)| *hash);
        kept.push(row);
    }
    Some(kept)
}
