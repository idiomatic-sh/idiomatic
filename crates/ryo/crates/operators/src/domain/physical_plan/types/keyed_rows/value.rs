//! Rows keyed by the hash of their key: one `hash_index` table, its keys' runs and each row's next row of the same key.

/// Rows keyed by the hash of their key, with no allocation per row and no key copied out: each key's first and last row and
/// its row count in a hash table, and the next row of the same key in `next`, ascending, so a key's rows are read in source
/// order. A key is one name column's text, or the cells of key columns, compared from the keyed rows' own cells.
#[derive(Debug, Default)]
pub(crate) struct KeyedRows {
    /// Each key's hash, first row and last row.
    pub(crate) keys: hashbrown::HashTable<KeyedRun>,
    /// The next row of the same key after each row; `NO_ROW` after the last.
    pub(crate) next: Vec<IndexedRow>,
}

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

impl KeyedRows {
    /// A table of no row with `room` for rows' links: what the room cases give a stage or a pool.
    #[cfg(test)]
    pub(crate) fn with_room(room: crate::domain::kept_room::types::Room) -> Self {
        Self {
            keys: hashbrown::HashTable::new(),
            next: Vec::with_capacity(room.0),
        }
    }

    /// Empties the table for its next rows, keeping its room.
    pub(crate) fn cleared(&mut self) {
        self.keys.clear();
        self.next.clear();
    }
}

impl crate::domain::kept_room::types::KeptRoom for KeyedRows {
    fn kept(&mut self) {
        self.keys.kept();
        self.next.kept();
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
        if self.keys.capacity() <= KEPT_ROOM && self.next.capacity() <= KEPT_ROOM {
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
