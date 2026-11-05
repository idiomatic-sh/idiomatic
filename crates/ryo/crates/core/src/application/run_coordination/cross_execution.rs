//! Project root delivery through the same prepared executable invocation.
use crate::domain::crossing::types::{ObservedRoot, PhaseRoots};
use ryo_execution::inbound::use_cases::execution::interface::{ExecutionRequest, ProjectExecution};
use ryo_execution::inbound::use_cases::execution::types::{
    EmitDemand, ObservationDemand, ObservedPublication, PreparedFrontiers, ProjectInput,
    ProjectObservation, ProjectScope, RequestExecution, RootOutcome, RuleDemand,
};
use ryo_values::domain::observations::types::{ObservationReason, ProducerReadiness};

/// Pure phase rows plus the actual admitted owner proof of each Complete named root.
pub(crate) struct ProgramEvaluated {
    pub roots: PhaseRoots,
    pub fed: Vec<ryo_execution::inbound::use_cases::execution::types::RowsFed>,
    pub publications: Vec<(
        ryo_values::domain::observations::types::ObservationIndex,
        ObservedPublication,
    )>,
}

pub(crate) fn evaluate_cross_roots(
    request: &RequestExecution,
    rule: RuleIndex,
    scope: &ProjectScope,
    evaluation: &ryo_execution::inbound::use_cases::metadata::interface::Partials,
    publication: &ryo_execution::inbound::use_cases::metadata::interface::Partials,
    ordinary: PreparedFrontiers,
    bound: &[ryo_execution::inbound::use_cases::execution::types::BoundInput],
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    arenas: &[ryo_values::domain::stored_facts::types::ArenaBytes],
) -> Result<ProgramEvaluated, OperatingError> {
    let mut run = request.open_project(
        ProjectInput {
            scope,
            evaluation,
            publication,
            ordinary,
            bound,
            arenas,
        },
        symbols,
    )?;
    let outcome = run.rule(
        rule,
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::Required,
        },
        symbols,
    )?;
    run.finish();
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
    for (output, outcome) in outcome.observations {
        let (table, readiness) = match outcome {
            ProjectObservation::Complete { table, publication } => {
                publications.push((output, publication));
                (Some(table), ProducerReadiness::Complete)
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
                    ProducerReadiness::Unknown(ObservationReason(error.to_string().into())),
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

use ryo_execution::inbound::use_cases::execution::types::{OperatingError, RuleIndex};
