//! Per-rule root, read and frontier metadata taken from the compiled plans.
use crate::application::file_dag::file_pass::service::step_sets;
use crate::application::file_dag::file_pass::types::PassProgram;
use crate::application::file_dag::program::preparation::types::{RootPlan, RulePlan};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_execution_domain::domain::trust::types::read_set::value::ReadSet;
use ryo_execution_domain::domain::trust::types::read_sets::value::ReadSets;
use ryo_execution_domain::domain::trust::types::relation_read::value::RelationRead;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::{Operation, Plan};

/// What each plan states, in rule order: its emit, frontiers, named outputs with what each reads on the way (the file pass's
/// step reads, the one computation of them), its frontiers' purposes and whether it renders.
///
/// # Errors
///
/// `NotRun` when the purposes or the step reads hold no entry for a rule or a named output's step.
pub(crate) fn plans(
    plans: &[Plan],
    pass: &PassProgram,
    purposes: &[Vec<FrontierPurpose>],
) -> Result<Vec<RulePlan>, OperatingError> {
    // A root that is a relation itself reads it whole: its set is stored here, once however many roots read it.
    let mut sets = ReadSets::default();
    plans
        .iter()
        .enumerate()
        .map(|(rule, plan)| {
            if pass.reads.get(rule).is_none() {
                return Err(OperatingError::NotRun);
            }
            let roots = plan
                .observations
                .iter()
                .map(|output| {
                    Ok(RootPlan {
                        phase: output.scope,
                        reads: match output.source {
                            Input::Step(step) => step_sets(pass, RuleIndex(rule), step)
                                .ok_or(OperatingError::NotRun)?
                                .uses
                                .clone(),
                            Input::Relation(relation) => sets.shared(vec![RelationRead::whole(relation)]),
                            Input::BoundObservation(_) => ReadSet::default(),
                        },
                    })
                })
                .collect::<Result<Vec<_>, OperatingError>>()?;
            Ok(RulePlan {
                emit: plan.emit.clone(),
                frontiers: plan.frontier.clone().into_boxed_slice(),
                roots: roots.into_boxed_slice(),
                observation_frontiers: purposes
                    .get(rule)
                    .ok_or(OperatingError::NotRun)?
                    .clone()
                    .into_boxed_slice(),
                frontier_dependencies:
                    ryo_execution_domain::domain::observation_schedule::service::link::service::frontier_dependencies(
                        plan,
                    )
                    .map_err(|error| OperatingError::ObservationIncomplete {
                        reason: error.to_string(),
                    }),
                semantic_needed: if plan
                    .steps
                    .iter()
                    .any(|step| matches!(step.operation, Operation::RenderSyntax { .. }))
                {
                    SemanticNeed::Required
                } else {
                    SemanticNeed::NotRequired
                },
            })
        })
        .collect()
}
