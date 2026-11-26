//! Rows keyed by the hash of their key: one `hash_index` table, its keys' runs and each row's next row of the same key; and
//! the one hash and the one comparison of a key's cells.
use crate::domain::contract::types::PredicateOutcome;
use ryo_relations::domain::relations::types::{CellKind, Flag, Value};
use std::hash::{Hash as _, Hasher as _};




/// The hash of a key: of its text for a name column, of its cells otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexKeyHash(pub(crate) u64);

/// A row of an indexed relation, by its place in the relation's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexedRow(pub(crate) u32);


/// The hash of a key's cells, in order: the one hash every index and row set keys its rows by.
pub(crate) fn hash_of_cells(cells: impl IntoIterator<Item = Value>) -> IndexKeyHash {
    // Spike (ryo-jenl): one multiply per cell, of the cell's payload word, the variant never hashed: equal cells have equal
    // words (a name's word is its text's hash, which is its equality), so it is a hash of the cells' equality.
    let mut hash = 0u64;
    for cell in cells {
        hash = (hash.rotate_left(5) ^ cell_word(cell)).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    IndexKeyHash(hash)

}





/// Spike (ryo-jenl): how a key of declared kinds is one machine word, so equal words are equal keys and the word's mix is its
/// exact hash: one cell of any kind (its payload), or two cells of 32-bit payloads (a node's place in its file, with every
/// key's node in one file per column; a name's symbol; a file; a flag). `Cells` for any other key: hashed and compared cell
/// by cell. Decided from the key columns' declared kinds (`CellKind`), never from a relation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum KeyWords {
    /// Hashed and compared cell by cell.
    #[default]
    Cells,
    /// One cell of this kind.
    One(CellKind),
    /// Two cells of these kinds; a node column's file is fixed by the first key.
    Two([CellKind; 2], [u32; 2]),
}

/// Spike (ryo-jenl): the words a key of `kinds` is, before its first row fixes a node column's file.
pub(crate) fn key_words(kinds: impl ExactSizeIterator<Item = CellKind>) -> KeyWords {
    let mut kinds = kinds;
    match kinds.len() {
        1 => kinds.next().map_or(KeyWords::Cells, KeyWords::One),
        2 => match (kinds.next(), kinds.next()) {
            (Some(one), Some(two)) if halved(one) && halved(two) => KeyWords::Two([one, two], [u32::MAX; 2]),
            _ => KeyWords::Cells,
        },
        _ => KeyWords::Cells,
    }
}

/// Whether a cell of `kind` has a 32-bit payload within its file.
fn halved(kind: CellKind) -> bool {
    matches!(kind, CellKind::Node | CellKind::Symbol | CellKind::File | CellKind::Flag)
}

/// Spike (ryo-jenl): the payload of a cell of `kind`; none for a cell of another kind.
#[inline]
pub(crate) fn word_of(kind: CellKind, cell: Value) -> Option<u64> {
    match (kind, cell) {
        (CellKind::Node, Value::Node(node)) => Some(node.0),
        (CellKind::Symbol, Value::Symbol(symbol)) => Some(u64::from(symbol.0)),
        (CellKind::File, Value::File(file)) => Some(u64::from(file.0)),
        (CellKind::Span, Value::Span(span)) => Some((u64::from(span.start.0) << 32) | u64::from(span.end.0)),
        (CellKind::Number, Value::Number(number)) => Some(number.0.cast_unsigned()),
        (CellKind::Flag, Value::Flag(flag)) => Some(u64::from(flag == Flag::Raised)),
        _ => None,
    }
}

/// Spike (ryo-jenl): the 32-bit payload of a cell of `kind` in a key whose node column is in file `file`
/// (`u32::MAX`: not fixed yet, fixed here); none for another kind or another file.
#[inline]
fn half_of(kind: CellKind, cell: Value, file: &mut u32) -> Option<u64> {
    if let (CellKind::Node, Value::Node(node)) = (kind, cell) {
        let here = u32::try_from(node.0 >> 32).unwrap_or(u32::MAX);
        if *file == u32::MAX {
            *file = here;
        }
        return (here == *file).then_some(node.0 & 0xffff_ffff);
    }
    word_of(kind, cell)
}

/// Spike (ryo-jenl): the exact hash of a key's word: a bijection of the word, so equal hashes are equal keys.
#[inline]
pub(crate) fn word_hash(word: u64) -> IndexKeyHash {
    let mixed = word.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    IndexKeyHash(mixed ^ (mixed >> 32))
}

impl KeyWords {
    /// Spike (ryo-jenl): the word of a key of cells `one` and `two` (none for a one-cell key), fixing a node column's file on
    /// its first key when `fix`; none for a cell of another kind or file.
    #[inline]
    pub(crate) fn word(&mut self, one: Value, two: Option<Value>, fix: bool) -> Option<u64> {
        match self {
            Self::Cells => None,
            Self::One(kind) => word_of(*kind, one),
            Self::Two(kinds, files) => {
                let two = two?;
                let mut held = *files;
                let high = half_of(kinds[0], one, &mut held[0])?;
                let low = half_of(kinds[1], two, &mut held[1])?;
                if fix {
                    *files = held;
                } else if held != *files {
                    return None;
                }
                Some((high << 32) | low)
            }
        }
    }
}

/// Spike (ryo-jenl): a cell's payload as one word, whatever its variant: equal cells have equal words.
#[inline]
pub(crate) fn cell_word(cell: Value) -> u64 {
    match cell {
        Value::Node(node) => node.0,
        Value::Symbol(symbol) => u64::from(symbol.0),
        Value::Name(name) => {
            let hash = name.hash;
            hash.0
        }
        Value::File(file) => u64::from(file.0),
        Value::Span(span) => (u64::from(span.start.0) << 32) | u64::from(span.end.0),
        Value::Number(number) => number.0.cast_unsigned(),
        Value::Flag(flag) => u64::from(flag == Flag::Raised),
    }
}
