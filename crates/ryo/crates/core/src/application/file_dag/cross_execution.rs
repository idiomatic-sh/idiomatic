//! Project root delivery through the same prepared executable invocation.
use super::program::facade::{
    self, EmitDemand, ObservationDemand, PreparedProgram, ProgramServices, ProjectObservation,
    RootOutcome, RuleDemand,
};
use crate::domain::{
    coverage::types::RuleIndex,
    crossing::types::{ObservedRoot, PhaseRoots},
    operating::{errors::OperatingError, types::OperatorContext},
};
use ryo_values::domain::observations::types::{ObservationReason, ProducerReadiness};
use std::sync::Arc;

/// Pure phase rows plus the actual admitted owner proof of each Complete named root.
pub(crate) struct ProgramEvaluated {
    pub roots: PhaseRoots,
    pub fed: Vec<crate::domain::operator::types::RowsFed>,
    pub publications: Vec<(
        ryo_values::domain::observations::types::ObservationIndex,
        facade::ObservedPublication,
    )>,
}

pub(crate) fn evaluate_cross_roots(
    program: &Arc<PreparedProgram>,
    services: &ProgramServices<'_>,
    rule: RuleIndex,
    required: facade::RequiredPartitions,
    partials: facade::ScopePartials<'_>,
    frontiers: facade::ProjectFrontiers<'_>,
    symbols: &mut ryo_language::domain::symbols::types::Symbols,
    paths: &[ryo_language::domain::files::types::FilePath],
    arenas: &[ryo_values::domain::stored_facts::types::ArenaBytes],
) -> Result<ProgramEvaluated, OperatingError> {
    let mut run = facade::open_project(program, required, partials, frontiers, symbols, services)?;
    let mut memo = Vec::new();
    let mut scratch = crate::domain::table_scratch::types::OperatorScratch::default();
    let mut context = OperatorContext {
        semantic: None,
        prepared_kinds: None,
        prepared_matchers: Some(program.matchers().as_ref()),
        bindings: program.bindings(rule)?,
        symbols: symbols.into(),
        source: crate::domain::table_texts::types::SourceView(&[]),
        paths,
        arenas,
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let outcome = run.rule(
        rule,
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::Required,
        },
        &mut context,
    )?;
    let mut roots = PhaseRoots::default();
    match outcome.emit {
        RootOutcome::Complete(table) => roots.emit = Some(table),
        RootOutcome::Partial { table, affected } => {
            roots.emit = Some(table);
            roots.emit_affected = Some(affected);
        }
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => roots.emit_error = Some(error),
        RootOutcome::Skipped(_) | RootOutcome::NoQuery => {}
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
            ProjectObservation::Other(RootOutcome::Complete(_)) => {
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
