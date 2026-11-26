//! Rows kept once, each by the hash of all its cells and its place among the rows it was kept from: a distinct's rows being
//! made, or a set operation's right side. A row set never follows a chain of rows, so it holds no link per row.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::physical_plan::types::CellStart;
use crate::domain::physical_plan::types::keyed_rows::value::{
    IndexKeyHash, IndexedRow, KeyWords, hash_of_cells, key_words, word_hash,
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
            self.words = key_words(table.kinds.iter().copied());
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





}
