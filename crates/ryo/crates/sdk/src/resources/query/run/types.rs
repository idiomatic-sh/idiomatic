//! The types of `query run`: what is asked and what comes back.

use std::path::PathBuf;

use crate::domain::cell::types::Slot;
use crate::domain::file::types::Unread;
use crate::domain::query::types::RyoQlQuery;

/// How many rows a run writes when its caller does not say.
pub const DEFAULT_LIMIT: RowLimit = RowLimit(20);

/// How many rows a run keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowLimit(pub u64);

/// A number of rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowCount(pub u64);

/// The heading of a column of an answer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Heading(pub Box<str>);

/// One row of an answer: its cells in the order of the headings, each typed; none for a value that reads as nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Row(pub Vec<Slot>);

/// One query to explore over a repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRequest {
    /// The repository's root directory.
    pub root: PathBuf,
    /// The query.
    pub query: RyoQlQuery,
    /// How many rows are kept: every row is counted, and the rows after these are not.
    pub limit: RowLimit,
}

/// What a run answers: the rows kept, and the files nothing read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RunReport {
    /// The headings of the columns.
    pub columns: Vec<Heading>,
    /// The rows kept, each cell typed.
    pub rows: Vec<Row>,
    /// The files no linked language read, with why.
    pub unread: Vec<Unread>,
    /// How many rows the query has.
    pub total: RowCount,
    /// How many of them were kept.
    pub shown: RowCount,
}
