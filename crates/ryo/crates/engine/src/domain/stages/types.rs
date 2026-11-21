//! The types of the `stages` concept: what a stage's loop reads and writes. A batch is columns of cells, one per register
//! its pipeline holds; a loop reads slices and writes vectors, and runs no step program. Crate-private.

use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// A row of a batch, by its number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BatchRow(pub(crate) u32);

/// Where a loop that gives several rows for one stopped when its batch was full: the row in, and how many of its rows were
/// already given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FanCursor {
    /// The row in.
    pub(crate) row: Count,
    /// How many of its rows were given.
    pub(crate) within: Count,
}

/// What a loop that gives several rows for one writes, and where it is: for each row out the row in it came from and its new
/// cell; the most rows a batch out holds; and where the loop stopped when the batch was full.
#[derive(Debug, Default)]
pub(crate) struct FanRows {
    /// The row in each row out came from.
    pub(crate) up: Vec<BatchRow>,
    /// The new cell of each row out.
    pub(crate) out: Vec<CellWord>,
    /// The most rows a batch out holds.
    pub(crate) limit: Count,
    /// Where the loop is among the rows in.
    pub(crate) cursor: FanCursor,
}

/// Whether a loop that gives several rows for one has read every row in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Batches {
    /// Every row in was read: the batch out is the last.
    Done,
    /// The batch out is full: the loop goes on from its cursor.
    More,
}

/// A key a loop asks an index for each row: its cells, the constant ones written, and the cells that change with the row.
#[derive(Debug)]
pub(crate) struct BatchKey<'a> {
    /// The key's cells.
    pub(crate) cells: &'a mut [CellWord],
    /// The cells that change with the row.
    pub(crate) parts: &'a [KeyPart<'a>],
}

/// One cell of a key that changes with the row: its place in the key and the column it is read from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyPart<'a> {
    /// Its place in the key.
    pub(crate) place: Count,
    /// The column of the batch.
    pub(crate) column: &'a [CellWord],
}

/// What a loop reads for each row: a column of the batch, or one cell that is the same for every row.
#[derive(Clone, Copy, Debug)]
pub(crate) enum BatchValue<'a> {
    /// A column of the batch.
    Column(&'a [CellWord]),
    /// One cell for every row.
    Constant(CellWord),
}

/// A comparison of two cells a `Keep` loop decides without a text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellTest {
    /// The cells are the same.
    Equal,
    /// The cells differ.
    NotEqual,
    /// The left number is below the right.
    Less,
    /// The left number is at most the right.
    LessOrEqual,
    /// The left number is above the right.
    Greater,
    /// The left number is at least the right.
    GreaterOrEqual,
}

/// Which rows a loop keeps by what a key holds: those whose key has a row, or those whose key has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeptWhen {
    /// The rows whose key has a row.
    Present,
    /// The rows whose key has none.
    Absent,
}

/// What a test says of one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowKept {
    /// The row is kept.
    Yes,
    /// The row is dropped.
    No,
}
