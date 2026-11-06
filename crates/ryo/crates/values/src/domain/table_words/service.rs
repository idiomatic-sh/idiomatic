//! The service of the `table_words` concept: free functions named by verb that write tables as
//! words, so they can wait in a bucket of the store, and read them back, with no IO and no ports.

use ryo_language::domain::words::service::{
    decode_kind, decode_places, decode_value, encode_kind, encode_places, encode_value,
};
use ryo_relations::domain::relations::types::{
    Bits, CROSS_PLACE, Integer, Name, NamePlace, PlaceNumber, Value, Words,
};

use super::types::{
    Admitted, CellCount, ColumnCount, CompactCursor, CompactFrame, CompactStorage, CompactTable,
    Completeness, DecodeMode, DecodePhase, DecodedTables, EncodedSize, Framing, PackedOffset,
    PackedSlice, StorageAddress, StorageLength, StoragePart, TableShape, WordSlice, WordSpelling,
};
use crate::domain::plan_parts::types::ColumnIndex;
use crate::domain::records::types::OwnedBytes;
use crate::domain::rows::types::RowIndex;

use crate::domain::stored_facts::types::PackedWords;

/// Writes a table as words, so it can wait in a bucket of the store: its widths, its row count,
/// its kinds, and its cells.
pub fn encode_table(table: &Table, places: &[NamePlace], out: &mut Words) {
    out.0.extend([
        u64::try_from(table.kinds.len()).unwrap_or(0),
        u64::try_from(table.count.0).unwrap_or(0),
    ]);
    for kind in &table.kinds {
        encode_kind(*kind, out);
    }
    encode_places(places, out);
    for value in &table.cells {
        encode_value(*value, out);
    }
}

/// Reads every table packed in `packed` as one: the rows of all of them one after another, the kinds of the first that has any,
/// and the places of their names added to `places`, the names of each table numbered after those of the tables before it. A table
/// that cannot be read to its end is left out, and so are those after it.
#[must_use]
pub fn decode_combined(packed: &PackedWords, places: &mut Vec<NamePlace>) -> Table {
    decode_combined_status(packed, places, DecodeMode::Permissive).table
}
/// The same decoder with an explicit completion proof for required named publications.
/// Refusal rolls back this call's name-place additions; legacy permissive callers are unchanged.
pub fn decode_complete(
    packed: &PackedWords,
    places: &mut Vec<NamePlace>,
) -> Result<Table, super::errors::TableWordsError> {
    let before = places.len();
    let decoded = decode_combined_status(packed, places, DecodeMode::Complete);
    if decoded.completeness == Completeness::Complete {
        Ok(decoded.table)
    } else {
        places.truncate(before);
        Err(super::errors::TableWordsError::Incomplete)
    }
}
fn decode_combined_status(
    packed: &PackedWords,
    places: &mut Vec<NamePlace>,
    mode: DecodeMode,
) -> DecodedTables {
    let mut complete =
        mode == DecodeMode::Permissive || canonical_words(packed) == WordSpelling::Canonical;
    let mut combined = Table::default();
    // A value takes two words at least, and a packed word a byte at least.
    observed_decode(DecodePhase::BeforeReserve, packed, &combined, places);
    combined.cells.reserve(packed.0.len() / 2);
    observed_decode(DecodePhase::AfterReserve, packed, &combined, places);
    let mut words = unpacked(packed);
    while let Some(Bits(columns)) = words.next() {
        let Some(Bits(rows)) = words.next() else {
            complete = false;
            break;
        };
        let (cells, count, kept) = (combined.cells.len(), combined.count.0, places.len());
        let had_kinds = !combined.kinds.is_empty();
        let mut readable = true;
        for column in 0..columns {
            match decode_kind(&mut words) {
                Some(kind) if !had_kinds => combined.kinds.push(kind),
                Some(kind) => {
                    if mode == DecodeMode::Complete
                        && combined.kinds.get(column as usize) != Some(&kind)
                    {
                        complete = false;
                    }
                }
                None => {
                    readable = false;
                    break;
                }
            }
        }
        if mode == DecodeMode::Complete && combined.kinds.len() != columns as usize {
            complete = false;
        }
        if readable && decode_places(&mut words, places).is_none() {
            readable = false;
        }
        let base = PlaceNumber(u32::try_from(kept).unwrap_or(u32::MAX));
        'rows: for _ in 0..rows {
            if !readable {
                break;
            }
            for _ in 0..columns {
                let Some(value) = decode_value(&mut words) else {
                    readable = false;
                    break 'rows;
                };
                combined.cells.push(rebased(value, base));
            }
            combined.count = RowCount(combined.count.0 + 1);
        }
        if !readable {
            complete = false;
            combined.cells.truncate(cells);
            combined.count = RowCount(count);
            places.truncate(kept);
            if !had_kinds {
                combined.kinds.clear();
            }
            break;
        }
    }
    observed_decode(DecodePhase::BeforeShrink, packed, &combined, places);
    combined.cells.shrink_to_fit();
    observed_decode(DecodePhase::AfterShrink, packed, &combined, places);
    DecodedTables {
        table: combined,
        completeness: if complete {
            Completeness::Complete
        } else {
            Completeness::Incomplete
        },
    }
}

/// Report only this decoder's caller-owned buffers, never process/allocator residency.
fn observed_decode(at: DecodePhase, packed: &PackedWords, table: &Table, places: &Vec<NamePlace>) {
    if tracing::enabled!(target: super::constants::LIFECYCLE_TARGET, tracing::Level::TRACE) {
        let phase = match at {
            DecodePhase::BeforeReserve => super::constants::BEFORE_RESERVE,
            DecodePhase::AfterReserve => super::constants::AFTER_RESERVE,
            DecodePhase::BeforeShrink => super::constants::BEFORE_SHRINK,
            DecodePhase::AfterShrink => super::constants::AFTER_SHRINK,
        };
        let bytes = crate::domain::stores::types::retention::value::RetainedBytes::table(table);
        let place_width = size_of::<NamePlace>();
        tracing::trace!(target: super::constants::LIFECYCLE_TARGET, phase, owner = super::constants::DECODING_OWNER,
            packed_live_bytes = packed.0.len(), packed_capacity_bytes = packed.0.capacity(),
            decoded_live_bytes = bytes.cells_live, decoded_capacity_bytes = bytes.cells_capacity,
            decoded_rows = bytes.rows, metadata_capacity_bytes = bytes.metadata,
            places_live_bytes = places.len() * place_width, places_capacity_bytes = places.capacity() * place_width);
    }
}

/// Frame-local names move by `base`; global cross-file symbol indices retain their owner.
fn rebased(value: Value, base: PlaceNumber) -> Value {
    match value {
        Value::Name(name) if base.0 != 0 && name.place.0 & CROSS_PLACE == 0 => Value::Name(Name {
            hash: name.hash,
            place: PlaceNumber(name.place.0 + base.0),
        }),
        other @ (Value::Node(_)
        | Value::Symbol(_)
        | Value::Name(_)
        | Value::File(_)
        | Value::Span(_)
        | Value::Number(Integer(_))
        | Value::Flag(_)) => other,
    }
}

/// The words packed seven bits to a byte, the high bit saying another byte follows.
#[must_use]
pub fn pack(words: WordSlice<'_>) -> PackedWords {
    // Capacity overflow uses the same standard Vec refusal as an unrepresentable allocation;
    // never wrap a size or silently truncate encoded data.
    let exact = encoded_size(words).map_or(usize::MAX, |size| size.0);
    let mut packed = PackedWords(Vec::with_capacity(exact));
    pack_onto(words, &mut packed);
    packed
}

/// Exact encoded payload size, checked before any allocation.
#[must_use]
pub fn encoded_size(words: WordSlice<'_>) -> Option<EncodedSize> {
    words.0.iter().try_fold(EncodedSize(0), |total, word| {
        with_encoded_word(total, Bits(*word))
    })
}

/// Add one varint's exact width without permitting the total to wrap.
#[must_use]
pub fn with_encoded_word(total: EncodedSize, Bits(mut word): Bits) -> Option<EncodedSize> {
    let mut width = 1_usize;
    while word >= 0x80 {
        width += 1;
        word >>= 7;
    }
    total.0.checked_add(width).map(EncodedSize)
}

/// The words packed as `pack` packs them, written after what `packed` holds: packed words run together are the words of each.
pub fn pack_onto(words: WordSlice<'_>, packed: &mut PackedWords) {
    let packed = &mut packed.0;
    for word in words.0 {
        let mut rest = *word;
        while rest >= 0x80 {
            packed.push(u8::try_from(rest & 0x7f).unwrap_or_default() | 0x80);
            rest >>= 7_u32;
        }
        packed.push(u8::try_from(rest).unwrap_or_default());
    }
}

/// The words `packed` holds, read one at a time; nothing for a last word cut short.
pub fn unpacked(packed: &PackedWords) -> impl Iterator<Item = Bits> + '_ {
    CompactCursor {
        bytes: PackedSlice(&packed.0),
        offset: PackedOffset(0),
    }
}

use crate::domain::rows::types::{RowCount, Table};
/// Existing pack() emits minimal u64 varints. A required frame refuses a cut word,
/// overflow or alternate spelling rather than letting unpacked() silently shorten it.
fn canonical_words(packed: &PackedWords) -> WordSpelling {
    canonical_bytes(PackedSlice(&packed.0))
}

fn canonical_bytes(bytes: PackedSlice<'_>) -> WordSpelling {
    let mut shift = 0_u32;
    for &byte in bytes.0 {
        let payload = byte & 0x7f;
        if shift > 63 || (shift == 63 && payload > 1) {
            return WordSpelling::Altered;
        }
        if byte & 0x80 == 0 {
            if shift != 0 && payload == 0 {
                return WordSpelling::Altered;
            }
            shift = 0;
        } else {
            if shift >= 63 {
                return WordSpelling::Altered;
            }
            shift += 7;
        }
    }
    if shift == 0 {
        WordSpelling::Canonical
    } else {
        WordSpelling::Altered
    }
}

/// A worker's source range must contain one whole table frame. This walks the
/// same value codec without retaining decoded cells or row offsets, so moving
/// a byte across adjacent source ranges cannot reassign another file's rows.
pub fn single_table_frame(bytes: PackedSlice<'_>) -> Framing {
    if canonical_bytes(bytes) != WordSpelling::Canonical {
        return Framing::Broken;
    }
    let mut cursor = CompactCursor {
        bytes,
        offset: PackedOffset(0),
    };
    let (Some(Bits(columns)), Some(Bits(rows))) = (cursor.next(), cursor.next()) else {
        return Framing::Broken;
    };
    for _ in 0..columns {
        if decode_kind(&mut cursor).is_none() {
            return Framing::Broken;
        }
    }
    let mut places = Vec::new();
    if decode_places(&mut cursor, &mut places).is_none() {
        return Framing::Broken;
    }
    let Some(cells) = columns.checked_mul(rows) else {
        return Framing::Broken;
    };
    for _ in 0..cells {
        if decode_value(&mut cursor).is_none() {
            return Framing::Broken;
        }
    }
    if cursor.offset.0 == bytes.0.len() && cursor.next().is_none() {
        Framing::Whole
    } else {
        Framing::Broken
    }
}

/// Admit frame boundaries, kinds and names without retaining decoded row cells.
/// The same byte/value walk as the existing decoder preserves permissive frame
/// rollback and Complete's all-or-error name rollback. WordSpelling is checked by
/// the caller in its existing preparation order, before query execution.
pub fn admit_compact(
    backing: std::sync::Arc<crate::domain::stored_facts::types::KeptWords>,
    places: &mut Vec<NamePlace>,
    mode: DecodeMode,
) -> Result<Admitted, super::errors::TableWordsError> {
    let before = places.len();
    let mut complete =
        mode == DecodeMode::Permissive || canonical_words(&backing.0) == WordSpelling::Canonical;
    let mut cursor = CompactCursor {
        bytes: PackedSlice(&backing.0.0),
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
                    row_offsets.push(cursor.offset);
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
        });
    }
    if mode == DecodeMode::Complete && !complete {
        places.truncate(before);
        return Err(super::errors::TableWordsError::Incomplete);
    }
    tracing::trace!(target: super::constants::LIFECYCLE_TARGET, phase=super::constants::COMPACT_ADMISSION, packed_bytes=backing.0.0.len(), decoded_rows=count, decoded_cells=total_cells, frame_count=frames.len(), spelling_names=names.len(), spelling_capacity_bytes=names.capacity()*size_of::<Name>(), frame_metadata_bytes=frames.capacity()*size_of::<CompactFrame>());
    Ok(Admitted {
        table: CompactTable {
            words: backing,
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
pub fn compact_cells(input: &CompactTable) -> CellCount {
    input.cells
}

/// Only an actually empty admitted source may receive its declared schema.
pub fn shape_compact(
    input: &mut CompactTable,
    kinds: &[ryo_relations::domain::relations::types::CellKind],
) -> Result<(), super::errors::TableWordsError> {
    if input.kinds.is_empty() {
        if input.cells.0 != 0 {
            return Err(super::errors::TableWordsError::Incomplete);
        }
        input.kinds.extend_from_slice(kinds);
    }
    Ok(())
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
    let offset = *input.row_offsets.get(row.0)?;
    let mut frame_at = input
        .frames
        .partition_point(|frame| frame.start <= offset)
        .checked_sub(1)?;
    let mut cursor = CompactCursor {
        bytes: PackedSlice(&input.words.0.0),
        offset,
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
pub fn compact_backing(
    input: &CompactTable,
) -> &std::sync::Arc<crate::domain::stored_facts::types::KeptWords> {
    &input.words
}
pub fn compact_metadata_bytes(input: &CompactTable) -> OwnedBytes {
    OwnedBytes(
        size_of::<CompactTable>()
            + 2 * size_of::<usize>()
            + input.frames.capacity() * size_of::<CompactFrame>()
            + input.kinds.capacity()
                * size_of::<ryo_relations::domain::relations::types::CellKind>()
            + input.row_offsets.capacity() * size_of::<PackedOffset>(),
    )
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
