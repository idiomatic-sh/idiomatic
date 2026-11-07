//! The types of the `row_texts` concept: the text of rows, where a rule, a line, a row and a column place in it, and the totals of rows. Plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_relations::domain::relations::types::{Integer, Span};

/// One value of a row of the answer, typed: a row of a rule's own columns and a finding's row are these, and only the command line
/// writes them as text. `T` holds the text of a file or a text cell: `Box<str>` where the cell is owned, `&str` where a row is lent,
/// and a [`TextRange`] where the text is kept in the text of its rows. A value the engine holds in another kind is one of these with
/// nothing lost: a name is its text; a node and a yes or no are the text they read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cell<T = Box<str>> {
    /// A file of the repository, by its path relative to the root.
    File(T),
    /// Text: a name, or what a node or a yes or no reads as.
    Text(T),
    /// A whole number.
    Number(Integer),
    /// A stretch of a file, in bytes, as the engine holds it: `NO_SPAN` when the parser gave none.
    Span(Span),
}

/// Where the text of a kept cell is in the text of its rows, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextRange {
    /// Where it starts.
    pub start: RowsTextAt,
    /// Where it ends.
    pub end: RowsTextAt,
}

/// A place in the text of rows, in bytes from its start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowsTextAt(pub u32);

/// The text of a file or a text cell, lent from where it is kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellText<'a>(pub &'a str);

/// The text of the file and text cells of rows, one after the other, owned.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowsText(pub Box<str>);

/// The text of the file and text cells of rows, one after the other, lent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowsTextView<'a>(pub &'a str);

/// The text of the file and text cells of a file's rows, one after the other, as it is written: the buffer a worker keeps the text
/// of its cells in and reuses for the next file, so it is a `String` and not a `Box<str>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderedText(pub String);

impl std::ops::Deref for RenderedText {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl std::ops::DerefMut for RenderedText {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl std::fmt::Write for RenderedText {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

/// Where each row ends among the cells of the rows, row after row, owned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowEnds(pub Vec<u32>);

/// The rule that emitted a row, by its place among the rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedRule(pub u32);

/// The line a row is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedLine(pub u32);

/// A row, by its place among the rows rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedRow(pub u32);

/// The character in the line a row is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedColumn(pub u32);

/// Where one row of findings is among the rows, and which rule gave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowPlacing {
    /// The rule that emitted the row.
    pub rule: PlacedRule,
    /// The line it is on.
    pub line: PlacedLine,
    /// The character in the line.
    pub column: PlacedColumn,
    /// The row's place among the rows rendered.
    pub row: PlacedRow,
}

/// Findings as typed cells: every cell one after the other, the text of their file and text cells, where each row ends, and the
/// order the rows leave in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FindingRows {
    /// The text of the file and text cells, one after the other.
    pub text: RowsText,
    /// Every cell, row after row, its text kept in `text`; a cell with no value is `None`.
    pub cells: Vec<Option<Cell<TextRange>>>,
    /// Where each row ends in `cells`.
    pub ends: RowEnds,
    /// The rows, in the order they leave in.
    pub order: Vec<RowPlacing>,
    /// How many rows the file had.
    pub total: RowTotal,
}

/// A count of rows, to be written as a number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowTotal(pub u64);
