//! The service of the `table_words` concept: free functions named by verb that write tables as
//! words, so they can wait in a bucket of the store, and read them back, with no IO and no ports.

use ryo_language::domain::words::service::{
    decode_kind, decode_places, decode_value, encode_kind, encode_places, encode_value,
};
use ryo_relations::domain::relations::types::{
    Bits, Integer, Name, NamePlace, PlaceNumber, Value, Words,
};

use super::types::{CompactFrame, CompactTable, DecodeMode, WordSlice};

use ryo_values::domain::stored_facts::types::PackedWords;

/// Writes a table as words, so it can wait in a bucket of the store: its widths, its row count,
/// its kinds, and its cells.
pub(crate) fn encode_table(table: &Table, places: &[NamePlace], out: &mut Words) {
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
pub(crate) fn decode_combined(packed: &PackedWords, places: &mut Vec<NamePlace>) -> Table {
    decode_combined_status(packed, places, DecodeMode::Permissive).0
}
/// The same decoder with an explicit completion proof for required named publications.
/// Refusal rolls back this call's name-place additions; legacy permissive callers are unchanged.
pub(crate) fn decode_complete(
    packed: &PackedWords,
    places: &mut Vec<NamePlace>,
) -> Result<Table, super::errors::WordsDecodeError> {
    let before = places.len();
    let (table, complete) = decode_combined_status(packed, places, DecodeMode::Complete);
    if complete {
        Ok(table)
    } else {
        places.truncate(before);
        Err(super::errors::WordsDecodeError::Incomplete)
    }
}
fn decode_combined_status(
    packed: &PackedWords,
    places: &mut Vec<NamePlace>,
    mode: DecodeMode,
) -> (Table, bool) {
    let mut complete = mode == DecodeMode::Permissive || canonical_words(packed);
    let mut combined = Table::default();
    // A value takes two words at least, and a packed word a byte at least.
    observed_decode("decode_before_reserve", packed, &combined, places);
    combined.cells.reserve(packed.0.len() / 2);
    observed_decode("decode_after_reserve", packed, &combined, places);
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
    observed_decode("decode_before_shrink", packed, &combined, places);
    combined.cells.shrink_to_fit();
    observed_decode("decode_after_shrink", packed, &combined, places);
    (combined, complete)
}

/// Report only this decoder's caller-owned buffers, never process/allocator residency.
fn observed_decode(
    phase: &'static str,
    packed: &PackedWords,
    table: &Table,
    places: &Vec<NamePlace>,
) {
    if tracing::enabled!(target: "ryo::diagnostic::lifecycle_memory", tracing::Level::TRACE) {
        let bytes = crate::domain::recording::types::retention::value::RetainedBytes::table(table);
        let place_width = size_of::<NamePlace>();
        tracing::trace!(target: "ryo::diagnostic::lifecycle_memory", phase, owner = "decoding_input",
            packed_live_bytes = packed.0.len(), packed_capacity_bytes = packed.0.capacity(),
            decoded_live_bytes = bytes.cells_live, decoded_capacity_bytes = bytes.cells_capacity,
            decoded_rows = bytes.rows, metadata_capacity_bytes = bytes.metadata,
            places_live_bytes = places.len() * place_width, places_capacity_bytes = places.capacity() * place_width);
    }
}

/// A value with the number of its place moved up by `base`, when it is a name of a table whose places start there.
fn rebased(value: Value, base: PlaceNumber) -> Value {
    match value {
        Value::Name(name) if base.0 != 0 => Value::Name(Name {
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
pub(crate) fn pack(words: WordSlice<'_>) -> PackedWords {
    // Capacity overflow uses the same standard Vec refusal as an unrepresentable allocation;
    // never wrap a size or silently truncate encoded data.
    let exact = encoded_size(words).unwrap_or(usize::MAX);
    let mut packed = PackedWords(Vec::with_capacity(exact));
    pack_onto(words, &mut packed);
    packed
}

/// Exact encoded payload size, checked before any allocation.
#[must_use]
pub(crate) fn encoded_size(words: WordSlice<'_>) -> Option<usize> {
    words
        .0
        .iter()
        .try_fold(0_usize, |total, word| with_encoded_word(total, *word))
}

/// Add one varint's exact width without permitting the total to wrap.
#[must_use]
pub(crate) fn with_encoded_word(total: usize, mut word: u64) -> Option<usize> {
    let mut width = 1_usize;
    while word >= 0x80 {
        width += 1;
        word >>= 7;
    }
    total.checked_add(width)
}

/// The words packed as `pack` packs them, written after what `packed` holds: packed words run together are the words of each.
pub(crate) fn pack_onto(words: WordSlice<'_>, packed: &mut PackedWords) {
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
pub(crate) fn unpacked(packed: &PackedWords) -> impl Iterator<Item = Bits> + '_ {
    CompactCursor {
        bytes: &packed.0,
        offset: 0,
    }
}

use ryo_values::domain::rows::types::{RowCount, Table};
/// Existing pack() emits minimal u64 varints. A required frame refuses a cut word,
/// overflow or alternate spelling rather than letting unpacked() silently shorten it.
fn canonical_words(packed: &PackedWords) -> bool {
    let mut shift = 0_u32;
    for &byte in &packed.0 {
        let payload = byte & 0x7f;
        if shift > 63 || (shift == 63 && payload > 1) {
            return false;
        }
        if byte & 0x80 == 0 {
            if shift != 0 && payload == 0 {
                return false;
            }
            shift = 0;
        } else {
            if shift >= 63 {
                return false;
            }
            shift += 7;
        }
    }
    shift == 0
}

/// Admit frame boundaries, kinds and names without retaining decoded row cells.
/// The same byte/value walk as the existing decoder preserves permissive frame
/// rollback and Complete's all-or-error name rollback. Spelling is checked by
/// the caller in its existing preparation order, before query execution.
pub(crate) fn admit_compact(
    backing: std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords>,
    places: &mut Vec<NamePlace>,
    mode: DecodeMode,
) -> Result<(CompactTable, Vec<Name>), super::errors::WordsDecodeError> {
    let before = places.len();
    let mut complete = mode == DecodeMode::Permissive || canonical_words(&backing.0);
    let mut cursor = CompactCursor {
        bytes: &backing.0.0,
        offset: 0,
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
        let cells = usize::try_from(rows).ok().and_then(|rows| {
            usize::try_from(columns)
                .ok()
                .and_then(|columns| rows.checked_mul(columns))
        });
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
            bytes: start..cursor.offset,
            cells: cells.unwrap_or(0),
            base,
        });
    }
    if mode == DecodeMode::Complete && !complete {
        places.truncate(before);
        return Err(super::errors::WordsDecodeError::Incomplete);
    }
    tracing::trace!(target: "ryo::diagnostic::lifecycle_memory", phase="compact_admission", packed_bytes=backing.0.0.len(), decoded_rows=count, decoded_cells=total_cells, frame_count=frames.len(), spelling_names=names.len(), spelling_capacity_bytes=names.capacity()*size_of::<Name>(), frame_metadata_bytes=frames.capacity()*size_of::<CompactFrame>());
    Ok((
        CompactTable {
            words: backing,
            kinds,
            frames,
            rows: count,
            cells: total_cells,
            row_offsets,
        },
        names,
    ))
}

/// Decode only when the shared invocation owner demands this admitted source.
/// Offset/name admission is immutable; no source-symbol mutation or remap repeats.
#[cfg(test)]
pub(crate) fn materialize_compact(
    input: &CompactTable,
) -> Result<Table, super::errors::WordsDecodeError> {
    let mut table = Table {
        kinds: input.kinds.clone(),
        count: RowCount(input.rows),
        cells: Vec::with_capacity(input.cells),
    };
    for frame in &input.frames {
        let bytes = input
            .words
            .0
            .0
            .get(frame.bytes.clone())
            .ok_or(super::errors::WordsDecodeError::Incomplete)?;
        let mut cursor = CompactCursor { bytes, offset: 0 };
        for _ in 0..frame.cells {
            let value =
                decode_value(&mut cursor).ok_or(super::errors::WordsDecodeError::Incomplete)?;
            table.cells.push(rebased(value, frame.base));
        }
        if cursor.offset != bytes.len() {
            return Err(super::errors::WordsDecodeError::Incomplete);
        }
    }
    Ok(table)
}
pub(crate) fn compact_kinds(
    input: &CompactTable,
) -> &[ryo_relations::domain::relations::types::CellKind] {
    &input.kinds
}
pub(crate) fn compact_rows(input: &CompactTable) -> usize {
    input.rows
}
pub(crate) fn compact_cells(input: &CompactTable) -> usize {
    input.cells
}

/// One byte cursor supplies exact immutable frame offsets; no second packed copy.
struct CompactCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Iterator for CompactCursor<'_> {
    type Item = Bits;
    fn next(&mut self) -> Option<Bits> {
        let (mut word, mut shift) = (0_u64, 0_u32);
        while let Some(byte) = self.bytes.get(self.offset) {
            self.offset += 1;
            word |= u64::from(byte & 0x7f)
                .checked_shl(shift)
                .unwrap_or_default();
            if byte & 0x80 == 0 {
                return Some(Bits(word));
            }
            shift += 7;
        }
        None
    }
}

/// Only an actually empty admitted source may receive its declared schema.
pub(crate) fn shape_compact(
    input: &mut CompactTable,
    kinds: &[ryo_relations::domain::relations::types::CellKind],
) -> Result<(), super::errors::WordsDecodeError> {
    if input.kinds.is_empty() {
        if input.cells != 0 {
            return Err(super::errors::WordsDecodeError::Incomplete);
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
    row: usize,
    count: usize,
    mut emit: impl FnMut(usize, Value),
) -> Option<()> {
    if row >= input.rows || count > input.kinds.len() {
        return None;
    }
    if count == 0 {
        return Some(());
    }
    let offset = *input.row_offsets.get(row)?;
    let mut frame_at = input
        .frames
        .partition_point(|frame| frame.bytes.start <= offset)
        .checked_sub(1)?;
    let mut cursor = CompactCursor {
        bytes: &input.words.0.0,
        offset,
    };
    for column in 0..count {
        while cursor.offset == input.frames.get(frame_at)?.bytes.end {
            frame_at += 1;
            cursor.offset = input.frames.get(frame_at)?.bytes.start;
        }
        let frame = input.frames.get(frame_at)?;
        let value = decode_value(&mut cursor)?;
        if cursor.offset > frame.bytes.end {
            return None;
        }
        emit(column, rebased(value, frame.base));
    }
    Some(())
}
pub(crate) fn compact_read<'a>(
    input: &CompactTable,
    row: usize,
    scratch: &'a mut [Value],
) -> Option<&'a [Value]> {
    if scratch.len() < input.kinds.len() {
        return None;
    }
    compact_values(input, row, input.kinds.len(), |column, value| {
        scratch[column] = value
    })?;
    Some(&scratch[..input.kinds.len()])
}
pub(crate) fn compact_cell(input: &CompactTable, row: usize, column: usize) -> Option<Value> {
    if column >= input.kinds.len() {
        return None;
    }
    let mut found = None;
    compact_values(input, row, column + 1, |_, value| found = Some(value))?;
    found
}
/// Admission validated every encoded cell once; this is a metadata-only shape check.
pub(crate) fn compact_rectangular(input: &CompactTable) -> bool {
    input.rows.checked_mul(input.kinds.len()) == Some(input.cells)
        && input.frames.iter().all(|frame| {
            frame.bytes.start <= frame.bytes.end && frame.bytes.end <= input.words.0.0.len()
        })
        && input
            .frames
            .iter()
            .try_fold(0usize, |sum, frame| sum.checked_add(frame.cells))
            == Some(input.cells)
        && (input.kinds.is_empty() || input.row_offsets.len() == input.rows)
}
pub(crate) fn compact_backing(
    input: &CompactTable,
) -> &std::sync::Arc<ryo_values::domain::stored_facts::types::KeptWords> {
    &input.words
}
pub(crate) fn compact_metadata_bytes(input: &CompactTable) -> usize {
    size_of::<CompactTable>()
        + 2 * size_of::<usize>()
        + input.frames.capacity() * size_of::<CompactFrame>()
        + input.kinds.capacity() * size_of::<ryo_relations::domain::relations::types::CellKind>()
        + input.row_offsets.capacity() * size_of::<usize>()
}

pub(crate) fn compact_storage(input: &CompactTable) -> [(usize, usize); 2] {
    [
        (input.words.0.0.as_ptr() as usize, input.words.0.0.len()),
        (input.row_offsets.as_ptr() as usize, input.row_offsets.len()),
    ]
}
