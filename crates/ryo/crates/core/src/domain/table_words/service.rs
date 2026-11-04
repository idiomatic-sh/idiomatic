//! The service of the `table_words` concept: free functions named by verb that write tables as
//! words, so they can wait in a bucket of the store, and read them back, with no IO and no ports.

use ryo_language::domain::words::service::{
    decode_kind, decode_places, decode_value, encode_kind, encode_places, encode_value,
};
use ryo_relations::domain::relations::types::{
    Bits, Integer, Name, NamePlace, PlaceNumber, Value, Words,
};

use super::types::WordSlice;

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
#[derive(Clone, Copy, PartialEq, Eq)]
enum DecodeMode {
    Permissive,
    Complete,
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
    let mut bytes = packed.0.iter();
    std::iter::from_fn(move || {
        let (mut word, mut shift) = (0_u64, 0_u32);
        for byte in bytes.by_ref() {
            word |= u64::from(byte & 0x7f)
                .checked_shl(shift)
                .unwrap_or_default();
            if byte & 0x80 == 0 {
                return Some(Bits(word));
            }
            shift += 7;
        }
        None
    })
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
