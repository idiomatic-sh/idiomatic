//! Pure named-root dependency resolution; the existing application graph owns execution and resources.
use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::constants::{EXPECT_BINDING_INDEX, EXPECT_OUTPUT_INDEX};
use crate::domain::observation_schedule::types::{PreparedObservationInputs, ResolvedObservationBinding, ResolvedObservationProducer, RuleSchedule};
use crate::domain::rule_intent::errors::RuleIntentError;
use crate::domain::rule_intent::types::FrontierPurpose;
use ryo_values::domain::observations::types::{BoundObservationInput, ObservationProducer};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;
use std::collections::{BTreeMap, BTreeSet};

/// Bind every output to the existing graph's actual node identity, with explicit unsupported admission.
pub(crate) fn graph_outputs(
    plans: &[Plan],
    nodes: &[Vec<Option<crate::domain::file_program::types::ProgramNodeId>>],
    request: crate::domain::observation_schedule::types::ObservationRequest<'_>,
) -> Option<crate::domain::file_program::types::GraphObservations> {
    use crate::domain::file_program::types::{GraphObservationOutput, GraphObservationRoot, GraphObservations};
use crate::domain::observation_schedule::types::{ObservationRequest};
    let ObservationRequest::Bound {
        bindings, layout, ..
    } = request
    else {
        return None;
    };
    let slots = plans
        .iter()
        .enumerate()
        .flat_map(|(rule, plan)| {
            plan.observation_inputs
                .iter()
                .enumerate()
                .map(
                    move |(input, _)| crate::domain::file_program::types::ObservationSlot {
                        rule: RuleIndex(rule),
                        input: ryo_values::domain::plan_parts::types::ObservationBindingId(
                            u32::try_from(input).expect(EXPECT_BINDING_INDEX),
                        ),
                    },
                )
        })
        .collect();
    let mut outputs: Vec<Vec<GraphObservationOutput>> = plans
        .iter()
        .enumerate()
        .map(|(rule, plan)| {
            plan.observations
                .iter()
                .map(|output| {
                    let root = if output.scope == StepScope::CrossFile {
                        GraphObservationRoot::CrossFile
                    } else {
                        match output.source {
                            ryo_values::domain::plan_parts::types::Input::Step(step) => nodes
                                .get(rule)
                                .and_then(|nodes| nodes.get(step.0 as usize))
                                .copied()
                                .flatten()
                                .map_or(
                                    GraphObservationRoot::Unavailable,
                                    GraphObservationRoot::PerFile,
                                ),
                            _ => GraphObservationRoot::Unavailable,
                        }
                    };
                    GraphObservationOutput {
                        root,
                        consumers: Vec::new(),
                    }
                })
                .collect()
        })
        .collect();
    let mut frontiers = Vec::new();
    for (rule, plan) in plans.iter().enumerate() {
        let positions: std::collections::HashMap<_, _> = plan
            .frontier
            .iter()
            .copied()
            .enumerate()
            .map(|(at, input)| (input, at))
            .collect();
        for (output, root) in plan.observations.iter().enumerate() {
            if root.scope != StepScope::PerFile {
                continue;
            }
            if let Some(&frontier) = positions.get(&root.source) {
                frontiers.push(crate::domain::observation_schedule::types::ObservedFrontierRoot {
                    producer: RuleIndex(rule),
                    output: ryo_values::domain::records::types::ObservationIndex(
                        u32::try_from(output).expect(EXPECT_OUTPUT_INDEX),
                    ),
                    frontier: ryo_values::domain::plan_parts::types::InputNumber(frontier),
                });
            }
        }
    }
    let mut retire = vec![Vec::new(); bindings.len()];
    for (consumer, inputs) in bindings.iter().enumerate() {
        let consumer = RuleIndex(consumer);
        for binding in inputs {
            for producer in &binding.producers {
                if let ResolvedObservationProducer::Output { rule, output } = producer
                    && let Some(root) = outputs
                        .get_mut(rule.0)
                        .and_then(|outputs| outputs.get_mut(output.0 as usize))
                    && !root.consumers.contains(&consumer)
                {
                    root.consumers.push(consumer);
                    retire[consumer.0].push(
                        crate::domain::file_program::types::ObservationRetirement {
                            producer: *rule,
                            output: *output,
                        },
                    );
                }
            }
        }
    }
    let bindings = bindings
        .iter()
        .map(|inputs| {
            inputs
                .iter()
                .map(|input| {
                    let kinds = input
                        .schema
                        .columns
                        .iter()
                        .map(|column| column.kind)
                        .collect::<Vec<_>>();
                    let schema = crate::domain::operation_schema::types::InputShape {
                        kinds: &kinds,
                        tree: None,
                    };
                    let union = ryo_values::domain::plans::types::Operation::Union {
                        left: ryo_values::domain::plan_parts::types::Input::Step(
                            ryo_values::domain::plan_parts::types::StepId(0),
                        ),
                        right: ryo_values::domain::plan_parts::types::Input::Step(
                            ryo_values::domain::plan_parts::types::StepId(1),
                        ),
                    };
                    crate::domain::file_program::types::GraphObservationBinding {
                        input: input.clone(),
                        union: crate::domain::operator::contract::service::prepare_for(
                            ryo_values::domain::query_operators::types::OperationKind::Union,
                            &union,
                            &[schema, schema],
                            &[],
                            input.schema.scope,
                        )
                        .map(std::sync::Arc::new),
                    }
                })
                .collect()
        })
        .collect();
    let observation_frontiers = plans
        .iter()
        .map(|plan| {
            let mut needed = std::collections::HashSet::new();
            let mut pending = plan
                .observations
                .iter()
                .map(|output| output.source)
                .collect::<Vec<_>>();
            while let Some(input) = pending.pop() {
                if !needed.insert(input) {
                    continue;
                }
                if let ryo_values::domain::plan_parts::types::Input::Step(step) = input {
                    if let Some(step) = plan.steps.get(step.0 as usize) {
                        pending.extend(ryo_values::domain::plans::service::named(
                            &ryo_values::domain::plans::service::inputs_of(&step.operation),
                        ));
                    }
                }
            }
            plan.frontier
                .iter()
                .map(|input| {
                    if needed.contains(input) {
                        FrontierPurpose::Observation
                    } else {
                        FrontierPurpose::Diagnostic
                    }
                })
                .collect()
        })
        .collect();
    Some(GraphObservations {
        observation_frontiers,
        layout,
        slots,
        frontiers,
        retire,
        bindings,
        outputs,
    })
}
