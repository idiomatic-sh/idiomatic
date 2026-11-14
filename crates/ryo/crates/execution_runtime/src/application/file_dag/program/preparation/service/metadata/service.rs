//! Per-rule root, read and frontier metadata taken from the compiled plans.
use crate::application::file_dag::program::preparation::types::{
    RootMetadata, RuleMetadata,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::interpreted_program::types::interpreted_program::value::InterpretedProgram;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plans::types::{Operation, Plan};

/// One entry per plan, in rule order.
pub(crate) fn rules(
    program: &InterpretedProgram,
    plans: &[Plan],
    observation_frontiers: &[Vec<FrontierPurpose>],
) -> Result<Vec<RuleMetadata>, OperatingError> {
    let mut metadata = Vec::with_capacity(plans.len());
    for (rule, plan) in plans.iter().enumerate() {
        let rule = RuleIndex(rule);
        // A rule with an errored origin is refused on its own roots; sibling rules prepare.
        let refusal = program
            .rules()
            .get(rule.0)
            .and_then(|compiled| {
                compiled
                    .original_steps
                    .iter()
                    .find_map(|origin| origin.as_ref().err())
            })
            .cloned();
        let reads = ryo_execution_domain::domain::trust::service::physical_reads(plan);
        let outputs = plan
            .observations
            .iter()
            .map(|output| {
                Ok(RootMetadata {
                    phase: output.scope,
                    reads: match output.source {
                        // A refused rule's named output reads nothing: it is refused on its root before any read.
                        Input::Step(step) => {
                            match reads.get(step.0 as usize).ok_or(OperatingError::NotRun)? {
                                Ok(reads) => reads.clone(),
                                Err(_) if refusal.is_some() => Vec::new(),
                                Err(error) => return Err(error.clone()),
                            }
                        }
                        Input::Relation(relation) => vec![RelationRead {
                            relation,
                            kinds: vec![],
                        }],
                        Input::BoundObservation(_) => vec![],
                    }
                    .into_boxed_slice(),
                })
            })
            .collect::<Result<Vec<_>, OperatingError>>()?;
        metadata.push(RuleMetadata {
            emit: plan.emit.clone(),
            frontiers: plan.frontier.clone().into_boxed_slice(),
            observations: outputs.into_boxed_slice(),
            observation_frontiers: observation_frontiers
                .get(rule.0)
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
        });
    }
    Ok(metadata)
}
