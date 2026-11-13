//! Why accepted differences are refused for a corpus.

/// Refused accepted differences.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CorpusDiscrepancyError {
    /// Refuses: accepted differences written for another corpus.
    #[error("accepted differences are for corpus {accepted}, the request reads corpus {actual}")]
    CorpusDiffers {
        /// The corpus digest the accepted differences name.
        accepted: String,
        /// The digest of the corpus the request reads.
        actual: String,
    },
    /// Refuses: a rule's accepted differences are written twice.
    #[error("accepted differences name {0} twice")]
    RuleNamedTwice(String),
    /// Refuses: a rule's accepted differences lack a reason or distinct findings.
    #[error("accepted differences for {0} need a reason and distinct missing or extra findings")]
    AcceptanceIncomplete(String),
}

impl From<CorpusDiscrepancyError>
    for ryo_benchmark_contracts::domain::reports::errors::ReportsError
{
    fn from(error: CorpusDiscrepancyError) -> Self {
        Self::Invalid(error.to_string())
    }
}
