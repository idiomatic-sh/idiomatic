//! Typed key identity shared by construction and immutable physical probes.
use super::types::{HashWord, KeyCellsHash, On};
use ryo_relations::domain::relations::types::{Flag, Integer, Value};
use std::hash::Hasher as _;

/// The word of one cell.
#[must_use]
pub fn word_of(value: &Value) -> HashWord {
    HashWord(match value {
        Value::Node(value) => value.0,
        Value::Symbol(value) => u64::from(value.0),
        Value::Name(value) => value.hash.0,
        Value::File(value) => u64::from(value.0),
        Value::Span(value) => (u64::from(value.start.0) << 32) | u64::from(value.end.0),
        Value::Number(Integer(value)) => value.cast_unsigned(),
        Value::Flag(value) => u64::from(*value == Flag::Raised),
    })
}

/// Folds one word into a running key hash: one step of the standard `FxHasher` (rustc-hash) over the running hash plus the word.
#[must_use]
pub fn mix(hash: KeyCellsHash, word: HashWord) -> KeyCellsHash {
    let mut hasher = rustc_hash::FxHasher::default();
    hasher.write_u64(hash.0.wrapping_add(word.0));
    KeyCellsHash(hasher.finish())
}

/// The hash of the cells of one key; an absent cell is the zero word.
#[must_use]
pub fn hash_key<'a>(cells: impl Iterator<Item = Option<&'a Value>>) -> KeyCellsHash {
    cells.fold(KeyCellsHash(0), |hash, cell| {
        mix(hash, cell.map_or(HashWord(0), word_of))
    })
}

/// The hash of one row on its whole or on some of its columns.
#[must_use]
pub fn hash_row(row: &[Value], on: On<'_>) -> KeyCellsHash {
    match on {
        On::Whole => hash_key(row.iter().map(Some)),
        On::Columns(columns) => {
            hash_key(columns.iter().map(|column| row.get(usize::from(*column))))
        }
    }
}
