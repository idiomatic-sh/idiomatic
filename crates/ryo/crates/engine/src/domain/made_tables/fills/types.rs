//! The types of the `fills` part: a made table's fill as the code holds it.

use crate::domain::code::types::{PoolAt, WordSlot};
use ryo_relations::domain::relations::types::Count;

/// The fills the fill side runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FillKind {
    /// A definition's rows: each pipeline's write.
    Rows,
    /// One row for each key of the gathered input: the key, then how many rows hold it.
    Count,
    /// The gathered input's rows, each key's together and ascending by the order column.
    Sorted,
    /// The rows of the gathered input at one end of their key's rows by the order column.
    Ends,
    /// Each distinct row of the gathered input, then its place among its key's rows by the order column.
    Places,
    /// `follow`: for each key of the gathered edges, each pair of a source and a fixed point it reaches.
    Follow,
    /// Two gathered inputs paired within each key, first in, first out: the left row, then the right row.
    Match,
    /// Rows written in the code.
    Constant,
}

/// A made table's fill: which fill, the table's width, the fill's one parameter (whether a definition's rows drop
/// repeats, the distinct column of a count or none, how many rows a constant table has), and where its inputs or its
/// constant rows start in the pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TableFill {
    /// Which fill.
    pub(crate) kind: FillKind,
    /// How many columns the table has.
    pub(crate) width: Count,
    /// The fill's one parameter.
    pub(crate) parameter: WordSlot,
    /// Where the inputs, or the constant rows, start.
    pub(crate) inputs: PoolAt,
}

/// One input of a fill: the entries of its pipelines, its key columns, and its order column when it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FillInputs<'c> {
    /// The entries of its pipelines.
    pub(crate) entries: &'c [WordSlot],
    /// Its key columns.
    pub(crate) keys: &'c [WordSlot],
    /// Its order column.
    pub(crate) order: Option<Count>,
    /// Where the record goes on after it: the next input, of a fill of two.
    pub(crate) after: PoolAt,
}
