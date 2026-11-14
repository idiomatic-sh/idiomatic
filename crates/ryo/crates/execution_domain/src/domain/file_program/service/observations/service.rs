//! The `observations` service: free functions named by verb over the domain's types, with no IO and no ports.

use crate::domain::coverage::types::RuleIndex;
use crate::domain::file_program::constants::EXPECT_BINDING_INDEX;
use crate::domain::observation_schedule::types::{ObservationRequest, ResolvedObservationProducer};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::Plan;

/// Bind every output to the existing graph's actual node identity, with explicit unsupported admission.
pub(crate) fn graph_outputs(
    plans: &[Plan],
    nodes: &[Vec<Option<crate::domain::file_program::types::ProgramNodeId>>],
    request: ObservationRequest<'_>,
) -> Option<crate::domain::file_program::types::GraphObservations> {
    use crate::domain::file_program::types::{
        GraphObservationOutput, GraphObservationRoot, GraphObservations,
    };
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
    let frontiers =
        crate::domain::observation_schedule::service::frontiers::service::observed_frontiers(plans);
    let consumers =
        crate::domain::observation_schedule::service::frontiers::service::output_consumers(
            plans, bindings,
        );
    for (rule, outputs_of) in consumers.iter().enumerate() {
        for (output, consumed) in outputs_of.iter().enumerate() {
            if let Some(root) = outputs
                .get_mut(rule)
                .and_then(|roots| roots.get_mut(output))
            {
                root.consumers.clone_from(consumed);
            }
        }
    }
    // Each consumer retires the outputs it reads in the order its bindings name them, each once.
    let mut retire: Vec<Vec<crate::domain::file_program::types::ObservationRetirement>> =
        vec![Vec::new(); bindings.len()];
    for (consumer, inputs) in bindings.iter().enumerate() {
        for producer in inputs.iter().flat_map(|binding| &binding.producers) {
            if let ResolvedObservationProducer::Output { rule, output } = producer {
                let retirement = crate::domain::file_program::types::ObservationRetirement {
                    producer: *rule,
                    output: *output,
                };
                if let Some(retired) = retire.get_mut(consumer)
                    && !retired
                        .iter()
                        .any(|each| each.producer == *rule && each.output == *output)
                {
                    retired.push(retirement);
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
    let observation_frontiers =
        crate::domain::observation_schedule::service::frontiers::service::frontier_purposes(plans);
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
