//! The types of the `cell` concept: one value of a row, typed, as a host declares it in the rows of its relations and as a run
//! answers it in a finding and in the rows of a query. Nothing is text that is not text.

use ryo_language::domain::relations::types::Span;

use crate::domain::file::types::RelativePath;

/// One value of a row, typed: a cell of a row of a relation the host declares, and a cell of a row a run answers. A value the engine
/// holds in another kind is one of these with nothing lost: a name is its text; a node and a yes or no are the text they read as.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Cell {
    /// A file of the repository, by its path relative to the root; in a row the host declares, a path the run does not list makes no
    /// row.
    File(RelativePath),
    /// Text: a name, or what a node or a yes or no reads as.
    Text(HostText),
    /// A whole number.
    Number(HostNumber),
    /// A stretch of a source file, in bytes, as the languages write it: `languages::NO_SPAN` when the parser gave none.
    Span(Span),
}

/// What a row holds at one place: a cell, or nothing where the value reads as nothing (a name of another file, a file the run does not
/// list). Nothing is its own case, not a text standing for it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Slot {
    /// The value reads as nothing.
    #[default]
    Nothing,
    /// The value, typed.
    Is(Cell),
}

/// The text of a cell.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostText(pub Box<str>);

/// The whole number of a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostNumber(pub i64);
