//! Project root delivery through the same prepared executable invocation.
use crate::domain::crossing::types::{ObservedRoot, PhaseRoots};
use ryo_execution::inbound::use_cases::execution::types::{
    ObservedPublication, ProjectObservation, RootOutcome,
};
use ryo_values::domain::records::types::ObservationReason;
use ryo_values::domain::records::types::ProducerReadiness;

/// Pure phase rows plus the actual admitted owner proof of each usable named root.
pub(crate) struct ProgramEvaluated {
    pub roots: PhaseRoots,
    pub fed: Vec<ryo_execution::inbound::use_cases::execution::types::RowsFed>,
    pub publications: Vec<(
        ryo_values::domain::records::types::ObservationIndex,
        ObservedPublication,
    )>,
}

pub(crate) fn collected(
    outcome: ryo_execution::inbound::use_cases::execution::types::ProjectRuleOutcome,
) -> Result<ProgramEvaluated, OperatingError> {
    let mut roots = PhaseRoots::default();
    match outcome.emit {
        RootOutcome::Complete(table) => roots.emit = Some(table),
        RootOutcome::Partial { table, affected } => {
            roots.emit = Some(table);
            roots.emit_affected = Some(affected);
        }
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => roots.emit_error = Some(error),
        RootOutcome::Skipped(_) | RootOutcome::NoQuery | RootOutcome::Deferred { .. } => {}
    }
    let mut publications = Vec::new();
    for ryo_execution::inbound::use_cases::execution::types::ProjectOutputOutcome {
        output,
        outcome,
    } in outcome.observations
    {
        let (table, readiness) = match outcome {
            ProjectObservation::Complete { table, publication } => {
                publications.push((output, publication));
                (Some(table), ProducerReadiness::Complete)
            }
            ProjectObservation::Partial { table, publication } => {
                publications.push((output, publication));
                (
                    Some(table),
                    ProducerReadiness::Unknown(ObservationReason(
                        "physical source facts are incomplete".into(),
                    )),
                )
            }
            ProjectObservation::Other(RootOutcome::Partial { .. }) => (
                None,
                ProducerReadiness::Unknown(ObservationReason(
                    "physical source facts are incomplete".into(),
                )),
            ),
            ProjectObservation::Other(RootOutcome::Unknown(error) | RootOutcome::Failed(error)) => {
                (
                    None,
                    ProducerReadiness::Unknown(ObservationReason(
                        format!("project phase named output {}: {error}", output.0).into(),
                    )),
                )
            }
            ProjectObservation::Other(RootOutcome::Skipped(_)) => {
                (None, ProducerReadiness::Skipped)
            }
            ProjectObservation::Other(RootOutcome::NoQuery) => (None, ProducerReadiness::NoQuery),
            ProjectObservation::Other(RootOutcome::Complete(_) | RootOutcome::Deferred { .. }) => {
                return Err(OperatingError::WrongOperation);
            }
        };
        roots.observations.push(ObservedRoot {
            output,
            table,
            readiness,
        });
    }
    Ok(ProgramEvaluated {
        roots,
        fed: outcome.counts.fed.into_vec(),
        publications,
    })
}

use ryo_execution::inbound::use_cases::execution::types::OperatingError;
