//! The shape of the data this entry does not control: each wire shape is a `<Name>Schema`, its checked conversion into the core's type is an `impl TryFrom<<Name>Schema>`, and each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::lint::errors::LintError as CoreError;
use ryo_core::inbound::use_cases::lint::types::{
    LintReport as CoreReport, LintRequest as CoreRequest, Rule,
};

use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::records::types::Unreadable as CoreUnreadable;
use ryo_values::domain::row_texts::types::Cell as RowCell;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::runs::types::{AnswerShape, Budget, MemoryBytes, Session};

use super::errors::RunError;
use super::types::{Heading, Row, RowCount, RunReport, RunRequest};
use crate::domain::capacity::types::MemoryBudget;
use crate::domain::cell::types::{Cell, HostNumber, HostText, Slot};
use crate::domain::file::types::{ReadFailure, RelativePath, Unread, Unreadable};

/// The id the query has as the one rule of the lint it runs as.
const QUERY_ID: &str = "query";

/// A request of the caller's, offered to the conversion.
struct RunRequestSchema(RunRequest);

/// The most memory the caller lets the run use, as the memory factory takes it.
pub(super) const fn budget(memory: MemoryBudget) -> Budget {
    match memory {
        MemoryBudget::Bytes(bytes) => Budget::Bytes(MemoryBytes(bytes.get())),
        MemoryBudget::Machine => Budget::Machine,
    }
}

/// A core failure, offered to the conversion.
struct RunErrorSchema(CoreError);

/// What a run answered and the rows it wrote, offered to the conversion.
struct RunReportSchema {
    /// Where the rows the run wrote were kept.
    collector: ryo_core::outbound::row_sink::types::Collector,
    /// What the run answered.
    report: CoreReport,
}

impl TryFrom<RunRequestSchema> for CoreRequest {
    type Error = RunError;

    fn try_from(schema: RunRequestSchema) -> Result<Self, Self::Error> {
        let request = schema.0;
        let root = request
            .root
            .to_str()
            .ok_or_else(|| RunError::PathNotText {
                path: request.root.clone(),
            })?
            .into();
        Ok(Self {
            session: Session {
                repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
                root: RootPath(root),
                search_scope: Default::default(),
                given_inventory: Default::default(),
            },
            files: Vec::new(),
            source_language: None,
            rules: vec![Rule {
                observations: Vec::new(),
                id: ryo_values::domain::runs::types::RuleId(QUERY_ID.into()),
                query: request.query.into_bundle(),
                applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
                parameters: Vec::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            }],
            hosted: Vec::new(),
            answer: AnswerShape::Rows(RowTotal(request.limit.0)),
        })
    }
}

impl From<RunReportSchema> for RunReport {
    fn from(schema: RunReportSchema) -> Self {
        let RunReportSchema { collector, report } = schema;
        let collected = std::mem::take(
            &mut *collector
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        Self {
            columns: collected
                .columns
                .into_iter()
                .map(|each| Heading(each.0))
                .collect(),
            rows: collected
                .rows
                .into_iter()
                .map(|each| Row(each.0.iter().map(|cell| cell_of(cell.as_ref())).collect()))
                .collect(),
            unread: report
                .unread
                .into_iter()
                .map(|each| Unread {
                    path: RelativePath(each.path.0),
                    reason: match each.reason {
                        CoreUnreadable::NoLanguage => Unreadable::NoLanguage,
                        CoreUnreadable::Failed(reason) => Unreadable::Failed(ReadFailure(reason.0)),
                    },
                })
                .collect(),
            total: RowCount(collected.summary.total.0),
            shown: RowCount(collected.summary.shown.0),
        }
    }
}

impl From<RunErrorSchema> for RunError {
    fn from(schema: RunErrorSchema) -> Self {
        match schema.0 {
            CoreError::ObservationBindings { reason } => Self::Query { reason },
            CoreError::ObservationExecution { reason } => Self::Evaluate { reason },
            CoreError::Query { error, .. } => Self::Query {
                reason: error.to_string(),
            },
            CoreError::UnknownKind { reason, .. } => Self::Query { reason },
            CoreError::MissingParameter { .. } => Self::Query {
                reason: "the query has no value for a parameter it declares".to_owned(),
            },
            CoreError::Rule { reason, .. } | CoreError::Execution { reason } => {
                Self::Evaluate { reason }
            }
            CoreError::Corpus { reason } => Self::Corpus { reason },
            CoreError::Facts { reason } => Self::Facts { reason },
            CoreError::Language { reason } => Self::Language { reason },
            error @ CoreError::SourceLanguage { .. } => Self::Language {
                reason: error.to_string(),
            },
            // A language that wrote what its file cannot hold, a store that a run keeping none never reads, and the id the query
            // is given, which is never blank nor another's, are faults of the languages or of ryo: the language is what is told.
            error @ (CoreError::Provider { .. }
            | CoreError::Store { .. }
            | CoreError::BlankRuleId
            | CoreError::DuplicateRuleId { .. }) => Self::Language {
                reason: error.to_string(),
            },
            CoreError::Memory { reason } => Self::Memory { reason },
            CoreError::TooLarge { size, budget } => Self::TooLarge { size, budget },
            CoreError::MemoryBudgetExceeded { used, budget, .. } => {
                Self::MemoryBudgetExceeded { used, budget }
            }
            CoreError::Scheduler { reason } => Self::Scheduler { reason },
            CoreError::Output { reason } => Self::Output { reason },
            CoreError::OutputClosed => Self::OutputClosed,
        }
    }
}

/// The caller's cell for a typed value of a row, as the core holds it: none for a value that reads as nothing.
fn cell_of(cell: Option<&RowCell>) -> Slot {
    cell.map_or(Slot::Nothing, |cell| {
        Slot::Is(match cell {
            RowCell::File(path) => Cell::File(RelativePath(path.clone())),
            RowCell::Text(text) => Cell::Text(HostText(text.clone())),
            RowCell::Number(number) => Cell::Number(HostNumber(number.0)),
            RowCell::Span(span) => Cell::Span(*span),
        })
    })
}

/// The core's request for the caller's.
///
/// # Errors
///
/// `PathNotText` for a root that is not text.
pub(super) fn request(request: RunRequest) -> Result<CoreRequest, RunError> {
    RunRequestSchema(request).try_into()
}

/// The caller's report for the rows a run wrote and what it answered.
pub(super) fn report(
    collector: ryo_core::outbound::row_sink::types::Collector,
    report: CoreReport,
) -> RunReport {
    RunReportSchema { collector, report }.into()
}

/// The caller's failure for the core's.
pub(super) fn error(error: CoreError) -> RunError {
    RunErrorSchema(error).into()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
