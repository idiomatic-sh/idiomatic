//! The `lint` use case's failures.

use super::types::ProviderFault;
use crate::domain::budget::errors::BudgetError;
use crate::domain::visit::errors::VisitError;
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::language::errors::LanguageError;
use crate::outbound::lint_store::errors::LintStoreError;
use crate::outbound::memory::errors::MemoryError;
use crate::outbound::rows::errors::RowsError;
use crate::outbound::scheduler::errors::SchedulerError;
use ryo_execution::inbound::use_cases::execution::types::facts::FactsError;
use ryo_values::domain::plan_parts::types::RefusedQuery;
use ryo_values::domain::rule_requests::errors::RulesError;

/// Why `lint` failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LintError {
    /// Required application execution dependencies were inconsistent.
    #[error("execution: {reason}")]
    Execution {
        /// Why the execution scope could not be established.
        reason: String,
    },
    /// A required observation publication could not be admitted during execution.
    #[error("observation execution failed: {reason}")]
    ObservationExecution { reason: String },
    /// The explicitly declared named-output dependency contract is invalid.
    #[error("invalid observation bindings: {reason}")]
    ObservationBindings {
        /// Configuration failure before file execution.
        reason: String,
    },
    /// An explicitly selected source provider is not linked at this request edge.
    #[error("explicit source provider `{name}` must name exactly one linked provider")]
    SourceLanguage {
        /// The exact provider name supplied by the caller.
        name: String,
    },
    /// A rule's query is wrong, or cannot be run.
    #[error("rule `{id}`: {reason}")]
    Rule {
        /// The rule's id: text that came from the caller.
        id: String,
        /// Why.
        reason: String,
    },
    /// A rule's query compares the kind of a node or a token with a word that names no kind of any linked language.
    #[error("rule `{id}`: {reason}")]
    UnknownKind {
        /// The rule's id: text that came from the caller.
        id: String,
        /// The word and the languages it names no kind of.
        reason: String,
    },
    /// A rule's query is not correct RyoQL.
    #[error("rule `{id}`: {error}")]
    Query {
        /// The rule's id: text that came from the caller.
        id: String,
        /// What is wrong with the text, and where.
        error: RefusedQuery,
    },
    /// A rule's query has a parameter nothing was bound to.
    #[error("rule `{id}` has no value for a parameter it declares")]
    MissingParameter {
        /// The rule's id.
        id: String,
    },
    /// A rule has no id, so no finding could name it.
    #[error("a rule has a blank id")]
    BlankRuleId,
    /// Two rules have the same id, so a finding could not say which gave it.
    #[error("two rules are named `{id}`")]
    DuplicateRuleId {
        /// The id both have.
        id: String,
    },
    /// The repository could not be listed.
    #[error("corpus: {reason}")]
    Corpus {
        /// What the corpus port said.
        reason: String,
    },
    /// The store a run keeps could not be read or written.
    #[error("store: {reason}")]
    Store {
        /// What the store port said.
        reason: String,
    },
    /// The fact store failed.
    #[error("facts: {reason}")]
    Facts {
        /// What the facts port said.
        reason: String,
    },
    /// A language wrote output for a file that the file cannot hold.
    #[error("language `{}` wrote output its file cannot hold: {code}", provider)]
    Provider {
        /// The language: text that came from the program that linked it.
        provider: String,
        /// What was wrong.
        code: ProviderFault,
    },
    /// The relation catalog or the manifests could not be read.
    #[error("language: {reason}")]
    Language {
        /// What the language port said.
        reason: String,
    },
    /// Memory could not be read.
    #[error("memory: {reason}")]
    Memory {
        /// Why.
        reason: String,
    },
    /// The repository does not fit the memory budget.
    #[error("the repository is {size} bytes and the budget is {budget}")]
    TooLarge {
        /// The bytes of the repository.
        size: u64,
        /// The bytes the run may use.
        budget: u64,
    },
    /// The process holds more memory than the budget while it runs: refused before the machine runs out.
    #[error("the run holds {used} bytes and the budget is {budget}, in the rules {}", plans.join(", "))]
    MemoryBudgetExceeded {
        /// The bytes the process holds.
        used: u64,
        /// The bytes the run may use.
        budget: u64,
        /// The rules the run was working for: the ones that govern the file, or the one whose finish it was making.
        plans: Vec<String>,
    },
    /// The workers could not be started.
    #[error("scheduler: {reason}")]
    Scheduler {
        /// What the scheduler port said.
        reason: String,
    },
    /// The answer could not be written.
    #[error("output: {reason}")]
    Output {
        /// What the rows port said.
        reason: String,
    },
    /// Whoever reads the answer stopped reading, so it was not finished.
    #[error("whoever reads the answer stopped reading")]
    OutputClosed,
}

impl From<RulesError> for LintError {
    fn from(error: RulesError) -> Self {
        match error {
            RulesError::BlankRuleId => Self::BlankRuleId,
            RulesError::DuplicateRuleId { id } => Self::DuplicateRuleId {
                id: id.0.to_string(),
            },
            RulesError::MissingParameter { id } => Self::MissingParameter {
                id: id.0.to_string(),
            },
            RulesError::Failed { id, reason } => Self::Rule {
                id: id.0.to_string(),
                reason,
            },
            RulesError::SemanticIncomplete { id, reason }
            | RulesError::ObservationIncomplete { id, reason } => Self::Rule {
                id: id.0.to_string(),
                reason,
            },
        }
    }
}

impl From<BudgetError> for LintError {
    fn from(error: BudgetError) -> Self {
        match error {
            BudgetError::TooLarge { size, budget } => Self::TooLarge {
                size: size.0,
                budget: budget.0,
            },
            BudgetError::Exceeded {
                used,
                budget,
                plans,
            } => Self::MemoryBudgetExceeded {
                used: used.0,
                budget: budget.0,
                plans: plans.into_iter().map(|id| id.0.to_string()).collect(),
            },
        }
    }
}

impl From<VisitError> for LintError {
    fn from(error: VisitError) -> Self {
        match error {
            VisitError::UnknownRelation { relation } => Self::Language {
                reason: format!("the catalog does not list {relation:?}"),
            },
        }
    }
}

impl From<FactsError> for LintError {
    fn from(error: FactsError) -> Self {
        Self::Facts {
            reason: error.to_string(),
        }
    }
}

impl From<LintStoreError> for LintError {
    fn from(error: LintStoreError) -> Self {
        Self::Store {
            reason: error.to_string(),
        }
    }
}

impl From<CorpusError> for LintError {
    fn from(error: CorpusError) -> Self {
        Self::Corpus {
            reason: error.to_string(),
        }
    }
}

impl From<MemoryError> for LintError {
    fn from(error: MemoryError) -> Self {
        Self::Memory {
            reason: error.to_string(),
        }
    }
}

impl From<SchedulerError> for LintError {
    fn from(error: SchedulerError) -> Self {
        Self::Scheduler {
            reason: error.to_string(),
        }
    }
}

impl From<LanguageError> for LintError {
    fn from(error: LanguageError) -> Self {
        Self::Language {
            reason: error.to_string(),
        }
    }
}

impl From<RowsError> for LintError {
    fn from(error: RowsError) -> Self {
        match error {
            RowsError::Closed => Self::OutputClosed,
            RowsError::Failed { reason } => Self::Output { reason },
        }
    }
}

impl From<crate::application::file_analysis::errors::FileAnalysisError> for LintError {
    fn from(error: crate::application::file_analysis::errors::FileAnalysisError) -> Self {
        use crate::application::file_analysis::errors::FileAnalysisError;
        match error {
            FileAnalysisError::Execution(error) => Self::Execution {
                reason: error.to_string(),
            },
            FileAnalysisError::ProjectObservationMismatch => Self::Language {
                reason: "project entry observation receipt did not match the engine batch"
                    .to_owned(),
            },
            FileAnalysisError::SourceLanguage { name } => Self::SourceLanguage { name },
            FileAnalysisError::Store(error) => error.into(),
            FileAnalysisError::Corpus(error) => error.into(),
            FileAnalysisError::Provider { provider, code } => Self::Provider {
                provider,
                code: code.into(),
            },
            FileAnalysisError::Facts(error) => error.into(),
            FileAnalysisError::Rules(error) => error.into(),
            FileAnalysisError::Budget(error) => error.into(),
            FileAnalysisError::Memory(error) => error.into(),
            FileAnalysisError::Language(error) => error.into(),
        }
    }
}

/// What was wrong with a language's output, as the `Provider` failure shows it: a value the failure carries, not a failure of its own.
impl std::fmt::Display for ProviderFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ProviderFault::SpanOutsideFile => "a span is not within its file",
            ProviderFault::NodeOutsideTable => "a node is not in its file's table",
            ProviderFault::NodeOfAnotherFile => "a node is of another file",
            ProviderFault::FileNotVisited => "a cell names a file that is not the one visited",
        })
    }
}

impl From<ryo_execution::inbound::use_cases::execution::types::RuleCompilationError> for LintError {
    fn from(
        error: ryo_execution::inbound::use_cases::execution::types::RuleCompilationError,
    ) -> Self {
        match error {
            ryo_execution::inbound::use_cases::execution::types::RuleCompilationError::Observations(
                reason,
            ) => Self::ObservationBindings {
                reason: reason.to_string(),
            },
            ryo_execution::inbound::use_cases::execution::types::RuleCompilationError::Rules(reason) => {
                reason.into()
            }
            ryo_execution::inbound::use_cases::execution::types::RuleCompilationError::Query {
                id,
                error,
            } => Self::Query { id, error },
        }
    }
}

impl From<crate::application::cross_file::errors::CrossError> for LintError {
    fn from(error: crate::application::cross_file::errors::CrossError) -> Self {
        use crate::application::cross_file::errors::CrossError;
        match error {
            CrossError::Operating(error) => Self::ObservationExecution {
                reason: error.to_string(),
            },
            CrossError::Rules(error) => error.into(),
            CrossError::Facts(error) => error.into(),
            CrossError::Scheduler(error) => error.into(),
        }
    }
}
