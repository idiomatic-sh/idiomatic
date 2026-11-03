//! Typed key identity shared by construction and immutable physical probes.
use super::types::{HashWord, KeyCellsHash, On};
use ryo_relations::domain::relations::types::{Flag, Integer, Value};
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
#[must_use]
pub const fn mix(hash: KeyCellsHash, word: HashWord) -> KeyCellsHash {
    KeyCellsHash((hash.0.rotate_left(5) ^ word.0).wrapping_mul(0x517c_c1b7_2722_0a95))
}
#[must_use]
pub fn hash_key<'a>(cells: impl Iterator<Item = Option<&'a Value>>) -> KeyCellsHash {
    cells.fold(KeyCellsHash(0), |hash, cell| {
        mix(hash, cell.map_or(HashWord(0), word_of))
    })
}
#[must_use]
pub fn hash_row(row: &[Value], on: On<'_>) -> KeyCellsHash {
    match on {
        On::Whole => hash_key(row.iter().map(Some)),
        On::Columns(columns) => hash_key(
            columns
                .iter()
                .map(|column| usize::try_from(column.0).ok().and_then(|at| row.get(at))),
        ),
    }
}
