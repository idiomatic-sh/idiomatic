//! The support of the `frontiers` cases.
use crate::application::file_dag::frontiers::types::PreparedFrontiers;
use crate::application::file_dag::program::project_phase::service::ProjectPhase;
use crate::application::file_dag::program::project_phase::types::{
    PhaseScope, ProjectInput, ProjectTask,
};
use crate::application::file_dag::program::rule_outcome::types::{
    ProjectBatch, ProjectOutputOutcome, RuleOutcome,
};
use ryo_execution_domain::domain::coverage::types::{Partials, RuleIndex};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{EmitDemand, ObservationDemand, RuleDemand};
use ryo_language::domain::symbols::types::Symbols;

/// The one root `rule` evaluated in `phase` over `scope` and `ordinary`, with `demand`.
pub(super) fn evaluate_one(
    phase: &ProjectPhase<'_>,
    rule: RuleIndex,
    demand: RuleDemand,
    scope: &PhaseScope,
    ordinary: PreparedFrontiers,
) -> Result<RuleOutcome<ProjectOutputOutcome>, OperatingError> {
    let partials = Partials::default();
    let batch = phase.evaluate(
        vec![ProjectTask {
            rule,
            demand,
            input: ProjectInput {
                scope,
                evaluation: &partials,
                publication: &partials,
                ordinary,
                bound: &[],
                arenas: &[],
            },
        }],
        &mut Symbols::default(),
        |_| Ok::<(), OperatingError>(()),
    );
    match batch {
        ProjectBatch::Complete(mut done) => done.remove(0),
        ProjectBatch::Refused { error, .. } => Err(error),
    }
}

pub(super) const DIAGNOSTIC: RuleDemand = RuleDemand {
    emit: EmitDemand::Enabled,
    observations: ObservationDemand::Required,
};
