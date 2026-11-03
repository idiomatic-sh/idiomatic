//! Failures of file and project analysis.
use crate::domain::admission::errors::AdmissionError;
use crate::domain::budget::errors::BudgetError;
use crate::domain::rules::errors::RulesError;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::language::errors::LanguageError;
use crate::outbound::memory::errors::MemoryError;
/// Why an analysis could not finish.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FileAnalysisError {
    /// Required execution scope dependencies were inconsistent.
    #[error(transparent)]
    Execution(#[from] crate::domain::operating::errors::OperatingError),
    /// The request's selected provider is missing or ambiguous.
    #[error("explicit source provider `{name}` must name exactly one linked provider")]
    SourceLanguage {
        /// The exact caller-supplied name.
        name: String,
    },
    /// Required normalized-frontier staging or replay failed.
    #[error(transparent)]
    Store(#[from] crate::outbound::lint_store::errors::LintStoreError),
    /// A required project dependency could not be recorded.
    #[error(transparent)]
    Corpus(#[from] crate::outbound::corpus::errors::CorpusError),
    /// The language wrote invalid facts.
    #[error("language `{provider}` wrote output its file cannot hold: {code}")]
    Provider {
        /// The linked language.
        provider: String,
        /// The invalid output.
        code: AdmissionError,
    },
    /// The fact store refused an operation.
    #[error(transparent)]
    Facts(#[from] FactsError),
    /// Evaluation refused a rule.
    #[error(transparent)]
    Rules(#[from] RulesError),
    /// The memory budget was exceeded.
    #[error(transparent)]
    Budget(#[from] BudgetError),
    /// The memory resource failed.
    #[error(transparent)]
    Memory(#[from] MemoryError),
    /// Project analysis failed.
    #[error(transparent)]
    Language(#[from] LanguageError),
}
