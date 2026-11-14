//! The `file_execution` application operation.

use crate::application::file_dag::file_pass::constants::PRODUCER_COVERAGE_INCOMPLETE;
use crate::application::file_dag::program::facade_runtime::source_admission::service::table_of;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::application::file_dag::program::runtime::service::InvocationOutcome;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
use ryo_relations::domain::relations::types::Relation;
use std::rc::Rc;

pub(crate) fn outcome_of<'facts>(
    result: Result<InvocationOutcome, OperatingError>,
    owner: &Rc<RelationOwner>,
    read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
) -> RootOutcome {
    match result {
        Ok(InvocationOutcome::Published(input)) => match table_of(owner, &input, read) {
            Ok(table) => match input.coverage() {
                ryo_execution_domain::domain::completion::types::CoverageState::Complete => {
                    RootOutcome::Complete(table)
                }
                coverage @ (ryo_execution_domain::domain::completion::types::CoverageState::Partial { .. }
                | ryo_execution_domain::domain::completion::types::CoverageState::Inapplicable {
                    ..
                }) => RootOutcome::Partial {
                    table,
                    affected: ryo_execution_domain::domain::frontier_coverage::service::affected_files(coverage)
                        .unwrap_or_default(),
                },
            },
            Err(error) => classify(error),
        },
        Ok(InvocationOutcome::Ready | InvocationOutcome::Suspended(_)) => {
            RootOutcome::Failed(OperatingError::NotRun)
        }
        Err(error) => classify(error),
    }
}

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
