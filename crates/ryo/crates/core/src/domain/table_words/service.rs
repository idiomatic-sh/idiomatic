//! The service of the `table_words` concept: free functions named by verb that write tables as
//! words, so they can wait in a bucket of the store, and read them back, with no IO and no ports.

use ryo_language::domain::relations::types::{
    Bits, Integer, Name, NamePlace, PlaceNumber, Value, Words,
};
use ryo_language::domain::words::service::{
    decode_kind, decode_places, decode_value, encode_kind, encode_places, encode_value,
};

use super::types::WordSlice;
use crate::domain::table_model::types::{RowCount, Table};
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
    let mut combined = Table::default();
    // A value takes two words at least, and a packed word a byte at least.
    combined.cells.reserve(packed.0.len() / 2);
    let mut words = unpacked(packed);
    while let Some(Bits(columns)) = words.next() {
        let Some(Bits(rows)) = words.next() else {
            break;
        };
        let (cells, count, kept) = (combined.cells.len(), combined.count.0, places.len());
        let had_kinds = !combined.kinds.is_empty();
        let mut readable = true;
        for _ in 0..columns {
            match decode_kind(&mut words) {
                Some(kind) if !had_kinds => combined.kinds.push(kind),
                Some(_) => {}
                None => readable = false,
            }
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
            combined.cells.truncate(cells);
            combined.count = RowCount(count);
            places.truncate(kept);
            if !had_kinds {
                combined.kinds.clear();
            }
            break;
        }
    }
    combined.cells.shrink_to_fit();
    combined
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
    let mut packed = PackedWords(Vec::with_capacity(words.0.len() * 2));
    pack_onto(words, &mut packed);
    packed
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
