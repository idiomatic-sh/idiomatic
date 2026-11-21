//! The types of the `stages` concept: what a stage's loop reads and writes. A batch is columns of cells, one per register
//! its pipeline holds; a loop reads slices and writes vectors, and runs no step program. Crate-private.

use crate::domain::code::types::PoolAt;
use crate::domain::indexes::types::ColumnView;
use crate::domain::indexes::types::FileStamp;
use crate::domain::indexes::types::RowRun;
use crate::domain::stages::verdicts::types::TextVerdicts;
use crate::domain::steps::types::{BoundKind, StepRegex, TestOperation};
use ryo_language::domain::facts::service::intern;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::facts::types::FileWords;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::domain::functions::types::MadeText;
use ryo_operators::domain::capture::types::{CaptureOutput, CaptureSpans};
use ryo_operators::domain::contract::errors::ContractError;
use ryo_operators::domain::patterns::types::{PatternMatcher, TextGlobSet};
use ryo_operators::domain::source_matches::types::TextRowDriver;
use ryo_operators::domain::table_scratch::types::Patterns;
use ryo_relations::domain::relations::service::to_bits;
use ryo_relations::domain::relations::types::{Count, FileId, Value};
use ryo_relations::domain::terms::types::{NameText, Symbol};
use ryo_values::domain::row_texts::types::CellText;

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

/// What a test of a text is decided with besides the file's words: the operators crate's matcher, the sources of the
/// rule set's regular expressions by number, and the expressions compiled so far.
#[derive(Debug)]
pub(crate) struct TextMeans<'a> {
    /// The matcher.
    pub(crate) matcher: PatternMatcher<'a>,
    /// The sources of the regular expressions, by number.
    pub(crate) regexes: &'a [StepRegex],
    /// The expressions compiled so far, by source.
    pub(crate) compiled: &'a mut Patterns,
    /// What each test of a text against constants holds for, by word, as far as it was asked.
    pub(crate) verdicts: &'a mut TextVerdicts,
    /// The file: a file's own words are other texts in the next.
    pub(crate) stamp: FileStamp,
}

/// What one text is decided with, when a test's verdict for its word is not known yet: the matcher, the expressions,
/// and the test's globs, compiled the first time.
#[derive(Debug)]
pub(crate) struct Deciding<'a, 'g> {
    /// The matcher.
    pub(crate) matcher: PatternMatcher<'a>,
    /// The sources of the regular expressions, by number.
    pub(crate) regexes: &'a [StepRegex],
    /// The expressions compiled so far, by source.
    pub(crate) compiled: &'a mut Patterns,
    /// The test's globs, once compiled.
    pub(crate) globs: &'g mut Option<TextGlobSet>,
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
    /// Where the right set lies in the pool, when it is one: what names it.
    pub(crate) set_at: Option<PoolAt>,
}

/// What a call of a per-row meaning is made with besides the file's words: the operators crate's matcher, the sources of
/// the rule set's regular expressions, the expressions compiled so far, and the file's bytes.
#[derive(Debug)]
pub(crate) struct CallMeans<'a> {
    /// The matcher.
    pub(crate) matcher: PatternMatcher<'a>,
    /// The sources of the regular expressions, by number.
    pub(crate) regexes: &'a [StepRegex],
    /// The expressions compiled so far, by source.
    pub(crate) compiled: &'a mut Patterns,
    /// The file's bytes.
    pub(crate) source: ByteSlice<'a>,
}

/// Where a call's rows go: the cells of every row it gave, row by row; for each row of the batch the stretch of those
/// rows it gave; and the scratch the meanings work in.
#[derive(Debug)]
pub(crate) struct CallOut<'r> {
    /// The cells of the rows given, row by row, as many a row as the call has outputs.
    pub(crate) answers: &'r mut Vec<CellWord>,
    /// For each row of the batch, the rows it gave.
    pub(crate) spans: &'r mut Vec<RowRun>,
    /// The scratch.
    pub(crate) scratch: &'r mut CallScratch,
}

/// The scratch the per-row meanings work in: kept between batches, never freed.
#[derive(Debug, Default)]
pub(crate) struct CallScratch {
    /// A row's text, owned while words are interned from it.
    pub(crate) text: ScratchText,
    /// A constant text of the call: a replacement, a slot pattern, a group's name.
    pub(crate) constant: ScratchText,
    /// A text a meaning writes.
    pub(crate) written: ScratchText,
    /// The slots a capture has matched so far.
    pub(crate) slots: TextRanges,
    /// The values a meaning reads of a row.
    pub(crate) values: Vec<Value>,
    /// The row a source search hands on.
    pub(crate) joined: Vec<Value>,
    /// The ways a capture matched: every way's slots, one way after the other.
    pub(crate) ways: CaptureWays,
    /// The texts of a function's arguments for one row, and the text it made of them.
    pub(crate) texts: FunctionTexts,
}

/// The texts a function reads and the one it makes, for one row: owned by the scratch.
#[derive(Debug, Default)]
pub(crate) struct FunctionTexts {
    /// The arguments' texts, in order.
    pub(crate) arguments: Vec<MadeText>,
    /// The text made.
    pub(crate) made: MadeText,
}

/// A text a meaning reads or writes, owned by the scratch.
#[derive(Debug, Default)]
pub(crate) struct ScratchText(pub(crate) String);

/// Stretches of a text, each from a byte to before another.
#[derive(Debug, Default)]
pub(crate) struct TextRanges(pub(crate) Vec<std::ops::Range<usize>>);

/// Every way a capture matched a path: the ranges of each way's slots, one way after the other, and how many ways.
#[derive(Debug, Default)]
pub(crate) struct CaptureWays {
    /// The ranges, way by way.
    pub(crate) ranges: TextRanges,
    /// How many ways.
    pub(crate) ways: Count,
}

impl CaptureOutput for CaptureWays {
    fn captures(&mut self, ranges: CaptureSpans<'_>) {
        self.ranges.0.extend_from_slice(ranges.0);
        self.ways = Count(self.ways.0.saturating_add(1));
    }
}

/// The rows a source search gives, taken as cells: each row's values appended to the answers, a captured text interned
/// in the file's words.
#[derive(Debug)]
pub(crate) struct SearchedRows<'r> {
    /// The file's words.
    pub(crate) words: &'r mut FileWords,
    /// The cells of the rows given.
    pub(crate) answers: &'r mut Vec<CellWord>,
    /// How many rows were given.
    pub(crate) rows: Count,
}

impl TextRowDriver for SearchedRows<'_> {
    fn intern(&mut self, text: CellText<'_>) -> Value {
        let word = intern(self.words, NameText(text.0));
        Value::Symbol(Symbol(u32::try_from(word.0).unwrap_or(u32::MAX)))
    }

    fn emit(&mut self, row: &[Value]) -> Result<(), ContractError> {
        self.answers
            .extend(row.iter().map(|value| CellWord(to_bits(*value).0)));
        self.rows = Count(self.rows.0.saturating_add(1));
        Ok(())
    }
}
