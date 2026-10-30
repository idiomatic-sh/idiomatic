//! The failures of `linter lint`.

use super::types::{Port, ProviderName, RowNumber, RuleId};
use crate::domain::host::types::{HostColumnName, HostKind, HostRelationName};
use crate::domain::query::errors::ValidateError;

/// Why `lint` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LintError {
    /// A rule's query is not correct RyoQL.
    #[error("rule `{id}`: {error}")]
    Rule {
        /// The rule.
        id: RuleId,
        /// What is wrong with its text, and where.
        error: ValidateError,
    },
    /// A rule's query could not be run.
    #[error("rule `{id}`: {reason}")]
    Unrunnable {
        /// The rule.
        id: RuleId,
        /// Why.
        reason: String,
    },
    /// A rule's query has a parameter nothing was bound to.
    #[error("rule `{id}` has no value for a parameter it declares")]
    MissingParameter {
        /// The rule.
        id: RuleId,
    },
    /// The repository does not fit the memory budget.
    #[error("the repository is {size} bytes and the budget is {budget}")]
    TooLarge {
        /// The bytes of the repository.
        size: u64,
        /// The bytes the run may use.
        budget: u64,
    },
    /// A language wrote output for a file that the file cannot hold: a language bug, refused before anything is kept.
    #[error("language `{provider}` wrote output its file cannot hold: {code}")]
    Provider {
        /// The language.
        provider: ProviderName,
        /// What was wrong.
        code: ProviderCode,
    },
    /// The run holds more memory than the budget allows: refused before the machine runs out.
    #[error("the run holds {used} bytes and the budget is {budget}")]
    MemoryBudgetExceeded {
        /// The bytes the run holds.
        used: u64,
        /// The bytes it may use.
        budget: u64,
        /// The rules the run was working for.
        plans: Vec<RuleId>,
    },
    /// Two relations the host declared have one name, so a rule could not say which it reads.
    #[error("the host declared two relations named `{name}`")]
    DuplicateHostRelation {
        /// The name both have.
        name: HostRelationName,
    },
    /// A row of a relation the host declared has more or fewer cells than the relation has columns.
    #[error("row {} of `{relation}` has {given} cells and the relation has {expected} columns", row.0)]
    HostRowWidth {
        /// The relation.
        relation: HostRelationName,
        /// The row.
        row: RowNumber,
        /// How many columns the relation has.
        expected: usize,
        /// How many cells the row has.
        given: usize,
    },
    /// A cell of a row of a relation the host declared is not of the kind of its column.
    #[error("row {} of `{relation}` has a cell in `{column}` that is not {expected:?}", row.0)]
    HostCellKind {
        /// The relation.
        relation: HostRelationName,
        /// The row.
        row: RowNumber,
        /// The column.
        column: HostColumnName,
        /// What the column holds.
        expected: HostKind,
    },
    /// A directory the caller named is not text, so it cannot be named to the run.
    #[error("the path {path:?} is not text")]
    PathNotText {
        /// The path.
        path: std::path::PathBuf,
    },
    /// A part of the system failed.
    #[error("{what:?}: {reason}")]
    Io {
        /// Which part.
        what: Port,
        /// What it said.
        reason: String,
    },
}

/// What was wrong with the output a language wrote for a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProviderCode {
    /// A span ends before it starts, or after the file ends.
    #[error("a span is not within its file")]
    SpanOutsideFile,
    /// A node is at a place the file's table does not have.
    #[error("a node is not in its file's table")]
    NodeOutsideTable,
    /// A node belongs to another file.
    #[error("a node is of another file")]
    NodeOfAnotherFile,
    /// A cell that names a file names one that is not the file visited.
    #[error("a cell names a file that is not the one visited")]
    FileNotVisited,
}
