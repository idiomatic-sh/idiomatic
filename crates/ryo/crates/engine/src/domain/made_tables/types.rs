//! The types of the `made_tables` concept: the rows of a table the engine makes for a file, and the room its fills work in.
//! Crate-private.

use crate::domain::indexes::types::{IndexArena, SourceRow};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Count, Integer};
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The rows of one table for a file: its cells row by row, kept between files and emptied, never freed.
#[derive(Debug, Default)]
pub(crate) struct TableRows {
    /// The cells, row by row.
    pub(crate) cells: Vec<CellWord>,
    /// How many cells a row has.
    pub(crate) width: Count,
    /// How many rows it holds: a table of no column has rows too.
    pub(crate) count: Count,
    /// The rows it holds, by what they hold: what a write that drops repeats asks.
    pub(crate) seen: hashbrown::HashTable<SourceRow>,
    /// Of a sorted table: its order column, row by row, as the numbers a `Latest` searches.
    pub(crate) orders: Vec<Integer>,
}

/// The room a fill that groups its input works in: the input's rows as gathered, the index that groups them, and one row
/// being written. Kept between fills, never freed.
#[derive(Debug, Default)]
pub(crate) struct GroupRoom {
    /// The index of the input by its keys.
    pub(crate) index: IndexArena,
    /// The key columns as the operators crate names them.
    pub(crate) keys: Vec<ColumnIndex>,
    /// One group's row, as its meaning wrote it.
    pub(crate) row: Vec<CellWord>,
    /// The values of the counted column in one group, to count the distinct ones.
    pub(crate) values: Vec<CellWord>,
}
