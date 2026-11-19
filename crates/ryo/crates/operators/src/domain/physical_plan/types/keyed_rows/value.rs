//! Rows keyed by the hash of their key: one `hash_index` table, its keys' runs and each row's next row of the same key; and
//! the one hash and the one comparison of a key's cells.
use crate::domain::contract::types::PredicateOutcome;
use ryo_relations::domain::relations::types::{CellKind, Flag, Value};
use std::hash::{Hash as _, Hasher as _};

/// Rows keyed by the hash of their key, with no allocation per row and no key copied out: each key's first and last row and
/// its row count in a hash table, and the next row of the same key in `next`, ascending, so a key's rows are read in source
/// order. A key is one name column's text, or the cells of key columns, compared from the keyed rows' own cells.
#[derive(Debug, Default)]
pub(crate) struct KeyedRows {
    /// Each key's hash, first row and last row.
    pub(crate) keys: hashbrown::HashTable<KeyedRun>,
    /// The next row of the same key after each row; `NO_ROW` after the last.
    pub(crate) next: Vec<IndexedRow>,
    /// Spike (ryo-jenl): the key census class of the last build.
    pub(crate) census: u8,
    /// Spike (ryo-jenl): whether the keys are words, each key's hash exact.
    pub(crate) words: KeyWords,
    /// Spike (ryo-jenl): a one-word key's place in `runs` by its word less `base` (`NO_SLOT`: no key), for keys whose words
    /// span few more places than there are rows; empty when the keys are hashed.
    pub(crate) slots: Vec<u32>,
    /// Spike (ryo-jenl): the keys addressed by `slots`, in the order of their first rows.
    pub(crate) runs: Vec<KeyedRun>,
    /// Spike (ryo-jenl): the smallest word of a position-addressed index.
    pub(crate) base: u64,
    /// Spike (ryo-jenl): each row's word, read once while the index is built.
    pub(crate) scratch: Vec<u64>,
}

/// Spike (ryo-jenl): a slot of no key.
pub(crate) const NO_SLOT: u32 = u32::MAX;

/// One key of an index: its hash, the first and last of its rows and how many rows it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyedRun {
    /// The hash of the key: of its text for a name column, of its cells otherwise.
    pub(crate) hash: IndexKeyHash,
    /// Its first row.
    pub(crate) first: IndexedRow,
    /// Its last row.
    pub(crate) last: IndexedRow,
    /// How many rows it has.
    pub(crate) rows: ryo_values::domain::rows::types::RowCount,
}

/// The hash of a key: of its text for a name column, of its cells otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexKeyHash(pub(crate) u64);

/// A row of an indexed relation, by its place in the relation's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IndexedRow(pub(crate) u32);

/// The row after a key's last row.
pub(crate) const NO_ROW: IndexedRow = IndexedRow(u32::MAX);

/// The hash of a key's cells, in order: the one hash every index and row set keys its rows by.
pub(crate) fn hash_of_cells(cells: impl IntoIterator<Item = Value>) -> IndexKeyHash {
    // Spike (ryo-jenl): one multiply per cell, of the cell's payload word, the variant never hashed: equal cells have equal
    // words (a name's word is its text's hash, which is its equality), so it is a hash of the cells' equality.
    if crate::spike::dense_levers()[2] {
        let mut hash = 0u64;
        for cell in cells {
            hash = (hash.rotate_left(5) ^ cell_word(cell)).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
        return IndexKeyHash(hash);
    }
    let mut hasher = rustc_hash::FxHasher::default();
    for cell in cells {
        cell.hash(&mut hasher);
    }
    IndexKeyHash(hasher.finish())
}

/// Whether two keys hold the same cells, column by column: as many cells, each present and equal.
pub(crate) fn same_cells(
    one: impl ExactSizeIterator<Item = Option<Value>>,
    other: impl ExactSizeIterator<Item = Option<Value>>,
) -> PredicateOutcome {
    if one.len() == other.len()
        && one
            .zip(other)
            .all(|(cell, against)| cell.is_some() && cell == against)
    {
        PredicateOutcome::Holds
    } else {
        PredicateOutcome::Fails
    }
}

impl KeyedRows {
    /// A table of no row with `room` for rows' links: what the room cases give a stage or a pool.
    #[cfg(test)]
    pub(crate) fn with_room(room: crate::domain::kept_room::types::Room) -> Self {
        Self {
            keys: hashbrown::HashTable::new(),
            next: Vec::with_capacity(room.0),
            census: 0,
            words: KeyWords::Cells,
            slots: Vec::new(),
            runs: Vec::new(),
            base: 0,
            scratch: Vec::new(),
        }
    }

    /// Empties the table for its next rows, keeping its room.
    pub(crate) fn cleared(&mut self) {
        self.keys.clear();
        self.next.clear();
        self.words = KeyWords::Cells;
        self.slots.clear();
        self.runs.clear();
    }

    /// Spike (ryo-jenl): the key of word `word`, by position when the index is position-addressed, else by its exact hash.
    #[inline]
    pub(crate) fn word_run(&self, word: u64) -> Option<KeyedRun> {
        if self.slots.is_empty() && self.runs.is_empty() {
            let hash = word_hash(word);
            return self.keys.find(hash.0, |run| run.hash == hash).copied();
        }
        let at = usize::try_from(word.wrapping_sub(self.base)).ok()?;
        let slot = *self.slots.get(at)?;
        self.runs.get(usize::try_from(slot).ok()?).copied()
    }

    /// Spike (ryo-jenl): every key, hashed or position-addressed.
    pub(crate) fn each_run(&mut self) -> impl Iterator<Item = &mut KeyedRun> {
        self.keys.iter_mut().chain(self.runs.iter_mut())
    }
}

impl crate::domain::kept_room::types::KeptRoom for KeyedRows {
    fn kept(&mut self) {
        self.keys.kept();
        self.next.kept();
        self.slots.kept();
        self.runs.kept();
        self.scratch.kept();
    }

    #[cfg(test)]
    fn room(&self) -> crate::domain::kept_room::types::Room {
        self.keys.room().max(self.next.room())
    }
}

/// A keyed-row table a worker's pool keeps: within the room any scratch keeps past a file.
impl crate::domain::kept_room::types::Poolable for KeyedRows {
    fn fits(&self) -> crate::domain::kept_room::types::RoomFit {
        use crate::domain::kept_room::constants::KEPT_ROOM;
        if self.keys.capacity() <= KEPT_ROOM
            && self.next.capacity() <= KEPT_ROOM
            && self.slots.capacity() <= 4 * KEPT_ROOM
            && self.runs.capacity() <= KEPT_ROOM
            && self.scratch.capacity() <= KEPT_ROOM
        {
            crate::domain::kept_room::types::RoomFit::Fits
        } else {
            crate::domain::kept_room::types::RoomFit::Exceeds
        }
    }

    fn held(&self) -> crate::domain::kept_room::types::RoomHeld {
        if self.keys.capacity() == 0 && self.next.capacity() == 0 {
            crate::domain::kept_room::types::RoomHeld::Empty
        } else {
            crate::domain::kept_room::types::RoomHeld::Holds
        }
    }

    fn emptied(&mut self) {
        self.cleared();
    }
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
