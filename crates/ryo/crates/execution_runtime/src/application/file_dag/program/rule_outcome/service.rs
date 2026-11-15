//! The `rule_outcome` service: a root's readiness for the named outputs a file records, and its outcome when it raised an error.

use crate::application::file_dag::file_pass::constants::PRODUCER_COVERAGE_INCOMPLETE;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::operating::errors::OperatingError;

/// How ready a named output is after its root answered `outcome`; a root deferred past the file says nothing yet.
pub(crate) fn readiness_of(
    outcome: &RootOutcome,
) -> Option<ryo_values::domain::records::types::ProducerReadiness> {
    use ryo_values::domain::records::types::ObservationReason;
    use ryo_values::domain::records::types::ProducerReadiness;
    Some(match outcome {
        RootOutcome::Complete(_) => ProducerReadiness::Complete,
        RootOutcome::Partial { .. } => {
            ProducerReadiness::Unknown(ObservationReason(PRODUCER_COVERAGE_INCOMPLETE.into()))
        }
        RootOutcome::Skipped(_) => ProducerReadiness::Skipped,
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => {
            ProducerReadiness::Unknown(ObservationReason(error.to_string().into()))
        }
        RootOutcome::NoQuery => ProducerReadiness::NoQuery,
        RootOutcome::Deferred { .. } => return None,
    })
}

/// The answer of a root that raised `error`: unknown when its inputs were incomplete, failed otherwise.
pub(crate) fn classify(error: OperatingError) -> RootOutcome {
    match error {
        OperatingError::ObservationIncomplete { .. }
        | OperatingError::SemanticIncomplete { .. } => RootOutcome::Unknown(error),
        other => RootOutcome::Failed(other),
    }
}
