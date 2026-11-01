//! The shape of the data this command does not control: its arguments become the service's request, and
//! the service's end becomes what the command reports. Each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::lint::errors::LintError;
use ryo_core::inbound::use_cases::lint::types::{LintRequest, Rule};
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::records::types::RowTotal;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::{AnswerShape, RuleApplicability};
use ryo_values::domain::runs::types::{Budget, MemoryBytes, Session};

/// Why the arguments were refused, or the queries could not be answered.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RunSchemaError {
    /// A query text is empty.
    #[error("the query text is empty")]
    BlankText,
    /// The one query could not be answered: what is wrong with it, said as the query's own.
    #[error("{}", alone(.0))]
    Failed(LintError),
    /// One of several queries could not be answered: what is wrong, naming the query by its place among them.
    #[error("{0}")]
    FailedAmong(LintError),
}

/// What is wrong with the one query a run had, without the name of the rule it ran as.
fn alone(error: &LintError) -> String {
    match error {
        LintError::Query { error, .. } => error.to_string(),
        LintError::Rule { reason, .. } | LintError::UnknownKind { reason, .. } => reason.clone(),
        LintError::MissingParameter { .. } => {
            "the query has no value for a parameter it declares".to_owned()
        }
        other @ (LintError::BlankRuleId
        | LintError::DuplicateRuleId { .. }
        | LintError::Corpus { .. }
        | LintError::Store { .. }
        | LintError::Facts { .. }
        | LintError::Provider { .. }
        | LintError::Language { .. }
        | LintError::Memory { .. }
        | LintError::TooLarge { .. }
        | LintError::MemoryBudgetExceeded { .. }
        | LintError::Scheduler { .. }
        | LintError::Output { .. }
        | LintError::OutputClosed) => other.to_string(),
    }
}

/// The arguments as the command line gives them, offered to the conversion.
struct RunArgumentsSchema {
    /// The repository's root directory.
    root: String,
    /// The RyoQL text to run. Give it more than once to run several queries over one read of the repository: the answer then starts
    /// with a `rule` column, the query's place among them from 1, and has every query's columns.
    text: Vec<String>,
    /// How many rows are shown; every row is counted.
    limit: u64,
}

/// The most memory the arguments let the run use, as the memory factory takes it: none means the machine's.
pub(super) const fn budget(bytes: u64) -> Budget {
    match bytes {
        0 => Budget::Machine,
        bytes => Budget::Bytes(MemoryBytes(bytes)),
    }
}

impl TryFrom<RunArgumentsSchema> for LintRequest {
    type Error = RunSchemaError;

    fn try_from(schema: RunArgumentsSchema) -> Result<Self, Self::Error> {
        if schema.text.iter().any(|text| text.trim().is_empty()) {
            return Err(RunSchemaError::BlankText);
        }
        Ok(Self {
            session: Session {
                repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
                root: RootPath(schema.root.into()),
            },
            files: Vec::new(),
            // Each query is a rule over every file, named by its place among the queries.
            rules: schema
                .text
                .into_iter()
                .enumerate()
                .map(|(place, text)| Rule {
                    id: RuleId((place + 1).to_string().into()),
                    query: ryo_values::domain::program::types::QueryBundle::from(QueryText(
                        text.into(),
                    )),
                    applicability: RuleApplicability::Always,
                    parameters: Vec::new(),
                    include: Vec::new(),
                    exclude: Vec::new(),
                })
                .collect(),
            hosted: Vec::new(),
            answer: AnswerShape::Rows(RowTotal(schema.limit)),
        })
    }
}

/// Where the run's store is kept and the build that keeps it: the edge's cache directory beneath the project's `root`, and the
/// edge's build.
pub(super) fn store(
    edge: &crate::edge::Edge,
    root: &RootPath,
) -> ryo_core::outbound::lint_store::types::Store {
    ryo_core::outbound::lint_store::types::Store {
        // Joined by `/` on every platform: the directory is the same place written the same way everywhere.
        directory: ryo_core::outbound::lint_store::types::CacheDirectory(
            [&*root.0, STORE_SEPARATOR, &*edge.store.0].concat().into(),
        ),
        build: edge.build.clone(),
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `BlankText` for a query text that is empty.
pub(super) fn request(
    root: String,
    text: Vec<String>,
    limit: u64,
) -> Result<LintRequest, RunSchemaError> {
    RunArgumentsSchema { root, text, limit }.try_into()
}

/// What the command says of the failure of a run of the queries `rules` stand for: the one query's own, or naming which of several.
pub(super) fn failure(error: LintError, rules: &[Rule]) -> RunSchemaError {
    if let [_] = rules {
        RunSchemaError::Failed(error)
    } else {
        RunSchemaError::FailedAmong(error)
    }
}

/// What a run says of the stream it writes its answer to refusing the answer: a reader that closed it, or any other failure of the
/// write.
pub(super) fn unwritten(error: &std::io::Error) -> LintError {
    if error.kind() == std::io::ErrorKind::BrokenPipe {
        LintError::OutputClosed
    } else {
        LintError::Output {
            reason: error.to_string(),
        }
    }
}

/// How the run ended: a reader that closed the stream took what it wanted, so that end is not an error.
///
/// # Errors
///
/// The run's failure, unless it is that the output was closed.
pub(super) fn delivered(ended: Result<(), RunSchemaError>) -> Result<(), RunSchemaError> {
    match ended {
        Err(
            RunSchemaError::Failed(LintError::OutputClosed)
            | RunSchemaError::FailedAmong(LintError::OutputClosed),
        ) => Ok(()),
        other => other,
    }
}

/// What the user does next about a refusal of the arguments.
impl crate::outcome::remedy::Remedy for RunSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::BlankText => {
                crate::outcome::remedy::Next::Fix("pass the query with `--text`".to_owned())
            }
            Self::Failed(error) | Self::FailedAmong(error) => {
                crate::outcome::remedy::Remedy::next(error)
            }
        }
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// What joins the project's root and the store's directory beneath it, on every platform.
const STORE_SEPARATOR: &str = "/";
