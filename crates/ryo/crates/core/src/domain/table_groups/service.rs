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

/// Lists the rows of `table` into `order.sorted` in the order of the hash of their key, those with
/// one hash by key, and rows with the same key in the order they came: the rows of a key are
/// together, for the operators that group.
///
/// # Errors
///
/// `NoSuchColumn` when `on` names a column the table lacks.
pub(crate) fn sort_rows(
    table: &Table,
    on: On<'_>,
    order: &mut KeyIndex,
) -> Result<(), TableModelError> {
    if let On::Columns(columns) = on {
        check_columns(&table.kinds, columns)?;
    }
    let KeyIndex {
        sorted,
        pairs,
        runs,
        ..
    } = order;
    // Rows with one key that come one after the other, as they do when a step made them key by key, are grouped as they are:
    // a key is hashed where its run begins and no more. A key that comes back after another is for the sort.
    runs.clear();
    sorted.clear();
    let mut head: Option<&[Value]> = None;
    let mut clustered = true;
    for each in positions(table.count) {
        let row = row_in(table, each).unwrap_or_default();
        let same = head.is_some_and(|first| match on {
            On::Whole => first == row,
            On::Columns(columns) => columns
                .iter()
                .all(|column| column_in(first, *column) == column_in(row, *column)),
        });
        if !same {
            if !runs.insert(hash_row(row, on)) {
                clustered = false;
                break;
            }
            head = Some(row);
        }
        sorted.push(each);
    }
    if clustered {
        return Ok(());
    }
    pairs.clear();
    pairs.extend(positions(table.count).map(|each| HashedRow {
        hash: row_in(table, each).map_or(KeyCellsHash(0), |row| hash_row(row, on)),
        row: each,
    }));
    pairs.sort_unstable();
    sorted.clear();
    sorted.extend(pairs.iter().map(|pair| pair.row));
    // Two keys with one hash are put in the order of their keys, which is rare enough to cost nothing.
    let mut start = 0;
    for tied in pairs.chunk_by(|left, right| left.hash == right.hash) {
        let end = start + tied.len();
        if let Some(rows) = sorted.get_mut(start..end)
            && rows.len() > 1
        {
            rows.sort_by(|left, right| {
                let (Some(left_row), Some(right_row)) =
                    (row_in(table, *left), row_in(table, *right))
                else {
                    return left.cmp(right);
                };
                let ordering = match on {
                    On::Whole => left_row.cmp(right_row),
                    On::Columns(columns) => columns
                        .iter()
                        .map(|column| {
                            column_in(left_row, *column).cmp(&column_in(right_row, *column))
                        })
                        .find(|each| each.is_ne())
                        .unwrap_or(Ordering::Equal),
                };
                ordering.then(left.cmp(right))
            });
        }
        start = end;
    }
    Ok(())
}

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
    sort_rows(table, on, order)?;
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
    for run in order.sorted.chunk_by(same_key) {
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
    sort_rows(table, On::Columns(keys), order)?;
    let key_of_row = |each: &RowIndex| row_in(table, *each).unwrap_or_default();
    let same_key = |left: &RowIndex, right: &RowIndex| {
        keys.iter().all(|column| {
            column_in(key_of_row(left), *column) == column_in(key_of_row(right), *column)
        })
    };
    groups.clear();
    for run in order.sorted.chunk_by(same_key) {
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
