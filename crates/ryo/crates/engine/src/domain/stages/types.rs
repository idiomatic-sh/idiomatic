//! The types of the `stages` concept: what a stage's loop reads and writes. A batch is columns of cells, one per register
//! its pipeline holds; a loop reads slices and writes vectors, and runs no step program. Crate-private.

use crate::domain::indexes::types::ColumnView;
use crate::domain::steps::types::{BoundKind, TestOperation};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Count, FileId};

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
    /// The bits every handle given carries besides the row's number: the file of a node, nothing of another source.
    pub(crate) tag: CellWord,
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

/// The range a probe is held to: the order column of its index's source, and for each row its two bounds, each in or out.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyRange<'a> {
    /// The order column of the source.
    pub(crate) order: ColumnView<'a>,
    /// The low bound of each row.
    pub(crate) low: BatchValue<'a>,
    /// Whether a row equal to the low bound is in.
    pub(crate) low_end: BoundKind,
    /// The high bound of each row.
    pub(crate) high: BatchValue<'a>,
    /// Whether a row equal to the high bound is in.
    pub(crate) high_end: BoundKind,
}

/// The handles of a source's rows, one after the other: the first row, the row past the last, and the bits every handle
/// carries besides the row's number (the file of a node; nothing of another source).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct HandleRun {
    /// The first row.
    pub(crate) first: Count,
    /// The row past the last.
    pub(crate) end: Count,
    /// The bits every handle carries besides the row's number.
    pub(crate) tag: CellWord,
}

/// What a node row's cells are read from besides the table: the word of each kind's and each field's name by its code,
/// the file's bytes, and the file.
#[derive(Clone, Copy, Debug)]
pub(crate) struct NodeNames<'a> {
    /// The word of each kind's name, by the kind's code.
    pub(crate) kinds: &'a [CellWord],
    /// The word of each field's name, by the field's code.
    pub(crate) fields: &'a [CellWord],
    /// The file's bytes.
    pub(crate) text: ByteSlice<'a>,
    /// The file.
    pub(crate) file: FileId,
}

/// A test of a text over a batch: the test, its left value, and its right: a value, or the cells of a set.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TextTest<'a> {
    /// The test.
    pub(crate) test: TestOperation,
    /// The left value of each row.
    pub(crate) left: BatchValue<'a>,
    /// The right value of each row, when it is no set.
    pub(crate) right: Option<BatchValue<'a>>,
    /// The cells of the right set, when it is one.
    pub(crate) set: &'a [CellWord],
}
