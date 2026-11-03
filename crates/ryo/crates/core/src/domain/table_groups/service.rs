//! The `table_groups` service: sorting the rows of a table by their key and counting or digesting the groups, with no IO and no ports.

use std::cmp::Ordering;
use std::hash::Hasher as _;

use ryo_relations::domain::relations::types::Value;

use super::types::{Group, Hashes, SetDigest, ValueHash};

use crate::domain::table_groups::types::CountRoom;
use crate::domain::table_model::errors::TableModelError;
use crate::domain::table_model::service::{check_columns, column_in, positions, row_in};
use ryo_derived::domain::contract::key::hash_row;

use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_derived::domain::contract::types::{HashedRow, KeyCellsHash, KeyIndex, On};
use ryo_values::domain::plan_parts::types::{ColumnIndex, FlagState};

/// Counts the rows of `table` for each key `on` names, or the distinct values of the cell at
/// `distinct` when there is one, into `groups`, in the order the groups first appeared.
///
/// # Errors
///
/// `NoSuchColumn` when `on` or `distinct` names a column the table lacks.
pub(crate) fn count_groups(
    table: &Table,
    on: On<'_>,
    distinct: Option<ColumnIndex>,
    room: &mut CountRoom<'_>,
) -> Result<(), TableModelError> {
    let CountRoom {
        order,
        groups,
        values,
    } = room;
    let wide = |column: ColumnIndex| {
        usize::try_from(column.0).map_or(true, |place| place >= table.kinds.len())
    };
    if let Some(missing) = distinct.filter(|column| wide(*column)) {
        return Err(TableModelError::NoSuchColumn { index: missing.0 });
    }
    if let On::Columns(columns) = on {
        check_columns(&table.kinds, columns)?;
    }
    let (key_of_row, value_of_row) = (
        |each: &RowIndex| row_in(table, *each).unwrap_or_default(),
        |each: &RowIndex| {
            distinct
                .and_then(|column| column_in(row_in(table, *each).unwrap_or_default(), column))
                .copied()
        },
    );
    let same_key = |left: &RowIndex, right: &RowIndex| match on {
        On::Whole => key_of_row(left) == key_of_row(right),
        On::Columns(columns) => columns.iter().all(|column| {
            column_in(key_of_row(left), *column) == column_in(key_of_row(right), *column)
        }),
    };
    groups.clear();
    for run in order.chunk_by(same_key) {
        let first = run.iter().copied().min().unwrap_or(RowIndex(0));
        let counted = if distinct.is_none() {
            run.len()
        } else {
            values.clear();
            values.extend(run.iter().filter_map(value_of_row));
            values.sort_unstable();
            values.dedup();
            values.len()
        };
        groups.push(Group {
            first,
            counted: RowCount(counted),
            digest: SetDigest(0),
        });
    }
    groups.sort_unstable_by_key(|group| group.first);
    Ok(())
}

/// What a value is for a digest: the hash of its text, or of its number, so that the same text in
/// two files, whatever the numbers of its names, gives the same hash.
pub(crate) fn hash_of(value: Value, texts: &Texts<'_>) -> ValueHash {
    let mut hasher = rustc_hash::FxHasher::default();
    match datum(value, texts) {
        Datum::Unlocated => {
            hasher.write(ryo_relations::domain::relations::constants::UNLOCATED.as_bytes());
        }
        Datum::Text(text) => hasher.write(text.0.as_bytes()),
        Datum::Number(number) => hasher.write_i64(number.0),
        Datum::Flag(flag) => hasher.write_u8(u8::from(flag == FlagState::Raised)),
        Datum::Numbers(numbers) => numbers.iter().for_each(|number| hasher.write_i64(number.0)),
        Datum::Texts(items) => items
            .iter()
            .for_each(|text| hasher.write(text.0.as_bytes())),
    }
    ValueHash(hasher.finish())
}

/// Groups the rows of `table` by `keys` and digests, for each group, the set of distinct values of
/// the column `over`: the same set gives the same digest whatever the order of the rows or the
/// repeats, and `counted` is how many values the set has. `hashes` is the room the digest is made in.
///
/// # Errors
///
/// `NoSuchColumn` when `keys` or `over` names a column the table lacks.
pub(crate) fn digest_groups(
    table: &Table,
    keys: &[ColumnIndex],
    over: ColumnIndex,
    texts: &Texts<'_>,
    room: &mut CountRoom<'_>,
    hashes: &mut Hashes,
) -> Result<(), TableModelError> {
    let hashes = &mut hashes.0;
    let CountRoom { order, groups, .. } = room;
    if usize::try_from(over.0).map_or(true, |place| place >= table.kinds.len()) {
        return Err(TableModelError::NoSuchColumn { index: over.0 });
    }
    check_columns(&table.kinds, keys)?;
    let key_of_row = |each: &RowIndex| row_in(table, *each).unwrap_or_default();
    let same_key = |left: &RowIndex, right: &RowIndex| {
        keys.iter().all(|column| {
            column_in(key_of_row(left), *column) == column_in(key_of_row(right), *column)
        })
    };
    groups.clear();
    for run in order.chunk_by(same_key) {
        let first = run.iter().copied().min().unwrap_or(RowIndex(0));
        hashes.clear();
        hashes.extend(
            run.iter()
                .filter_map(|each| column_in(key_of_row(each), over).copied())
                .map(|value| hash_of(value, texts).0),
        );
        hashes.sort_unstable();
        hashes.dedup();
        let mut folded = rustc_hash::FxHasher::default();
        for hash in hashes.iter() {
            folded.write_u64(*hash);
        }
        groups.push(Group {
            first,
            counted: RowCount(hashes.len()),
            digest: SetDigest(folded.finish().cast_signed()),
        });
    }
    groups.sort_unstable_by_key(|group| group.first);
    Ok(())
}

use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
