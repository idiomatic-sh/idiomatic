//! The service of the `compact_table` concept: free functions named by verb that admit a table of packed words
//! as a compact backing, with no IO and no ports, and read its rows without decoding the whole population.

use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::rows::types::RowCount;
use crate::domain::rows::types::RowIndex;
use crate::domain::table_words::service::{canonical_words, rebased};
use crate::domain::table_words::types::{
    Admitted, CellCount, ColumnCount, CompactBacking, CompactCursor, CompactFrame, CompactStorage,
    CompactTable, DecodeMode, FrameOffset, PackedOffset, PackedSlice, StorageAddress,
    StorageLength, StoragePart, TableShape, WordSpelling,
};
use ryo_language::domain::words::service::{decode_kind, decode_places, decode_value};
use ryo_relations::domain::relations::types::{Bits, Name, NamePlace, PlaceNumber, Value};

/// Admit frame boundaries, kinds and names without retaining decoded row cells.
/// The same byte/value walk as the existing decoder preserves permissive frame
/// rollback and Complete's all-or-error name rollback. WordSpelling is checked by
/// the caller in its existing preparation order, before query execution.
pub fn admit_compact(
    backing: CompactBacking,
    places: &mut Vec<NamePlace>,
    mode: DecodeMode,
) -> Result<Admitted, crate::domain::table_words::errors::TableWordsError> {
    let before = places.len();
    let mut complete =
        mode == DecodeMode::Permissive || canonical_words(&backing.0.0) == WordSpelling::Canonical;
    let mut cursor = CompactCursor {
        bytes: PackedSlice(&backing.0.0.0),
        offset: PackedOffset(0),
    };
    let mut kinds = Vec::new();
    let mut frames = Vec::new();
    let mut names = Vec::new();
    let mut count = 0_usize;
    let mut total_cells = 0_usize;
    let mut row_offsets = Vec::new();
    while let Some(Bits(columns)) = cursor.next() {
        let Some(Bits(rows)) = cursor.next() else {
            complete = false;
            break;
        };
        let kept = places.len();
        let names_before = names.len();
        let offsets_before = row_offsets.len();
        let had_kinds = !kinds.is_empty();
        let mut readable = true;
        for column in 0..columns {
            match decode_kind(&mut cursor) {
                Some(kind) if !had_kinds => kinds.push(kind),
                Some(kind) => {
                    if mode == DecodeMode::Complete && kinds.get(column as usize) != Some(&kind) {
                        complete = false;
                    }
                }
                None => {
                    readable = false;
                    break;
                }
            }
        }
        if mode == DecodeMode::Complete && kinds.len() != columns as usize {
            complete = false;
        }
        if readable && decode_places(&mut cursor, places).is_none() {
            readable = false;
        }
        let base = PlaceNumber(u32::try_from(kept).unwrap_or(u32::MAX));
        let start = cursor.offset;
        let cells = match (usize::try_from(rows), usize::try_from(columns)) {
            (Ok(rows), Ok(columns)) => rows.checked_mul(columns),
            _ => None,
        };
        if cells.is_none() {
            readable = false;
        }
        if readable {
            for cell in 0..cells.unwrap_or(0) {
                if !kinds.is_empty() && (total_cells + cell) % kinds.len() == 0 {
                    // A row is found again from its frame's start: a frame past four GiB is not readable.
                    let Ok(within) = u32::try_from(cursor.offset.0 - start.0) else {
                        readable = false;
                        break;
                    };
                    row_offsets.push(FrameOffset(within));
                }
                let Some(value) = decode_value(&mut cursor) else {
                    readable = false;
                    break;
                };
                if let Value::Name(name) = rebased(value, base) {
                    // Existing spelling validation ignores source-backed names.
                    // Do not allocate an occurrence list for those large token populations.
                    let at = usize::try_from(name.place.0).unwrap_or(usize::MAX);
                    let relevant =
                        name.place.0 & ryo_relations::domain::relations::types::CROSS_PLACE != 0
                            || places.get(at).is_some_and(|place| {
                                matches!(
                                    place.origin,
                                    ryo_relations::domain::relations::types::Origin::Derived(_)
                                )
                            });
                    let key = (name.hash, name.place);
                    let previous = names
                        .last()
                        .map(|previous: &Name| (previous.hash, previous.place));
                    if relevant && previous != Some(key) {
                        names.push(name);
                    }
                }
            }
        }
        let next_rows = count.checked_add(usize::try_from(rows).unwrap_or(usize::MAX));
        let next_cells = total_cells.checked_add(cells.unwrap_or(0));
        if !readable || next_rows.is_none() || next_cells.is_none() {
            complete = false;
            places.truncate(kept);
            names.truncate(names_before);
            row_offsets.truncate(offsets_before);
            if !had_kinds {
                kinds.clear();
            }
            break;
        }
        count = next_rows.unwrap_or(count);
        total_cells = next_cells.unwrap_or(total_cells);
        frames.push(CompactFrame {
            start,
            end: cursor.offset,
            cells: CellCount(cells.unwrap_or(0)),
            base,
            first_row: RowIndex(offsets_before),
        });
    }
    if mode == DecodeMode::Complete && !complete {
        places.truncate(before);
        return Err(crate::domain::table_words::errors::TableWordsError::Incomplete);
    }
    tracing::trace!(target: crate::domain::table_words::constants::LIFECYCLE_TARGET, phase=crate::domain::table_words::constants::COMPACT_ADMISSION, packed_bytes=backing.0.0.0.len(), decoded_rows=count, decoded_cells=total_cells, frame_count=frames.len(), spelling_names=names.len(), spelling_capacity_bytes=names.capacity()*size_of::<Name>(), frame_metadata_bytes=frames.capacity()*size_of::<CompactFrame>());
    Ok(Admitted {
        table: CompactTable {
            words: backing.0,
            kinds,
            frames,
            rows: RowCount(count),
            cells: CellCount(total_cells),
            row_offsets,
        },
        names,
    })
}

pub fn compact_kinds(input: &CompactTable) -> &[ryo_relations::domain::relations::types::CellKind] {
    &input.kinds
}
pub fn compact_rows(input: &CompactTable) -> RowCount {
    input.rows
}

/// Read one admitted physical row without allocating a whole decoded population.
/// Permissive decoding historically concatenates self-tagged cells even across
/// frames with different widths. Preserve that exact flat-row interpretation.
fn compact_values(
    input: &CompactTable,
    row: RowIndex,
    count: ColumnCount,
    mut emit: impl FnMut(ColumnIndex, Value),
) -> Option<()> {
    if row.0 >= input.rows.0 || count.0 > input.kinds.len() {
        return None;
    }
    if count.0 == 0 {
        return Some(());
    }
    let Ok(within) = usize::try_from(input.row_offsets.get(row.0)?.0) else {
        return None;
    };
    // The frame the row starts in is the last that no later row starts before: frames in which no row starts come
    // before it or carry a larger first row.
    let mut frame_at = input
        .frames
        .partition_point(|frame| frame.first_row.0 <= row.0)
        .checked_sub(1)?;
    let mut cursor = CompactCursor {
        bytes: PackedSlice(&input.words.0.0),
        offset: PackedOffset(input.frames.get(frame_at)?.start.0.checked_add(within)?),
    };
    for column in 0..count.0 {
        while cursor.offset == input.frames.get(frame_at)?.end {
            frame_at += 1;
            cursor.offset = input.frames.get(frame_at)?.start;
        }
        let frame = input.frames.get(frame_at)?;
        let value = decode_value(&mut cursor)?;
        if cursor.offset > frame.end {
            return None;
        }
        let Ok(column) = u32::try_from(column) else {
            return None;
        };
        emit(ColumnIndex(column), rebased(value, frame.base));
    }
    Some(())
}
pub fn compact_read<'a>(
    input: &CompactTable,
    row: RowIndex,
    scratch: &'a mut [Value],
) -> Option<&'a [Value]> {
    if scratch.len() < input.kinds.len() {
        return None;
    }
    compact_values(
        input,
        row,
        ColumnCount(input.kinds.len()),
        |column, value| scratch[column.0 as usize] = value,
    )?;
    Some(&scratch[..input.kinds.len()])
}
pub fn compact_cell(input: &CompactTable, row: RowIndex, column: ColumnIndex) -> Option<Value> {
    if column.0 as usize >= input.kinds.len() {
        return None;
    }
    let mut found = None;
    compact_values(
        input,
        row,
        ColumnCount(column.0 as usize + 1),
        |_, value| found = Some(value),
    )?;
    found
}
/// Admission validated every encoded cell once; this is a metadata-only shape check.
pub fn compact_rectangular(input: &CompactTable) -> TableShape {
    if input.rows.0.checked_mul(input.kinds.len()) == Some(input.cells.0)
        && input
            .frames
            .iter()
            .all(|frame| frame.start <= frame.end && frame.end.0 <= input.words.0.0.len())
        && input
            .frames
            .iter()
            .try_fold(0usize, |sum, frame| sum.checked_add(frame.cells.0))
            == Some(input.cells.0)
        && (input.kinds.is_empty() || input.row_offsets.len() == input.rows.0)
    {
        TableShape::Rectangular
    } else {
        TableShape::Ragged
    }
}
pub fn compact_backing(input: &CompactTable) -> CompactBacking {
    CompactBacking(std::sync::Arc::clone(&input.words))
}
pub fn compact_storage(input: &CompactTable) -> CompactStorage {
    CompactStorage {
        words: StoragePart {
            address: StorageAddress(input.words.0.0.as_ptr() as usize),
            length: StorageLength(input.words.0.0.len()),
        },
        offsets: StoragePart {
            address: StorageAddress(input.row_offsets.as_ptr() as usize),
            length: StorageLength(input.row_offsets.len()),
        },
    }
}
