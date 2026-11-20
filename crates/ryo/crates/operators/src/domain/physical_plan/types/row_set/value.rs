//! Rows kept once, each by the hash of all its cells and its place among the rows it was kept from: a distinct's rows being
//! made, or a set operation's right side. A row set never follows a chain of rows, so it holds no link per row.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::kept_room::constants::KEPT_ROOM;
use crate::domain::kept_room::types::{Poolable, RoomFit, RoomHeld};
use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::keyed_rows::value::{
    IndexKeyHash, IndexedRow, KeyWords, hash_of_cells, key_words, same_cells, word_hash,
};
use crate::domain::table_scratch::types::SetMembership;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::service::row_in;
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// Whether a row is the first of its kind among the rows kept before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FirstSeen {
    /// No row kept before it holds the same cells: it is kept.
    First,
    /// A row kept before it holds the same cells: it is not kept.
    Repeat,
}

/// Rows kept once, each the first of its kind, compared from the rows' own cells: no row is copied out.
#[derive(Debug, Default)]
pub(crate) struct RowSet {
    /// Each kept row, by the hash of its cells; seen by its tests, which plant a row of a colliding hash.
    pub(super) kept: hashbrown::HashTable<SetEntry>,
    /// Spike (ryo-jenl): whether the rows kept are words (`KeyWords`), each hash exact.
    pub(super) words: KeyWords,
}

/// One kept row: the hash of its cells and its place among the rows it was kept from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SetEntry {
    /// The hash of all its cells.
    pub(super) hash: IndexKeyHash,
    /// Its place among the rows it was kept from.
    pub(super) row: IndexedRow,
}

/// Whether row `row` of a side of `kinds`, its cells read through `cell`, holds the cells of `cells`.
fn same_row(
    kinds: &[CellKind],
    cell: &impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    row: RowIndex,
    cells: &[Value],
) -> PredicateOutcome {
    same_cells(
        cells.iter().map(|value| Some(*value)),
        columns(kinds).map(|column| cell(row, column)),
    )
}

/// Every column of rows of `kinds`, in order.
pub(crate) fn columns(kinds: &[CellKind]) -> impl ExactSizeIterator<Item = ColumnIndex> {
    (0..kinds.len()).map(|at| ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX)))
}

/// The row a kept row is, among the rows it was kept from.
fn indexed(row: IndexedRow) -> RowIndex {
    RowIndex(usize::try_from(row.0).unwrap_or(usize::MAX))
}

impl RowSet {
    /// Keeps the row `table` holds from cell `start` when no row kept before it holds the same cells: `First`, and it is kept;
    /// `Repeat` otherwise. The set keeps the rows of `table` in order, so the row kept is the table's next row.
    pub(crate) fn first_seen(&mut self, table: &Table, start: CellStart) -> FirstSeen {
        let new = table.cells.get(start.0..).unwrap_or_default();
        if self.kept.is_empty() {
            self.words = if crate::spike::dense_lever("repeats") {
                key_words(table.kinds.iter().copied())
            } else {
                KeyWords::Cells
            };
        }
        // Spike (ryo-jenl): a row that is one word hashes by the word: cheaper than by its cells, the same rows compared.
        let word = match (self.words, new) {
            (KeyWords::Cells, _) => None,
            (_, [one]) => self.words.word(*one, None, true),
            (_, [one, two]) => self.words.word(*one, Some(*two), true),
            _ => None,
        };
        let hash = word.map_or_else(|| hash_of_cells(new.iter().copied()), word_hash);
        if self
            .kept
            .find(hash.0, |kept| {
                kept.hash == hash && row_in(table, indexed(kept.row)) == Some(new)
            })
            .is_some()
        {
            return FirstSeen::Repeat;
        }
        let row = IndexedRow(u32::try_from(self.kept.len()).unwrap_or(u32::MAX));
        let _: hashbrown::hash_table::OccupiedEntry<'_, SetEntry> =
            self.kept
                .insert_unique(hash.0, SetEntry { hash, row }, |kept| kept.hash.0);
        FirstSeen::First
    }

    /// Empties itself, then keeps once each of `count` rows of `kinds`, each cell read through `cell`, by all its cells: a
    /// set operation's right side, which `holds` then asks.
    ///
    /// # Errors
    ///
    /// `NoSuchColumn` for a column of the kinds that a row does not hold.
    pub(crate) fn built(
        &mut self,
        count: RowCount,
        kinds: &[CellKind],
        cell: impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
    ) -> Result<(), PortableOperatorFault> {
        self.kept.clear();
        self.words = KeyWords::Cells;
        if crate::spike::keystats() {
            crate::spike::key_rows(3, crate::spike::key_class(kinds.iter().copied()), count.0 as u64);
        }
        let mut cells = Vec::with_capacity(kinds.len());
        for at in 0..count.0 {
            cells.clear();
            for column in columns(kinds) {
                cells.push(
                    cell(RowIndex(at), column)
                        .ok_or(PortableOperatorFault::NoSuchColumn { index: column })?,
                );
            }
            let hash = hash_of_cells(cells.iter().copied());
            if self
                .kept
                .find(hash.0, |kept| {
                    kept.hash == hash
                        && same_row(kinds, &cell, indexed(kept.row), &cells)
                            == PredicateOutcome::Holds
                })
                .is_none()
            {
                let row = IndexedRow(u32::try_from(at).unwrap_or(u32::MAX));
                let _: hashbrown::hash_table::OccupiedEntry<'_, SetEntry> = self
                    .kept
                    .insert_unique(hash.0, SetEntry { hash, row }, |kept| kept.hash.0);
            }
        }
        Ok(())
    }

    /// Whether `row`, a row's cells, equals a row of the side of `kinds` the set was built from, its cells read through `cell`.
    pub(crate) fn holds(
        &self,
        kinds: &[CellKind],
        cell: impl Fn(RowIndex, ColumnIndex) -> Option<Value>,
        row: &[Value],
    ) -> SetMembership {
        let hash = hash_of_cells(row.iter().copied());
        if self
            .kept
            .find(hash.0, |kept| {
                kept.hash == hash
                    && same_row(kinds, &cell, indexed(kept.row), row) == PredicateOutcome::Holds
            })
            .is_some()
        {
            SetMembership::In
        } else {
            SetMembership::Out
        }
    }

    /// How many rows it keeps: none for a set not built in the open file, or built from no row.
    pub(crate) fn count(&self) -> RowCount {
        RowCount(self.kept.len())
    }

    /// A set of no row with `room` for rows: what the room cases give a stage or a pool.
    #[cfg(test)]
    pub(crate) fn with_room(room: crate::domain::kept_room::types::Room) -> Self {
        Self {
            kept: hashbrown::HashTable::with_capacity(room.0),
            words: KeyWords::Cells,
        }
    }

    /// The room it keeps.
    #[cfg(test)]
    pub(crate) fn room(&self) -> crate::domain::kept_room::types::Room {
        crate::domain::kept_room::types::Room(self.kept.capacity())
    }

    /// Spike (ryo-fmx3): room for `rows` more rows.
    pub(crate) fn reserved(&mut self, rows: usize) {
        self.kept.reserve(rows, |kept| kept.hash.0);
    }

    /// Empties the set for its next rows, keeping its room.
    pub(crate) fn cleared(&mut self) {
        self.kept.clear();
        self.words = KeyWords::Cells;
    }
}

/// A row set a worker's pool keeps: within the room any scratch keeps past a file.
impl Poolable for RowSet {
    fn fits(&self) -> RoomFit {
        if self.kept.capacity() <= KEPT_ROOM {
            RoomFit::Fits
        } else {
            RoomFit::Exceeds
        }
    }

    fn held(&self) -> RoomHeld {
        if self.kept.capacity() == 0 {
            RoomHeld::Empty
        } else {
            RoomHeld::Holds
        }
    }

    fn emptied(&mut self) {
        self.cleared();
    }
}
