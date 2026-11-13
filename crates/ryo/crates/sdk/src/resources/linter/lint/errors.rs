//! The failures of `linter lint`.

use super::types::{Port, ProviderName, RowNumber, RuleId};
use crate::domain::file::types::RelativePath;
use crate::domain::host::types::{HostColumnName, HostKind, HostRelationName};
use crate::domain::query::errors::ValidateError;

/// Why `lint` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LintError {
    /// The engine could not execute the prepared request.
    #[error("execution: {reason}")]
    Execution {
        /// Why execution failed.
        reason: String,
    },
    /// A required named publication could not be admitted in this execution.
    #[error("observation execution failed: {reason}")]
    ObservationExecution { reason: String },
    /// The explicitly declared named-output dependency contract is invalid.
    #[error("invalid observation bindings: {reason}")]
    ObservationBindings {
        /// Configuration failure before file execution.
        reason: String,
    },
    /// An explicit source provider is unavailable at the request edge.
    #[error("explicit source provider `{name}` must name exactly one linked provider")]
    SourceLanguage {
        /// The exact provider name supplied by the caller.
        name: String,
    },
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
    /// A project search root or inventory entry does not name a repository-relative path.
    #[error(
        "invalid project path {path:?}: expected a repository-relative path without parent traversal"
    )]
    ProjectPath {
        /// The caller's path before normalization.
        path: RelativePath,
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

// The conversions from the core's failures are the entry's own: a public `From` impl would put the core's errors on the SDK's surface.
impl LintError {
    /// The caller's failure for a core failure of a lint.
    pub(super) fn from_core(error: ryo_core::inbound::use_cases::lint::errors::LintError) -> Self {
        use ryo_core::inbound::use_cases::lint::errors::LintError as CoreError;
        use ryo_core::inbound::use_cases::lint::types::ProviderFault as CoreCode;
        let io = |what: Port, reason: String| Self::Io { what, reason };
        match error {
            CoreError::Execution { reason } => Self::Execution { reason },

            CoreError::SourceLanguage { name } => Self::SourceLanguage { name },
            CoreError::ObservationBindings { reason } => Self::ObservationBindings { reason },
            CoreError::ObservationExecution { reason } => Self::ObservationExecution { reason },
            CoreError::Query { id, error } => Self::Rule {
                id: RuleId(id.into()),
                error: ValidateError::from_refused(error),
            },
            CoreError::Rule { id, reason } | CoreError::UnknownKind { id, reason } => {
                Self::Unrunnable {
                    id: RuleId(id.into()),
                    reason,
                }
            }
            CoreError::MissingParameter { id } => Self::MissingParameter {
                id: RuleId(id.into()),
            },
            CoreError::TooLarge { size, budget } => Self::TooLarge { size, budget },
            CoreError::MemoryBudgetExceeded {
                used,
                budget,
                plans,
            } => Self::MemoryBudgetExceeded {
                used,
                budget,
                plans: plans.into_iter().map(|plan| RuleId(plan.into())).collect(),
            },
            CoreError::Provider { provider, code } => Self::Provider {
                provider: ProviderName(provider.into()),
                code: match code {
                    CoreCode::SpanOutsideFile => ProviderCode::SpanOutsideFile,
                    CoreCode::NodeOutsideTable => ProviderCode::NodeOutsideTable,
                    CoreCode::NodeOfAnotherFile => ProviderCode::NodeOfAnotherFile,
                    CoreCode::FileNotVisited => ProviderCode::FileNotVisited,
                },
            },
            CoreError::Corpus { reason } => io(Port::Corpus, reason),
            CoreError::Store { reason } => io(Port::Store, reason),
            CoreError::Facts { reason } => io(Port::Facts, reason),
            CoreError::Language { reason } => io(Port::Language, reason),
            CoreError::Memory { reason } => io(Port::Memory, reason),
            CoreError::Scheduler { reason } => io(Port::Scheduler, reason),
            CoreError::Output { reason } => io(Port::Output, reason),
            CoreError::OutputClosed => {
                io(Port::Output, "whoever reads the answer stopped".to_owned())
            }
            // Ids are unique and not blank by construction: these cannot happen, and say so if they do.
            other @ (CoreError::BlankRuleId | CoreError::DuplicateRuleId { .. }) => {
                Self::Unrunnable {
                    id: RuleId(Box::default()),
                    reason: other.to_string(),
                }
            }
        }
    }

    /// The caller's failure for a core failure of opening.
    pub(super) fn from_open(error: ryo_core::inbound::use_cases::open::errors::OpenError) -> Self {
        use ryo_core::inbound::use_cases::open::errors::OpenError;
        let (what, reason) = match error {
            OpenError::Language { reason } => (Port::Language, reason),
            OpenError::Memory { reason } => (Port::Memory, reason),
        };
        Self::Io { what, reason }
    }
}
