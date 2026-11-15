//! Per-rule root, read and frontier metadata taken from the compiled plans.
use crate::application::file_dag::file_pass::types::NodeReads;
use crate::application::file_dag::program::preparation::types::{
    ObservationMetadata, RootMetadata, RootPlan, RuleMetadata, RulePlan,
};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::interpreted_program::types::interpreted_program::value::InterpretedProgram;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::FrontierPurpose;
use ryo_execution_domain::domain::rule_intent::types::SemanticNeed;
use ryo_execution_domain::domain::trust::types::RelationRead;
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
    reads: &[Box<[NodeReads]>],
    purposes: &[Vec<FrontierPurpose>],
) -> Result<Vec<RulePlan>, OperatingError> {
    plans
        .iter()
        .enumerate()
        .map(|(rule, plan)| {
            let steps = reads.get(rule).ok_or(OperatingError::NotRun)?;
            let roots = plan
                .observations
                .iter()
                .map(|output| {
                    Ok(RootPlan {
                        phase: output.scope,
                        reads: match output.source {
                            Input::Step(step) => steps
                                .get(usize::try_from(step.0).unwrap_or(usize::MAX))
                                .ok_or(OperatingError::NotRun)?
                                .uses
                                .clone(),
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

/// Each rule's interpreter regions, for the project phase: its emitted region and each named output's source, the rule's
/// authoring refusal (the file pass program's, the one computation of it) answered on each of them.
///
/// # Errors
///
/// `NotRun` for a rule the interpreter's program or the plans do not hold.
pub(crate) fn rules(
    program: &InterpretedProgram,
    plans: &[Plan],
    rule_plans: &[RulePlan],
    executable: &ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram,
) -> Result<Vec<RuleMetadata>, OperatingError> {
    let mut metadata = Vec::with_capacity(plans.len());
    for (rule, plan) in plans.iter().enumerate() {
        let rule = RuleIndex(rule);
        let stated = rule_plans.get(rule.0).ok_or(OperatingError::NotRun)?;
        // A rule with an authoring error is refused on its own roots; sibling rules prepare.
        let refusal =
            ryo_execution_domain::domain::executable_program::service::rule(executable, rule)?
                .emit
                .as_ref()
                .err()
                .cloned();
        let outputs = plan
            .observations
            .iter()
            .zip(stated.roots.iter())
            .map(|(output, root)| RootMetadata {
                source: match &refusal {
                    Some(error) => Err(error.clone()),
                    None => source(program, named, rule, output.source),
                },
                phase: root.phase,
                reads: root.reads.clone(),
            })
            .collect::<Vec<_>>();
        metadata.push(RuleMetadata {
            emit_root: match &refusal {
                Some(error) => Err(error.clone()),
                None => program
                    .rules()
                    .get(rule.0)
                    .ok_or(OperatingError::NotRun)?
                    .emit
                    .clone(),
            },
            observations: outputs.into_boxed_slice(),
        });
    }
    Ok(metadata)
}
