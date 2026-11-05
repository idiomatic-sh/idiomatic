//! The types of the `render` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. The rows one file gave, as typed cells, waiting for their turn to leave.

use ryo_relations::domain::relations::types::Offset;

use {
    crate::domain::finish::types::ColumnNumber, crate::domain::finish::types::FileLine,
    ryo_values::domain::runs::types::RuleNumber,
};

use crate::domain::value_cells::types::Scene;
use ryo_values::domain::plan_parts::types::Heading;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::records::types::RenderedText;
use ryo_values::domain::records::types::{Cell, RowEnds, RowTotal, TextRange};

/// How many key columns a finding's row has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyCount(pub usize);

/// A rendered row, by its place among the rows rendered, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RowNumber(pub u32);

/// How many rows of a file may be rendered, or written: the rest are counted and nothing more.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RowLimit(pub usize);

/// A row of a table, by its place in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TableRow(pub u32);

/// Where one rendered row is among the rows of its file, and which rule gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowOrder {
    /// The rule that emitted the row.
    pub rule: RuleNumber,
    /// The line it is on; zero when it has no place in the file.
    pub line: FileLine,
    /// The character in the line.
    pub column: ColumnNumber,
    /// The row's place among the rows rendered.
    pub row: RowNumber,
}

/// The rows of one file as typed cells: every cell one after the other, the text of their file and text cells, where each row ends,
/// and the order the rows go out in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Rendered {
    /// The text of the file and text cells, row after row.
    pub text: RenderedText,
    /// Every cell, row after row, its text kept in `text`; a cell with no value is `None`.
    pub cells: Vec<Option<Cell<TextRange>>>,
    /// Where each row ends in `cells`, by the row's place among the rows rendered.
    pub ends: RowEnds,
    /// The rows, in the order they were rendered; sort it to choose the order they leave in.
    pub order: Vec<RowOrder>,
    /// How many rows the file had, of which `order` holds the ones rendered.
    pub total: RowTotal,
    /// The rows to order by place and keep, while the rows are chosen: the place, then the row.
    pub keys: Vec<RowKey>,
}

/// Where a row starts in its file, or that it has no place there; rows with no place sort first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum KeyOffset {
    /// The row has no place in its file.
    Unplaced,
    /// The row starts here.
    At(Offset),
}

/// A row of a table before it is rendered: where it is in its file, then its place in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RowKey {
    /// Where the row starts in its file.
    pub offset: KeyOffset,
    /// The row's place in the table.
    pub index: TableRow,
}

/// The id of the rule a finding is of, as its first cell reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FindingRule<'a>(pub &'a str);

/// Which cells of a row are rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shown<'a> {
    /// Every cell of the row.
    All,
    /// The row among the columns of an answer of more than one rule: the rule's id, then each column of the answer.
    Placed(RuleRow<'a>),
    /// A finding's cells: the rule's id as text, the subject, the number of key columns, the value of each key column, then each
    /// column of its row as its heading, as text, and its value; [`FindingParts`] reads them back.
    Finding(FindingRule<'a>),
}

/// A row of one rule among the columns of an answer that has the rows of more than one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RuleRow<'a> {
    /// The rule that gave the row, named in its first cell.
    pub rule: FindingRule<'a>,
    /// What fills each column of the answer.
    pub cells: &'a [AnswerCell],
}

/// A column of a rule's row, by its place among the rule's columns, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ColumnPlace(pub usize);

/// What fills a column of the answer in a rule's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnswerCell {
    /// The rule's own column of the answer's heading.
    Column(ColumnPlace),
    /// Nothing: the rule has no column of that heading.
    Empty,
}

/// How the rows the rules emit are answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Answering {
    /// As findings: a row is its rule, its path, its key and its columns as one cell, and rows that give the same rule, path and key
    /// are one.
    Findings,
    /// As the rules' own columns: every row, in the order of the files and, in a file, of the rules and the places.
    Rows(RowsAnswer),
}

/// An answer of the rules' own columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RowsAnswer {
    /// How many rows are written: every row is counted, and the rows after these are not written.
    pub limit: RowTotal,
    /// The headings of the answer's columns, before the line and the character.
    pub headings: Vec<Heading>,
    /// Whose columns the answer's are.
    pub columns: AnswerColumns,
}

/// Whose columns an answer of the rules' own columns has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnswerColumns {
    /// The one rule's: a row is every cell of the rule's row.
    One,
    /// More than one rule's: the rule's id, then each heading any rule has, once, in the order the rules first name it; for each
    /// rule, what fills each of those columns in its rows.
    Many(Vec<Vec<AnswerCell>>),
}

/// What to render of a row; the line and the character follow its cells, empty for a row with no place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowLayout<'a> {
    /// Which cells.
    pub shown: Shown<'a>,
}

/// What one call renders: the rule that gave the rows, the rows, what a row emits, the cells shown of it, the file they are of, and
/// how many rows may be made into cells.
#[derive(Clone, Copy)]
pub(crate) struct RenderInput<'a> {
    /// The rule that emitted the rows.
    pub rule: RuleNumber,
    /// The rows.
    pub table: &'a Table,
    /// What a row of the rule emits: its subject, its key and its headings.
    pub emit: &'a Emit,
    /// The cells shown of a row.
    pub layout: RowLayout<'a>,
    /// The file the rows are of.
    pub scene: Scene<'a>,
    /// How many rows are made into cells; the rest are counted.
    pub limit: RowLimit,
}

/// The parts of a finding's row, read back from its cells: what [`Shown::Finding`] wrote, then the line and the character every row
/// ends with. `T` holds the text of a cell, as [`Cell`] says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FindingParts<'a, T> {
    /// The id of the rule.
    pub rule: &'a T,
    /// What the finding is about.
    pub subject: &'a Option<Cell<T>>,
    /// The value of each key column, in order.
    pub key: &'a [Option<Cell<T>>],
    /// Each column of the row: its heading, a text cell, then its value.
    pub columns: &'a [Option<Cell<T>>],
    /// The line; none when the finding has no place.
    pub line: &'a Option<Cell<T>>,
    /// The character in the line; none when the finding has no place.
    pub column: &'a Option<Cell<T>>,
}

use ryo_values::domain::rows::types::Table;
