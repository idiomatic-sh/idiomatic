//! Project root delivery through the same prepared executable invocation.
use super::program::facade::{
    self, EmitDemand, ObservationDemand, PreparedProgram, ProgramServices, RootOutcome, RuleDemand,
};
use crate::domain::{
    coverage::types::RuleIndex,
    crossing::types::{Evaluated, ObservedRoot, PhaseRoots},
    operating::{errors::OperatingError, types::OperatorContext},
};
use ryo_values::domain::observations::types::{ObservationReason, ProducerReadiness};
use std::sync::Arc;

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
) -> Result<Evaluated, OperatingError> {
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
        RootOutcome::Partial(table) => {
            roots.emit = Some(table);
            roots.emit_error = Some(OperatingError::ObservationIncomplete {
                reason: "physical source facts are incomplete".into(),
            });
        }
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => roots.emit_error = Some(error),
        RootOutcome::Skipped(_) | RootOutcome::NoQuery => {}
    }
    for (output, outcome) in outcome.observations {
        let (table, readiness) = match outcome {
            RootOutcome::Complete(table) => (Some(table), ProducerReadiness::Complete),
            RootOutcome::Partial(_) => (
                None,
                ProducerReadiness::Unknown(ObservationReason(
                    "physical source facts are incomplete".into(),
                )),
            ),
            RootOutcome::Unknown(error) | RootOutcome::Failed(error) => (
                None,
                ProducerReadiness::Unknown(ObservationReason(error.to_string().into())),
            ),
            RootOutcome::Skipped(_) => (None, ProducerReadiness::Skipped),
            RootOutcome::NoQuery => (None, ProducerReadiness::NoQuery),
        };
        roots.observations.push(ObservedRoot {
            output,
            table,
            readiness,
        });
    }
    Ok(Evaluated {
        roots,
        fed: outcome.counts.fed.into_vec(),
    })
}
