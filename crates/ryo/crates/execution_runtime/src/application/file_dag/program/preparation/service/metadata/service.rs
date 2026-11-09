//! Per-rule root, read and frontier metadata taken from the compiled plans.
use crate::application::file_dag::program::preparation::types::SemanticNeed;
use crate::application::file_dag::program::preparation::types::{
    ObservationMetadata, RootMetadata, RuleMetadata,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::types::{
    InputSource, executable_program::value::ExecutableProgram,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::trust::types::RelationRead;
use ryo_values::domain::plan_parts::types::{Input, StepId};
use ryo_values::domain::plans::types::{Operation, Plan};

/// One entry per plan, in rule order.
pub(crate) fn rules(
    program: &ExecutableProgram,
    named: Option<&ObservationMetadata>,
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
                    source: match &refusal {
                        Some(error) => Err(error.clone()),
                        None => source(program, named, rule, output.source),
                    },
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
            emit_root: match &refusal {
                Some(error) => Err(error.clone()),
                None => program
                    .rules()
                    .get(rule.0)
                    .ok_or(OperatingError::NotRun)?
                    .emit
                    .clone(),
            },
            frontiers: plan.frontier.clone().into_boxed_slice(),
            observations: outputs.into_boxed_slice(),
            observation_frontiers: observation_frontiers
                .get(rule.0)
                .ok_or(OperatingError::NotRun)?
                .clone()
                .into_boxed_slice(),
            frontier_dependencies:
                ryo_execution_domain::domain::observation_schedule::service::link::frontier_dependencies(
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
fn source(
    program: &ExecutableProgram,
    observations: Option<&ObservationMetadata>,
    rule: RuleIndex,
    source: Input,
) -> Result<InputSource, OperatingError> {
    match source {
        Input::Relation(relation) => Ok(InputSource::Relation(relation)),
        Input::Step(StepId(step)) => {
            let origin = program
                .rules()
                .get(rule.0)
                .and_then(|rule| rule.original_steps.get(step as usize))
                .ok_or(OperatingError::NotRun)?
                .as_ref()
                .map_err(Clone::clone)?;
            program
                .logical_regions()
                .get(origin.logical.0)
                .copied()
                .map(InputSource::Region)
                .ok_or(OperatingError::NotRun)
        }
        Input::BoundObservation(input) => observations
            .and_then(|observations| {
                observations
                    .slots
                    .iter()
                    .position(|slot| slot.rule == rule && slot.input == input)
            })
            .map(|slot| {
                InputSource::Observation(
                    ryo_execution_domain::domain::executable_program::types::ObservationPlace(slot),
                )
            })
            .ok_or(OperatingError::NotRun),
    }
}
